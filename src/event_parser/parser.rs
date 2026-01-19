use notify_debouncer_full::DebouncedEvent;
use tokio::sync::mpsc::{self, Receiver, Sender};

pub struct EventParser;

impl EventParser {
    fn new() -> Self {
        Self
    }

    pub fn parse(&self, input: Vec<DebouncedEvent>){
        //Call the hasher for generating what action to do on the server
        println!("{:?}", input);
    }
}

pub struct EventListener {
    parser: EventParser,
    receiver: Receiver<Vec<DebouncedEvent>>,
    sender: Sender<Vec<DebouncedEvent>>
}

impl EventListener {
    pub fn new() -> Self {
        let (tx_parser, rx_parser) = mpsc::channel(1024);
        Self {
            parser: EventParser::new(),
            receiver: rx_parser,
            sender: tx_parser
        }
    }

    pub fn sender(&self) -> Sender<Vec<DebouncedEvent>> {
        self.sender.clone()
    }

    pub async fn run(mut self) {
        while let Some(batch) = self.receiver.recv().await {
            self.parser.parse(batch);
        }
    }

}

impl Default for EventListener {
    fn default() -> Self {
        Self::new()
    }
}
