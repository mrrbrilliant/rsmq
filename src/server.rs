use axum::routing::get;
use socketioxide::{
    extract::{AckSender, Data, SocketRef},
    SocketIo,
};
use tracing::info;
use tracing_subscriber::FmtSubscriber;

fn on_connect(socket: SocketRef) {
    info!("connected: {:?}", socket.id);
    // For subscriber (worker)
    socket.emit("what-role", "").ok();

    // For publisher
    socket.on("new-job", |socket: SocketRef, Data::<String>(data)| {
        info!("Received new-job: {}", &data);
        socket.to("sub").emit("job", &data).ok();
    });

    socket.on(
        "role",
        |socket: SocketRef, Data::<String>(data), ack: AckSender| {
            println!("role: {}", &data);

            if data == "sub" {
                socket.join("sub").ok();
                ack.send("successs").ok();
            }
        },
    );

    socket.on("job-done", |Data::<String>(data)| {
        println!("job-done: {}", &data);
    });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing::subscriber::set_global_default(FmtSubscriber::default())?;

    let (layer, io) = SocketIo::new_layer();

    io.ns("/", on_connect);

    let app = axum::Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .layer(layer);

    info!("Starting server");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
