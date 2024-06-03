mod console_ui;
mod consts;
mod utils;
mod web;

use std::{
    env::current_dir,
    io::{self, stdout},
    sync::Arc,
};

use console_ui::{run_app, App};
use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;
use tokio::sync::{mpsc, oneshot, RwLock};

fn main() -> io::Result<()> {
    let share_path_arr = Arc::new(RwLock::new(vec![]));
    let (tx, rx) = mpsc::channel(16);
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    web::run(
        Arc::new(RwLock::new(rx)),
        share_path_arr.clone(),
        shutdown_rx,
    );

    let result = match current_dir() {
        Ok(dir) => {
            enable_raw_mode()?;
            stdout().execute(EnterAlternateScreen)?;
            let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

            let app = App::new(tx, dir, share_path_arr)?;

            run_app(&mut terminal, app)?;

            stdout().execute(LeaveAlternateScreen)?;
            disable_raw_mode()?;
            Ok(())
        }
        Err(e) => Err(e),
    };

    // tell axum shutdown
    let _ = shutdown_tx.send(());

    result
}
