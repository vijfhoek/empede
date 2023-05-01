use std::borrow::Cow;

use anyhow::anyhow;
use async_std::{
    io::{prelude::BufReadExt, BufReader, WriteExt},
    net::TcpStream,
};
use mpdrs::lsinfo::LsInfoResponse;

pub(crate) fn host() -> String {
    let host = std::env::var("MPD_HOST").unwrap_or("localhost".to_string());
    let port = std::env::var("MPD_PORT").unwrap_or("6600".to_string());
    format!("{host}:{port}")
}

pub(crate) fn connect() -> Result<mpdrs::Client, mpdrs::error::Error> {
    mpdrs::Client::connect(host())
}

pub(crate) fn ls(path: &str) -> anyhow::Result<Vec<Entry>> {
    let info = connect()?.lsinfo(path)?;

    fn filename(path: &str) -> Cow<str> {
        std::path::Path::new(path)
            .file_name()
            .map(|x| x.to_string_lossy())
            .unwrap_or(Cow::Borrowed("n/a"))
    }

    Ok(info
        .iter()
        .map(|e| match e {
            LsInfoResponse::Song(song) => Entry::Song {
                name: song.title.as_ref().unwrap_or(&song.file).clone(),
                artist: song.artist.clone().unwrap_or(String::new()),
                path: song.file.clone(),
            },

            LsInfoResponse::Directory { path, .. } => Entry::Directory {
                name: filename(path).to_string(),
                path: path.to_string(),
            },

            LsInfoResponse::Playlist { path, .. } => Entry::Playlist {
                name: filename(path).to_string(),
                path: path.to_string(),
            },
        })
        .collect())
}

pub(crate) struct QueueItem {
    pub(crate) file: String,
    pub(crate) title: String,
    pub(crate) artist: Option<String>,
    pub(crate) playing: bool,
}

pub(crate) fn playlist() -> anyhow::Result<Vec<QueueItem>> {
    let mut client = connect()?;

    let current = client.status()?.song;

    let queue = client
        .queue()?
        .into_iter()
        .map(|song| QueueItem {
            file: song.file.clone(),
            title: song.title.as_ref().unwrap_or(&song.file).clone(),
            artist: song.artist.clone(),
            playing: current == song.place,
        })
        .collect();

    Ok(queue)
}

pub(crate) enum Entry {
    Song {
        name: String,
        artist: String,
        path: String,
    },
    Directory {
        name: String,
        path: String,
    },
    Playlist {
        name: String,
        path: String,
    },
}

pub(crate) async fn idle(systems: &[&str]) -> anyhow::Result<Vec<String>> {
    let mut stream = TcpStream::connect(host()).await?;
    let mut reader = BufReader::new(stream.clone());

    // skip OK MPD line
    // TODO check if it is indeed OK
    let mut buffer = String::new();
    reader.read_line(&mut buffer).await?;

    let systems = systems.join(" ");
    let command = format!("idle {systems}\n");
    stream.write_all(command.as_bytes()).await?;

    let mut updated = vec![];
    loop {
        buffer.clear();
        reader.read_line(&mut buffer).await?;
        if buffer == "OK\n" {
            break Ok(updated);
        }

        let (_, changed) = buffer
            .trim_end()
            .split_once(": ")
            .ok_or(anyhow!("unexpected response from MPD"))?;
        updated.push(changed.to_string());
    }
}
