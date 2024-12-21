use futures_util::FutureExt;
use rust_socketio::{
    asynchronous::{Client, ClientBuilder},
    Payload,
};

#[tokio::main]
async fn main() {
    let job_callback = |payload: Payload, socket: Client| {
        async move {
            println!("JOB: {:#?}", &payload);

            socket
                .emit("job-done", payload)
                .await
                .expect("Server unreachable");
        }
        .boxed()
    };

    let role_callback = |_payload: Payload, socket: Client| {
        async move {
            socket
                .emit("role", "sub")
                .await
                .expect("Server unreachable");
        }
        .boxed()
    };

    ClientBuilder::new("http://localhost:3000")
        .on("what-role", role_callback)
        .on("job", job_callback)
        .connect()
        .await
        .expect("Connection failed");

    std::future::pending::<()>().await
}
