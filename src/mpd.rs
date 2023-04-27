use std::borrow::Cow;

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
