use std::{
    env::current_dir,
    io::{self, stdout},
    path::{Path, PathBuf},
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};

fn main() -> io::Result<()> {
    match current_dir() {
        Ok(dir) => {
            enable_raw_mode()?;
            stdout().execute(EnterAlternateScreen)?;
            let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

            let app = App::new(dir);

            run_app(&mut terminal, app);

            disable_raw_mode()?;
            stdout().execute(LeaveAlternateScreen)?;
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('Q') => return Ok(()),
                    _ => {}
                }
            }
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum CurrentBlock {
    #[default]
    Dir,
    Shares,
}

struct App {
    current_block: CurrentBlock,
    dir_info: DirInfo,
}

impl App {
    fn new(current_dir: PathBuf) -> Self {
        Self {
            current_block: CurrentBlock::Dir,
            dir_info: DirInfo::new(current_dir),
        }
    }

    fn get_current_block(&self) -> CurrentBlock {
        self.current_block
    }

    fn get_parent_dir(&self) -> Option<&Path> {
        self.dir_info.current_dir.parent()
    }

    fn get_current_dir(&self) -> &Path {
        &self.dir_info.current_dir
    }
}

struct DirInfo {
    current_dir: PathBuf,
    // current_file: PathBuf,
}

impl DirInfo {
    fn new(current_dir: PathBuf) -> Self {
        Self { current_dir }
    }
}

fn handle_events() -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('Q') {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn ui(frame: &mut Frame, app: &mut App) {
    let main_layout = Layout::new(
        Direction::Vertical,
        [
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ],
    )
    .split(frame.size());

    ui_title(frame, main_layout[0]);

    ui_content(frame, main_layout[1], app);

    ui_status_line(frame, main_layout[2]);
}

fn ui_content(frame: &mut Frame, content_layout: Rect, app: &mut App) {
    let inner_layout = Layout::new(
        Direction::Horizontal,
        [Constraint::Percentage(70), Constraint::Percentage(30)],
    )
    .split(content_layout);

    ui_dir(frame, inner_layout[0], app);

    ui_shares(frame, inner_layout[1], app);
}

fn ui_dir(frame: &mut Frame, dir_block_layout: Rect, app: &mut App) {
    let mut dir_block = Block::bordered().title("Dir");

    if app.get_current_block() == CurrentBlock::Dir {
        dir_block = dir_block.style(Style::new().fg(Color::Yellow));
    }

    let dir_child = dir_block.inner(dir_block_layout);

    frame.render_widget(dir_block, dir_block_layout);

    let dir_layout = Layout::new(
        Direction::Horizontal,
        [
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ],
    )
    .split(dir_child);

    ui_parent_dir(frame, dir_layout[0], app);
    ui_current_dir(frame, dir_layout[1], app);
    ui_child_dir(frame, dir_layout[2], app);
}

fn ui_parent_dir(frame: &mut Frame, parent_dir_layout: Rect, app: &mut App) {
    frame.render_widget(
        Block::bordered().title("Parent Dir").style(Style::new()),
        parent_dir_layout,
    );
}

fn ui_current_dir(frame: &mut Frame, current_dir_layout: Rect, app: &mut App) {
    frame.render_widget(Block::bordered().title("Current Dir"), current_dir_layout);
}

fn ui_child_dir(frame: &mut Frame, child_dir_layout: Rect, app: &mut App) {
    frame.render_widget(Block::bordered().title("Child Dir"), child_dir_layout);
}

fn ui_shares(frame: &mut Frame, share_layout: Rect, app: &mut App) {
    let mut block = Block::bordered().title("Shares");
    if app.get_current_block() == CurrentBlock::Shares {
        block = block.style(Style::new().fg(Color::Yellow));
    }
    frame.render_widget(block, share_layout);
}

fn ui_title(frame: &mut Frame, title_layout: Rect) {
    let title = Span::styled(
        "kj, a command line file share manager",
        Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    );
    let title = Line::from(vec![title]);
    let text: Text = Text::from(vec![title]);

    frame.render_widget(Paragraph::new(text), title_layout);
}

fn ui_status_line(frame: &mut Frame, status_layout: Rect) {
    let style_key = Style::new()
        .fg(Color::Green)
        .bg(Color::Black)
        .add_modifier(Modifier::BOLD);

    let line = Line::from(vec![
        Span::raw("Press "),
        Span::styled("'Q'", style_key),
        Span::raw(" to exit, "),
        Span::styled("'ctrl + h'/'ctrl + j'", style_key),
        Span::raw(" switch panel, "),
        Span::styled("'h'/'j'/'k'/'l'", style_key),
        Span::raw(" to select file, "),
        Span::styled("'='/'-'", style_key),
        Span::raw(" add/remove share, "),
        Span::styled("'C'", style_key),
        Span::raw(" clear all shares."),
    ]);
    let text: Text = Text::from(vec![line]);

    frame.render_widget(Paragraph::new(text), status_layout);
}
