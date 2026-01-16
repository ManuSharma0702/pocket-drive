use tokio::sync::mpsc::Receiver;

pub struct EventParser;

impl EventParser {
    fn parse(&self){
        unimplemented!()
    }
}

struct EventListener {
    parser: EventParser,
    receiver: Receiver<>
}
