use std::{collections::HashMap, sync::mpsc::Sender, time::SystemTime};
use reqwest::multipart::{Form, Part};
use reqwest::{Client, StatusCode};
use serde::Serialize;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

use crate::db_listener::db::FileEntry;

#[derive(Hash, Eq, PartialEq, Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Operations{
    Insert,
    Update,
    Delete
}

pub enum FileUploaderCmd {
    Sync(HashMap<Operations, Vec<FileEntryDTO>>, Sender<(StatusCode, String)>),
    Get()
}

pub struct FileUploader{
    tx_uploader: tokio::sync::mpsc::Sender<FileUploaderCmd>,
    rx_uploader: tokio::sync::mpsc::Receiver<FileUploaderCmd>,
}

#[derive(Serialize, Debug)]
pub struct FileEntryDTO {
    file_path: String,
    file_hash: Option<String>,
    file_size: i64,
    modified_time: i64,
}

impl From<&FileEntry> for FileEntryDTO {
    fn from(value: &FileEntry) -> Self {
        FileEntryDTO { 
            file_path: value.path.to_string_lossy().to_string(), 
            file_hash: value.hash.clone(), 
            file_size: value.size as i64, 
            modified_time: value.modified
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64,
        }
    }
}

impl FileUploader {

    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(1024);
        Self {
            tx_uploader: tx,
            rx_uploader: rx
        }
    }

    pub fn get_sender(&self) -> tokio::sync::mpsc::Sender<FileUploaderCmd> {
        self.tx_uploader.clone()
    }

    pub async fn run(mut self) {
        while let Some(val) = self.rx_uploader.recv().await {
            self.execute(val).await;
        }
    }

    async fn execute(&self, cmd: FileUploaderCmd) {
        match cmd {
            FileUploaderCmd::Sync(operations, sender) => {
                let client = Client::new();
                let payload = serde_json::to_string(&operations).unwrap();
                let mut form = Form::new().text("payload", payload);

                for entries in operations.values(){
                    for dto in entries{
                        let path = &dto.file_path;
                        if !std::path::Path::new(path).exists() {
                            continue;
                        }

                        let file = File::open(path).await.unwrap();

                        let stream = FramedRead::new(file, BytesCodec::new());
                        let body = reqwest::Body::wrap_stream(stream);

                        let filename = std::path::Path::new(path)
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .to_string();

                        let part = Part::stream(body)
                            .file_name(filename)
                            .mime_str("application/octet-stream")
                            .unwrap();

                        // same name for multiple files
                        form = form.part("files", part);
                    }
                }

                dbg!(&operations);
                let response = client
                    .post("http://localhost:8000/sync")
                    .multipart(form)
                    .send()
                    .await
                    .unwrap();

                let status = response.status();
                let body = response.text().await.unwrap();

                println!("Body:\n{}", body);
                println!("Status: {}", status);
                sender.send((status, body)).unwrap();                
            }
            FileUploaderCmd::Get() => unimplemented!()
        }
    }
}

impl Default for FileUploader {
    fn default() -> Self {
        Self::new()
    }
}
