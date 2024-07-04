use std::{
    fs,
    net::SocketAddr,
    sync::{mpsc::channel, Arc},
};

use clap::Parser;
use tokio::time::Duration;

use screenpipe_server::{start_continuous_recording, DatabaseManager, Server};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// FPS for continuous recording
    #[arg(short, long, default_value_t = 5.0)]
    fps: f64,

    /// Audio chunk duration in seconds
    #[arg(short, long, default_value_t = 30)]
    audio_chunk_duration: u64,

    /// Port to run the server on
    #[arg(short, long, default_value_t = 3030)]
    port: u16,

    /// Disable audio recording
    #[arg(long, default_value_t = false)]
    disable_audio: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    use env_logger::Builder;
    use log::LevelFilter;
    let cli = Cli::parse();

    Builder::new()
        .filter(None, LevelFilter::Info)
        .filter_module("tokenizers", LevelFilter::Error)
        .filter_module("rusty_tesseract", LevelFilter::Error)
        .init();

    let local_data_dir = ensure_local_data_dir()?;
    let db = Arc::new(
        DatabaseManager::new(&format!("{}/db.sqlite", local_data_dir))
            .await
            .unwrap(),
    );
    let db_server = db.clone();
    // Channel for controlling the recorder
    let (_control_tx, control_rx) = channel();

    // Start continuous recording in a separate task
    let local_data_dir_clone = local_data_dir.clone();
    let _recording_task = tokio::spawn(async move {
        let audio_chunk_duration = Duration::from_secs(cli.audio_chunk_duration);

        start_continuous_recording(
            db,
            &local_data_dir_clone,
            cli.fps,
            audio_chunk_duration,
            control_rx,
            !cli.disable_audio,
        )
        .await
    });

    tokio::spawn(async move {
        let server = Server::new(db_server, SocketAddr::from(([0, 0, 0, 0], cli.port)));
        server.start().await.unwrap();
    });

    // Wait for the server to start
    println!("Server started on http://localhost:{}", cli.port);

    // Keep the main thread running
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        // You can add logic here to send control messages if needed
        // For example:
        // control_tx.send(RecorderControl::Pause).await?;
        // control_tx.send(RecorderControl::Resume).await?;
    }

    // This part will never be reached in the current implementation
    // control_tx.send(RecorderControl::Stop).await?;
    // recording_task.await??;
}

fn ensure_local_data_dir() -> anyhow::Result<String> {
    let local_data_dir = "./data".to_string(); // TODO: Use $HOME/.screenpipe/data
    fs::create_dir_all(&local_data_dir)?;
    Ok(local_data_dir)
}