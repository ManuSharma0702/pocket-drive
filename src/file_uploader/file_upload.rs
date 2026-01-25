use std::{collections::HashMap, sync::mpsc::Sender, time::SystemTime};

use reqwest::{Client, StatusCode};
use serde::Serialize;

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

                dbg!(&operations);
                let response = client
                    .post("http://localhost:8000/sync")
                    .header("Content-Type", "application/json")
                    .json(&operations)
                    .send()
                    .await
                    .unwrap();
                let status = response.status();
                let body = response.text().await.unwrap();
                println!("Status: {}", status);
                println!("Body:\n{}", body);
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
