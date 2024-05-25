use std::{
    env::current_dir,
    io::{self, stdout},
    path::PathBuf,
    rc::Rc,
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

            let mut app = App::new(dir);

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

struct App {
    dir_info: DirInfo,
}

impl App {
    fn new(current_dir: PathBuf) -> Self {
        Self {
            dir_info: DirInfo::new(current_dir),
        }
    }
}

struct DirInfo {
    current_dir: PathBuf,
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

    ui_content(frame, main_layout[1]);

    ui_status_line(frame, main_layout[2]);
}

fn ui_content(frame: &mut Frame, content_layout: Rect) {
    let inner_layout = Layout::new(
        Direction::Horizontal,
        [Constraint::Percentage(70), Constraint::Percentage(30)],
    )
    .split(content_layout);

    ui_dir(frame, inner_layout[0]);

    ui_shares(frame, inner_layout[1]);
}

fn ui_dir(frame: &mut Frame, dir_layout: Rect) {
    let dir_block = Block::bordered().title("Dir");
    let dir_child = dir_block.inner(dir_layout);

    frame.render_widget(dir_block, dir_layout);

    let dir_child_layout = Layout::new(
        Direction::Horizontal,
        [
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ],
    )
    .split(dir_child);

    frame.render_widget(Block::bordered().title("Parent Dir"), dir_child_layout[0]);
    frame.render_widget(Block::bordered().title("Current Dir"), dir_child_layout[1]);
    frame.render_widget(Block::bordered().title("Child Dir"), dir_child_layout[2]);
}

fn ui_shares(frame: &mut Frame, share_layout: Rect) {
    frame.render_widget(Block::bordered().title("Shares"), share_layout);
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
        Span::raw(" clear all share."),
    ]);
    let text: Text = Text::from(vec![line]);

    frame.render_widget(Paragraph::new(text), status_layout);
}
