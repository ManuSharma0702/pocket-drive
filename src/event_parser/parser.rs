use notify_debouncer_full::DebouncedEvent;
use tokio::sync::mpsc::{self, Receiver, Sender};

pub struct EventParser;

impl EventParser {
    fn new() -> Self {
        Self
    }

    fn parse(&self){
        unimplemented!()
        //send to listener which will send to API handler
    }
}

struct EventListener {
    parser: EventParser,
    receiver: Receiver<Vec<DebouncedEvent>>,
    sender: Sender<Vec<DebouncedEvent>>
}

impl EventListener {
    fn new() -> Self {
        let (tx_parser, rx_parser) = mpsc::channel(1024);
        let parser = EventParser::new();
        EventListener { parser, receiver: rx_parser, sender: tx_parser }
    }
}
