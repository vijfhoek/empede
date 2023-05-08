use anyhow::anyhow;
use async_std::{
    io::{prelude::BufReadExt, BufReader, WriteExt},
    net::TcpStream,
};
use mpdrs::lsinfo::LsInfoResponse;

pub fn host() -> String {
    let host = std::env::var("MPD_HOST").unwrap_or("localhost".to_string());
    let port = std::env::var("MPD_PORT").unwrap_or("6600".to_string());
    format!("{host}:{port}")
}

pub fn connect() -> Result<mpdrs::Client, mpdrs::error::Error> {
    let mut client = mpdrs::Client::connect(host())?;

    let password = std::env::var("MPD_PASSWORD").unwrap_or(String::new());
    if !password.is_empty() {
        client.login(&password)?;
    }

    Ok(client)
}

pub fn ls(path: &str) -> anyhow::Result<Vec<Entry>> {
    let info = connect()?.lsinfo(path)?;

    fn filename(path: &str) -> String {
        std::path::Path::new(path)
            .file_name()
            .map(|x| x.to_string_lossy().to_string())
            .unwrap_or("n/a".to_string())
    }

    Ok(info
        .iter()
        .map(|e| match e {
            LsInfoResponse::Song(song) => Entry::Song {
                name: song.title.as_ref().unwrap_or(&filename(&song.file)).clone(),
                artist: song.artist.clone().unwrap_or(String::new()),
                path: song.file.clone(),
            },

            LsInfoResponse::Directory { path, .. } => Entry::Directory {
                name: filename(path),
                path: path.to_string(),
            },

            LsInfoResponse::Playlist { path, .. } => Entry::Playlist {
                name: filename(path),
                path: path.to_string(),
            },
        })
        .collect())
}

pub struct QueueItem {
    pub file: String,
    pub title: String,
    pub artist: Option<String>,
    pub playing: bool,
}

pub fn playlist() -> anyhow::Result<Vec<QueueItem>> {
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

pub enum Entry {
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

pub struct Mpd {
    stream: TcpStream,
    reader: BufReader<TcpStream>,
}

impl Mpd {
    pub fn escape_str(s: &str) -> String {
        s.replace('\"', "\\\"").replace('\'', "\\'")
    }

    pub async fn connect() -> anyhow::Result<Self> {
        let mut stream = TcpStream::connect(host()).await?;
        let mut reader = BufReader::new(stream.clone());

        // skip OK MPD line
        // TODO check if it is indeed OK
        let mut buffer = String::new();
        reader.read_line(&mut buffer).await?;

        let password = std::env::var("MPD_PASSWORD").unwrap_or(String::new());
        if !password.is_empty() {
            let password = Self::escape_str(&password);
            let command = format!("password \"{password}\"\n");
            stream.write_all(command.as_bytes()).await?;

            buffer.clear();
            reader.read_line(&mut buffer).await?;
        }

        Ok(Self { stream, reader })
    }

    pub async fn command(&mut self, command: &str) -> anyhow::Result<()> {
        self.stream
            .write_all(format!("{command}\n").as_bytes())
            .await?;

        let mut buffer = String::new();
        loop {
            buffer.clear();
            self.reader.read_line(&mut buffer).await?;

            let split: Vec<_> = buffer.trim_end().split_ascii_whitespace().collect();

            if split[0] == "OK" {
                break Ok(());
            } else if split[0] == "ACK" {
                break Err(anyhow!(buffer));
            }
        }
    }

    pub async fn clear(&mut self) -> anyhow::Result<()> {
        self.command("clear").await
    }

    pub async fn add(&mut self, path: &str) -> anyhow::Result<()> {
        let path = Self::escape_str(path);
        self.command(&format!("add \"{path}\"")).await
    }

    pub async fn add_pos(&mut self, path: &str, pos: &str) -> anyhow::Result<()> {
        let path = Self::escape_str(path);
        let pos = Self::escape_str(pos);
        self.command(&format!("add \"{path}\" \"{pos}\"")).await
    }

    pub async fn play(&mut self) -> anyhow::Result<()> {
        self.command("play").await
    }

    pub async fn idle(&mut self, systems: &[&str]) -> anyhow::Result<Vec<String>> {
        let mut buffer = String::new();

        let systems = systems.join(" ");
        let command = format!("idle {systems}\n");
        self.stream.write_all(command.as_bytes()).await?;

        let mut updated = vec![];
        loop {
            buffer.clear();
            self.reader.read_line(&mut buffer).await?;
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
}
