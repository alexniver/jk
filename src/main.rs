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
use tracing_appender::rolling::{RollingFileAppender, Rotation};

fn main() -> io::Result<()> {
    let file_appender = RollingFileAppender::new(Rotation::DAILY, "log", "my_app.log");

    // 设置 tracing 订阅者，将日志输出到文件
    tracing_subscriber::fmt().with_writer(file_appender).init();

    let share_path_arr = Arc::new(RwLock::new(vec![]));
    let (tx, rx) = mpsc::channel(16);
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    web::run(rx, share_path_arr.clone(), shutdown_rx);

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
