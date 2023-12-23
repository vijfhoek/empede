use std::{collections::HashMap, sync::OnceLock};

use anyhow::anyhow;
use async_std::{
    io::{prelude::BufReadExt, BufReader, ReadExt, WriteExt},
    net::TcpStream,
    sync::{Mutex, MutexGuard},
    task::block_on,
};

pub fn host() -> String {
    let host = std::env::var("MPD_HOST").unwrap_or("localhost".to_string());
    let port = std::env::var("MPD_PORT").unwrap_or("6600".to_string());
    format!("{host}:{port}")
}

pub struct QueueItem {
    pub id: u32,
    pub file: String,
    pub title: String,
    pub artist: Option<String>,
    pub playing: bool,
}

#[derive(Debug)]
pub enum Entry {
    Song {
        track: Option<i32>,
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

#[derive(Debug)]
pub struct Mpd {
    stream: Option<TcpStream>,
    reader: Option<BufReader<TcpStream>>,
}

pub static INSTANCE: OnceLock<Mutex<Mpd>> = OnceLock::new();

pub async fn get_instance() -> MutexGuard<'static, Mpd> {
    let instance = INSTANCE.get_or_init(|| {
        let mut mpd = Mpd::new();
        block_on(mpd.connect()).unwrap();
        Mutex::from(mpd)
    });
    instance.lock().await
}

pub async fn command(command: &str) -> anyhow::Result<CommandResult> {
    get_instance().await.command(command).await
}

pub struct CommandResult {
    properties: Vec<(String, String)>,
    binary: Option<Vec<u8>>,
}

impl CommandResult {
    pub fn new(properties: Vec<(String, String)>) -> Self {
        Self {
            properties,
            binary: None,
        }
    }

    pub fn new_binary(properties: Vec<(String, String)>, binary: Vec<u8>) -> Self {
        Self {
            properties,
            binary: Some(binary),
        }
    }

    pub fn into_hashmap(self) -> HashMap<String, String> {
        self.properties.into_iter().collect()
    }

    pub fn into_hashmaps(self, split_at: &[&str]) -> Vec<HashMap<String, String>> {
        let mut output = Vec::new();
        let mut current = None;

        for (key, value) in self.properties {
            if split_at.contains(&key.as_str()) {
                if let Some(current) = current {
                    output.push(current);
                }
                current = Some(HashMap::new());
            }

            if let Some(current) = current.as_mut() {
                current.insert(key, value);
            }
        }

        if let Some(current) = current {
            output.push(current);
        }

        output
    }
}

impl Mpd {
    pub fn escape_str(s: &str) -> String {
        s.replace('\"', "\\\"").replace('\'', "\\'")
    }

    pub fn new() -> Self {
        Self {
            stream: None,
            reader: None,
        }
    }

    pub async fn connect(&mut self) -> anyhow::Result<()> {
        self.stream = Some(TcpStream::connect(host()).await?);
        self.reader = Some(BufReader::new(self.stream.as_mut().unwrap().clone()));

        // skip OK MPD line
        // TODO check if it is indeed OK
        let mut buffer = String::new();
        self.reader.as_mut().unwrap().read_line(&mut buffer).await?;

        let password = std::env::var("MPD_PASSWORD").unwrap_or_default();
        if !password.is_empty() {
            let password = Self::escape_str(&password);
            self.stream
                .as_mut()
                .unwrap()
                .write_all(format!(r#"password "{password}"\n"#).as_bytes())
                .await?;
            self.reader.as_mut().unwrap().read_line(&mut buffer).await?;
        }

        self.stream
            .as_mut()
            .unwrap()
            .write_all("binarylimit 1048576\n".as_bytes())
            .await?;
        self.reader.as_mut().unwrap().read_line(&mut buffer).await?;

        Ok(())
    }

    async fn read_binary_data(&mut self, size: usize) -> anyhow::Result<Vec<u8>> {
        let mut binary = vec![0u8; size];
        self.reader
            .as_mut()
            .unwrap()
            .read_exact(&mut binary)
            .await?;

        let mut buffer = String::new();

        // Skip the newline after the binary data
        self.reader.as_mut().unwrap().read_line(&mut buffer).await?;

        // Skip the "OK" after the binary data
        // TODO Check if actually OK
        self.reader.as_mut().unwrap().read_line(&mut buffer).await?;

        Ok(binary)
    }

    pub async fn command(&mut self, command: &str) -> anyhow::Result<CommandResult> {
        let mut properties = Vec::new();

        'retry: loop {
            self.stream
                .as_mut()
                .unwrap()
                .write_all(format!("{command}\n").as_bytes())
                .await?;

            let mut buffer = String::new();
            break 'retry (loop {
                buffer.clear();
                self.reader.as_mut().unwrap().read_line(&mut buffer).await?;

                if let Some((key, value)) = buffer.split_once(": ") {
                    let value = value.trim_end();
                    properties.push((key.to_string(), value.to_string()));

                    if key == "binary" {
                        let binary = self.read_binary_data(value.parse()?).await?;
                        break Ok(CommandResult::new_binary(properties, binary));
                    }
                } else if buffer.starts_with("OK") {
                    break Ok(CommandResult::new(properties));
                } else if buffer.starts_with("ACK") {
                    break Err(anyhow!(buffer));
                } else {
                    println!("Unexpected MPD response '{buffer}'");
                    self.connect().await.unwrap();
                    continue 'retry;
                }
            });
        }
    }

    pub async fn command_binary(&mut self, command: &str) -> anyhow::Result<CommandResult> {
        let mut buffer = Vec::new();

        loop {
            let command = format!("{} {}", command, buffer.len());
            let result = self.command(&command).await?;

            if let Some(mut binary) = result.binary {
                if !binary.is_empty() {
                    buffer.append(&mut binary);
                } else {
                    return Ok(CommandResult::new_binary(result.properties, buffer));
                }
            } else {
                return Ok(CommandResult::new(result.properties));
            }
        }
    }

    pub async fn clear(&mut self) -> anyhow::Result<()> {
        self.command("clear").await?;
        Ok(())
    }

    pub async fn add(&mut self, path: &str) -> anyhow::Result<()> {
        let path = Self::escape_str(path);
        self.command(&format!("add \"{path}\"")).await?;
        Ok(())
    }

    pub async fn add_pos(&mut self, path: &str, pos: &str) -> anyhow::Result<()> {
        let path = Self::escape_str(path);
        let pos = Self::escape_str(pos);
        self.command(&format!(r#"add "{path}" "{pos}""#)).await?;
        Ok(())
    }

    pub async fn play(&mut self) -> anyhow::Result<()> {
        self.command("play").await?;
        Ok(())
    }

    pub async fn idle(&mut self, systems: &[&str]) -> anyhow::Result<Vec<String>> {
        let systems = systems.join(" ");
        let result = self.command(&format!("idle {systems}")).await?;
        let changed = result
            .properties
            .iter()
            .filter(|(key, _)| key == "changed")
            .map(|(_, value)| value.clone())
            .collect();
        Ok(changed)
    }

    pub async fn albumart(&mut self, path: &str) -> anyhow::Result<Vec<u8>> {
        let path = Self::escape_str(path);
        let result = self
            .command_binary(&format!(r#"albumart "{path}""#))
            .await?;

        match result.binary {
            Some(binary) => Ok(binary),
            None => Err(anyhow!("no album art")),
        }
    }

    pub async fn readpicture(&mut self, path: &str) -> anyhow::Result<Vec<u8>> {
        let path = Self::escape_str(path);
        let result = self
            .command_binary(&format!(r#"readpicture "{path}""#))
            .await?;

        match result.binary {
            Some(binary) => Ok(binary),
            None => Err(anyhow!("no album art")),
        }
    }

    #[allow(clippy::manual_map)]
    pub async fn ls(&mut self, path: &str) -> anyhow::Result<Vec<Entry>> {
        fn get_filename(path: &str) -> String {
            std::path::Path::new(path)
                .file_name()
                .map(|x| x.to_string_lossy().to_string())
                .unwrap_or("n/a".to_string())
        }

        let path = Self::escape_str(path);
        let result = self
            .command(&format!(r#"lsinfo "{path}""#))
            .await?
            .into_hashmaps(&["file", "directory", "playlist"]);

        let files = result
            .iter()
            .flat_map(|prop| {
                if let Some(file) = prop.get("file") {
                    Some(Entry::Song {
                        track: prop.get("Track").and_then(|track| track.parse().ok()),
                        name: prop.get("Title").unwrap_or(&get_filename(file)).clone(),
                        artist: prop.get("Artist").unwrap_or(&String::new()).clone(),
                        path: file.to_string(),
                    })
                } else if let Some(file) = prop.get("directory") {
                    Some(Entry::Directory {
                        name: get_filename(file),
                        path: file.to_string(),
                    })
                } else if let Some(file) = prop.get("playlist") {
                    Some(Entry::Playlist {
                        name: get_filename(file),
                        path: file.to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(files)
    }

    pub async fn playlist(&mut self) -> anyhow::Result<Vec<QueueItem>> {
        let status = self.command("status").await?.into_hashmap();
        let current_songid = status.get("songid");

        let playlistinfo = self.command("playlistinfo").await?;
        let queue = playlistinfo.into_hashmaps(&["file"]);

        let queue = queue
            .iter()
            .map(|song| QueueItem {
                id: song["Id"].parse().unwrap(),
                file: song["file"].clone(),
                title: song.get("Title").unwrap_or(&song["file"]).clone(),
                artist: song.get("Artist").cloned(),
                playing: current_songid == song.get("Id"),
            })
            .collect();

        Ok(queue)
    }
}
