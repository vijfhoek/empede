use std::borrow::Cow;

use mpdrs::lsinfo::LsInfoResponse;

pub(crate) const HOST: &str = "192.168.1.203:6600";

pub(crate) async fn ls(path: &str) -> anyhow::Result<Vec<Entry>> {
    let mut mpd = mpdrs::Client::connect(HOST)?;
    let info = mpd.lsinfo(path)?;

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

            LsInfoResponse::Directory { path, .. } => {
                Entry::Directory {
                    name: filename(path).to_string(),
                    path: path.to_string(),
                }
            }

            LsInfoResponse::Playlist { path, .. } => Entry::Playlist {
                name: filename(path).to_string(),
                path: path.to_string(),
            },
        })
        .collect())
}

pub(crate) struct QueueItem {
    pub(crate) title: String,
    pub(crate) playing: bool,
}

pub(crate) async fn playlist() -> anyhow::Result<Vec<QueueItem>> {
    let mut client = mpdrs::Client::connect(HOST)?;

    let current = client.status()?.song;

    let queue = client
        .queue()?
        .into_iter()
        .map(|song| QueueItem {
            title: song.title.as_ref().unwrap_or(&song.file).clone(),
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
