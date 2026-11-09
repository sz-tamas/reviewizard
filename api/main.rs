use clap::Parser;
use reviewizard::services::{application, config::Config, http, server};
use std::net::Ipv4Addr;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::parse();
    let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, config.port)).await?;
    let (state, db) = application::initialize(&config).await?;
    let url = format!("http://{}", listener.local_addr()?);
    eprintln!(
        "reviewizard {} (Rust backend) serving {}",
        env!("REVIEWIZARD_VERSION"),
        state.workspace.root.display()
    );
    eprintln!("db: {}", db.display());
    eprintln!("listening on {url}");
    let live = state.live.clone();
    server::serve(
        listener,
        http::router(state),
        live,
        application::shutdown_signal(),
    )
    .await?;
    Ok(())
}
