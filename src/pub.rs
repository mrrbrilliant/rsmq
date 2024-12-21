use rust_socketio::asynchronous::ClientBuilder;
use std::{thread, time};

#[tokio::main]
async fn main() {
    let client = ClientBuilder::new("http://localhost:3000")
        .connect()
        .await
        .expect("Connection failed");

    thread::sleep(time::Duration::from_millis(100));
    client.emit("new-job", "hi").await.unwrap();
    client.disconnect().await.unwrap();
}
