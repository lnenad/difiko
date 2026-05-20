use clap::Parser;
use difiko::ui::theme;
use difiko::{app, cli, event, new_window};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    if cli.new_window {
        return new_window::relaunch_in_new_window();
    }
    // Load the user's theme overrides before the first render so colors are
    // baked in. A malformed theme.json falls back to the built-in palette
    // and surfaces as a toast once the App is up.
    let theme_err = match theme::load() {
        Ok(t) => {
            theme::init(t);
            None
        }
        Err(e) => Some(e),
    };
    let mut app = app::App::new(&cli);
    if let Some(e) = theme_err {
        app.toast(
            format!("theme.json failed to load, using defaults: {e}"),
            app::ToastKind::Error,
        );
    }
    event::run(app).await
}
