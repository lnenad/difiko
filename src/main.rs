use clap::Parser;
use difiko::{app, cli, event};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    let app = app::App::new(&cli);
    event::run(app).await
}
