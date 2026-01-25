use std::env ;

use pocket_drive::{db_listener::db::Db, event_listener::listener::EventListener, file_hasher::hasher::Hasher, file_uploader::file_upload::FileUploader, file_watcher::watcher::NotifyHandler};

#[tokio::main]
async fn main() {

    let args: Vec<String> = env::args().collect();
    let path = &args[1];

    let mut watcher = NotifyHandler::new(); 
    let hasher = Hasher::new();
    let uploader = FileUploader::new();

    let db = Db::new(hasher.get_sender(), uploader.get_sender());
    let listener = EventListener::new(db.get_sender());
    let sender = listener.sender();

    watcher.watch(path).unwrap();
    tokio::spawn(uploader.run());
    tokio::spawn(listener.run());

    //Start a task of file uploader which will be async since after network call there will be
    //waiting, which will post/delete/put the changes to home server
    //pass the tx of this to DB, which orchestrates parsing, hashing, and syncing
    //When receive an event db will first update server then update local hash

    std::thread::spawn(move || {
        db.run(&args[1].clone());
    });
    std::thread::spawn(move || {
        hasher.run();
    });

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

