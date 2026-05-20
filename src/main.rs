use clap::Parser;
use difiko::ui::theme;
use difiko::{app, cli, event, new_window};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    if cli.new_window {
        return new_window::relaunch_in_new_window();
    }
    // Load the user's theme overrides before the first render so colors
    // are baked in. The loader never fails: bad keys / unparseable color
    // values / broken JSON all fall back to defaults and surface as
    // per-issue toasts on first frame.
    let theme_load = theme::load();
    theme::init(theme_load.theme);
    let mut app = app::App::new(&cli);
    for issue in &theme_load.issues {
        app.toast(format!("theme.json: {issue}"), app::ToastKind::Error);
    }
    event::run(app).await
}
