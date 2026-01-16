use pocket_drive::file_watcher::watcher::NotifyHandler;

#[tokio::main]
async fn main() {
    let mut watcher = NotifyHandler::new(); 

    watcher.watch(".").unwrap();

    if let Some(mut rx) = watcher.receiver.take() {
        tokio::spawn(async move {
            while let Some(res) = rx.recv().await {
                match res {
                    //Send these events to an event handler task, which will decide what 
                    //to do with changes and based on that either
                    //Create, Update, Delete, Rename the file on the server
                    Ok(events) => println!("Events: {:?}", events),
                    Err(errors) => println!("Errors: {:?}", errors),
                }
            }
        });
    }

    tokio::signal::ctrl_c().await.unwrap();

}
