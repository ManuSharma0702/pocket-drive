use std::env;

use pocket_drive::{event_parser::parser::EventListener, file_watcher::watcher::NotifyHandler};

#[tokio::main]
async fn main() {
    let mut watcher = NotifyHandler::new(); 
    let listener = EventListener::new();
    let sender = listener.sender();

    let args: Vec<String> = env::args().collect();
    let path = &args[1];

    //Create local hash of directory files, take path of the file to hash 


    watcher.watch(path).unwrap();

    tokio::spawn(listener.run());

    if let Some(mut rx) = watcher.receiver.take() {
        tokio::spawn(async move {
            while let Some(res) = rx.recv().await {
                match res {
                    //Send these events to an event handler task, which will decide what 
                    //to do with changes and based on that either
                    //Create, Update, Delete, Rename the file on the server
                    Ok(events) => {
                        sender.send(events).await.unwrap();
                        // println!("Events: {:?}", events)
                    },
                    Err(errors) => {
                        println!("Errors: {:?}", errors)
                    }
                }
            }
        });
    }

    println!("Watching...");
    tokio::signal::ctrl_c().await.unwrap();
}
