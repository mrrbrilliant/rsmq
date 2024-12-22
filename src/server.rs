pub mod signal;

use std::sync::Arc;

use axum::routing::get;

use signal::{create_signal, Signal};
use socketioxide::{
    extract::{AckSender, Data, SocketRef, State},
    SocketIo,
};

use tracing::info;
use tracing_subscriber::FmtSubscriber;

fn on_connect(socket: SocketRef) {
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

    let count = create_signal(0);

    let (layer, io) = SocketIo::builder().with_state(count.clone()).build_layer();

    io.ns(
        "/",
        move |socket: SocketRef, State(state): State<Arc<Signal<i32>>>| {
            println!("CONN: {}", socket.id);
            state.set(state.get() + 1);
        },
    );

    count.effect(|value| {
        println!("{}", value);
    });

    let app = axum::Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .layer(layer);

    info!("Starting server");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    // runtime.dispose();
    Ok(())
}
