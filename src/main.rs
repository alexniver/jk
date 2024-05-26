use std::{
    cmp::Ordering,
    env::current_dir,
    io::{self, stdout},
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, ModifierKeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};

const PORT: u16 = 33231;

fn main() -> io::Result<()> {
    match current_dir() {
        Ok(dir) => {
            enable_raw_mode()?;
            stdout().execute(EnterAlternateScreen)?;
            let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

            let app = App::new(dir)?;

            run_app(&mut terminal, app)?;

            disable_raw_mode()?;
            stdout().execute(LeaveAlternateScreen)?;
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        sleep(Duration::from_millis(50));
        terminal.draw(|f| ui(f, &mut app))?;

        let mut is_left_ctrl = false;

        if let Event::Key(key) = event::read()? {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                is_left_ctrl = true;
            }

            if key.kind == KeyEventKind::Press {
                // match key_code
                match key.code {
                    KeyCode::Char('Q') => return Ok(()),
                    KeyCode::Char('h') => {
                        // 按下ctrl, 切换dir, 否则在当前block切换
                        if is_left_ctrl {
                            match app.get_current_block() {
                                CurrentBlock::Dir => app.set_current_block(CurrentBlock::Shares),
                                CurrentBlock::Shares => app.set_current_block(CurrentBlock::Dir),
                            }
                        } else {
                        }
                    }
                    KeyCode::Char('l') => {
                        // 按下ctrl, 切换dir, 否则在当前block切换
                        if is_left_ctrl {
                            match app.get_current_block() {
                                CurrentBlock::Dir => app.set_current_block(CurrentBlock::Shares),
                                CurrentBlock::Shares => app.set_current_block(CurrentBlock::Dir),
                            }
                        } else {
                        }
                    }
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
    share_info: ShareInfo,
}

impl App {
    fn new(current_dir: PathBuf) -> io::Result<Self> {
        let s = Self {
            current_block: CurrentBlock::Dir,
            dir_info: DirInfo::new(current_dir)?,
            share_info: ShareInfo::new(),
        };
        Ok(s)
    }

    fn get_current_block(&self) -> CurrentBlock {
        self.current_block
    }

    fn set_current_block(&mut self, target_block: CurrentBlock) {
        self.current_block = target_block
    }
}

struct DirInfo {
    current_dir: PathBuf,
    parent_dir_files: Vec<PathBuf>,
    current_dir_files: Vec<PathBuf>,
    child_dir_files: Vec<PathBuf>,
    selected_file_idx: usize,
}

impl DirInfo {
    fn new(current_dir: PathBuf) -> io::Result<Self> {
        let mut parent_dir_files = vec![];
        let mut current_dir_files = vec![];
        let mut child_dir_files = vec![];
        let selected_file_idx = 0;
        if let Some(parent_dir) = current_dir.parent() {
            get_files(parent_dir, &mut parent_dir_files)?
        }
        get_files(&current_dir, &mut current_dir_files)?;
        if current_dir_files.len() > 0 && current_dir_files[selected_file_idx].is_dir() {
            get_files(&current_dir_files[selected_file_idx], &mut child_dir_files)?;
        }
        let s = Self {
            current_dir,
            parent_dir_files,
            current_dir_files,
            child_dir_files,
            selected_file_idx,
        };
        Ok(s)
    }
}

struct ShareInfo {}

impl ShareInfo {
    fn new() -> Self {
        Self {}
    }
}

fn get_files(dir: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    files.clear();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        files.push(path);
    }
    files.sort_by(|a, b| {
        if a.is_dir() && b.is_file() {
            Ordering::Less
        } else if a.is_file() && b.is_dir() {
            Ordering::Greater
        } else {
            if let (Some(a_name), Some(b_name)) = (a.file_name(), b.file_name()) {
                a_name.cmp(b_name)
            } else {
                Ordering::Equal
            }
        }
    });
    Ok(())
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
        dir_block = dir_block.style(Style::new().fg(Color::Yellow).bold());
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
    let items: Vec<ListItem> = app
        .dir_info
        .parent_dir_files
        .iter()
        .map(|p| {
            let mut lines = vec![p.to_str().unwrap().into()];

            ListItem::new(lines).style(Style::default().fg(Color::Black).bg(Color::White))
        })
        .collect();
    let dir_list = List::new(items)
        .block(
            Block::bordered()
                .title("Parent Dir")
                .style(Style::default().gray()),
        )
        .direction(ListDirection::TopToBottom);
    frame.render_widget(dir_list, parent_dir_layout);
}

fn ui_current_dir(frame: &mut Frame, current_dir_layout: Rect, app: &mut App) {
    let items: Vec<ListItem> = app
        .dir_info
        .current_dir_files
        .iter()
        .map(|p| {
            let mut lines = vec![p.to_str().unwrap().into()];

            ListItem::new(lines).style(Style::default().fg(Color::Black).bg(Color::White))
        })
        .collect();
    let dir_list = List::new(items)
        .block(
            Block::bordered()
                .title("Current Dir")
                .style(Style::default().gray()),
        )
        .direction(ListDirection::TopToBottom);
    frame.render_widget(dir_list, current_dir_layout);
}

fn ui_child_dir(frame: &mut Frame, child_dir_layout: Rect, app: &mut App) {
    let items: Vec<ListItem> = app
        .dir_info
        .child_dir_files
        .iter()
        .map(|p| {
            let mut lines = vec![p.to_str().unwrap().into()];

            ListItem::new(lines).style(Style::default().fg(Color::Black).bg(Color::White))
        })
        .collect();
    let dir_list = List::new(items)
        .block(
            Block::bordered()
                .title("Child Dir")
                .style(Style::default().gray()),
        )
        .direction(ListDirection::TopToBottom);

    frame.render_widget(dir_list, child_dir_layout);
}

fn ui_shares(frame: &mut Frame, share_layout: Rect, app: &mut App) {
    let mut block = Block::bordered().title("Shares");
    if app.get_current_block() == CurrentBlock::Shares {
        block = block.style(Style::new().fg(Color::Yellow).bold());
    }
    frame.render_widget(block, share_layout);
}

fn ui_title(frame: &mut Frame, title_layout: Rect) {
    let title = Span::styled(
        format!("Visit localhost:{PORT}"),
        Style::new()
            .fg(Color::LightBlue)
            .add_modifier(Modifier::BOLD),
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
        Span::styled("'ctrl + h'/'ctrl + l'", style_key),
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
