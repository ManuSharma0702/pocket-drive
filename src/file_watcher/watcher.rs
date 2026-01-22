use std::{path::Path,time::Duration};

use notify::RecommendedWatcher;
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};
use tokio::sync::mpsc::{self, Receiver};

#[derive(Debug)]
pub struct NotifyHandler {
    pub notify_watcher: Option<Debouncer<RecommendedWatcher, RecommendedCache>>,
    pub receiver: Option<Receiver<DebounceEventResult>>
}

impl NotifyHandler {
    pub fn new() -> NotifyHandler {
        let (tx, rx) = mpsc::channel(1024);
        let debouncer = new_debouncer(Duration::from_secs(3), 
            None,
            move |result| {
                let tx = tx.clone();
                let _ = tx.blocking_send(result);
            },
        );

        NotifyHandler {
            notify_watcher: Some(debouncer.unwrap()),
            receiver: Some(rx)
        }
    }

    pub fn watch(&mut self, path: &str) -> notify::Result<()>{
        let watch_path = Path::new(path);
        if let Some(watcher) = self.notify_watcher.as_mut() {
            watcher.watch(watch_path, notify::RecursiveMode::Recursive).unwrap();
        }
        Ok(())
    }
}

impl Default for NotifyHandler {
    fn default() -> Self {
        Self::new()
    }
}
