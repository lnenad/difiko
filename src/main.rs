use clap::Parser;
use difiko::{app, cli, event, new_window};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    if cli.new_window {
        return new_window::relaunch_in_new_window();
    }
    let app = app::App::new(&cli);
    event::run(app).await
}
