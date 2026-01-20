use notify_debouncer_full::DebouncedEvent;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::db_listener::db::DbCmd;

pub struct EventListener {
    db_tx: std::sync::mpsc::Sender<DbCmd>,
    receiver: Receiver<Vec<DebouncedEvent>>,
    sender: Sender<Vec<DebouncedEvent>>
}

impl EventListener {
    pub fn new(db_tx : std::sync::mpsc::Sender<DbCmd>) -> Self {
        let (tx_parser, rx_parser) = mpsc::channel(1024);
        Self {
            db_tx,
            receiver: rx_parser,
            sender: tx_parser
        }
    }

    pub fn sender(&self) -> Sender<Vec<DebouncedEvent>> {
        self.sender.clone()
    }

    pub async fn run(mut self) {
        while let Some(batch) = self.receiver.recv().await {
            let _ = self.db_tx.send(DbCmd::ProcessEvents(batch));
        }
    }

}


