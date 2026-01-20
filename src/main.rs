use std::env ;

use pocket_drive::{db_listener::db::Db, event_listener::listener::EventListener, file_hasher::hasher::Hasher, file_watcher::watcher::NotifyHandler};

#[tokio::main]
async fn main() {
    let mut watcher = NotifyHandler::new(); 
    let db = Db::new();

    let listener = EventListener::new(db.get_sender());
    let sender = listener.sender();

    let hasher = Hasher::new(db.get_sender());

    let args: Vec<String> = env::args().collect();
    let path = &args[1];

    watcher.watch(path).unwrap();

    tokio::spawn(listener.run());

    std::thread::spawn(move || {
        db.run();
    });

    hasher.initialise(path);

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

