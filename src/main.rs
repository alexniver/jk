use std::{
    cmp::Ordering,
    collections::HashMap,
    env::current_dir,
    io::{self, stdout},
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};

const PORT: u16 = 33231;

const COLOR_FG: Color = Color::Green;
const COLOR_BG: Color = Color::Black;
const COLOR_HIGHLIGHT: Color = Color::DarkGray;

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
                            app.dir_info.set_current_to_parent()?;
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
                            app.dir_info.set_current_to_child()?;
                        }
                    }
                    KeyCode::Char('j') => {
                        app.dir_info.set_current_list_state_next();
                    }
                    KeyCode::Char('k') => {
                        app.dir_info.set_current_list_state_prev();
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
    parent: Option<PathInfo>,
    current: Option<PathInfo>,
    child: Option<PathInfo>,
    selected_map: HashMap<PathBuf, usize>,
}

impl DirInfo {
    fn new(current_dir: PathBuf) -> io::Result<Self> {
        let mut selected_map = HashMap::new();
        let (parent, current, child) = gen_parent_current_child(current_dir, &mut selected_map)?;
        let s = Self {
            parent,
            current,
            child,
            selected_map,
        };
        Ok(s)
    }

    fn set_current_dir(&mut self, path_buf: PathBuf) -> io::Result<()> {
        if let Some(current) = &self.current {
            if current.path == path_buf {
                return Ok(());
            }
            let (parent, current, child) =
                gen_parent_current_child(path_buf, &mut self.selected_map)?;
            self.parent = parent;
            self.current = current;
            self.child = child;
        }
        Ok(())
    }

    fn set_current_to_parent(&mut self) -> io::Result<()> {
        if let Some(parent) = &self.parent {
            self.set_current_dir(parent.path.clone())?;
        }
        Ok(())
    }

    fn set_current_to_child(&mut self) -> io::Result<()> {
        if let Some(child) = &self.child {
            self.set_current_dir(child.path.clone())?;
        }
        Ok(())
    }

    fn set_current_list_state(&mut self, idx: usize) {
        if let Some(ref mut current) = self.current {
            current.list_state.select(Some(idx));
            self.selected_map.insert(current.path.clone(), idx);
            if let Some(ref mut child) = self.child {
                child.set_path(current.files[idx].clone());
            }
        }
    }

    fn set_current_list_state_prev(&mut self) {
        if let Some(ref mut current) = self.current {
            let len = current.files.len();
            if let Some(idx) = current.list_state.selected() {
                if idx > 0 {
                    self.set_current_list_state(idx - 1);
                } else {
                    self.set_current_list_state(len - 1);
                }
            }
        }
    }

    fn set_current_list_state_next(&mut self) {
        if let Some(ref mut current) = self.current {
            let len = current.files.len();
            if let Some(idx) = current.list_state.selected() {
                if idx < len - 1 {
                    self.set_current_list_state(idx + 1)
                } else {
                    self.set_current_list_state(0)
                }
            }
        }
    }
}

fn gen_parent_current_child(
    current_dir: PathBuf,
    selected_map: &mut HashMap<PathBuf, usize>,
) -> io::Result<(Option<PathInfo>, Option<PathInfo>, Option<PathInfo>)> {
    let parent = if let Some(parent) = current_dir.parent() {
        let mut path_info = PathInfo::new(PathBuf::from(parent), PathType::Parent)?;
        let mut parent_selected_idx = 0;
        for (idx, p) in path_info.files.iter().enumerate() {
            if p == &current_dir {
                selected_map.insert(PathBuf::from(parent), idx);
                parent_selected_idx = idx;
            }
        }
        selected_map.insert(path_info.path.clone(), parent_selected_idx);
        path_info.auto_select(selected_map);
        Some(path_info)
    } else {
        None
    };

    let mut current = PathInfo::new(current_dir, PathType::Current)?;
    current.auto_select(selected_map);

    let selected_idx = if let Some(selected_idx) = current.list_state.selected() {
        selected_idx
    } else {
        0
    };
    let file = &current.files[selected_idx];

    let child = if file.is_dir() {
        let mut path_info = PathInfo::new(file.clone(), PathType::Child)?;
        path_info.auto_select(selected_map);
        Some(path_info)
    } else {
        None
    };

    Ok((parent, Some(current), child))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathType {
    Parent,
    Current,
    Child,
}

struct PathInfo {
    path: PathBuf,
    path_type: PathType,
    list_state: ListState,
    files: Vec<PathBuf>,
}

impl PathInfo {
    fn new(path: PathBuf, path_type: PathType) -> io::Result<Self> {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        let mut files = vec![];
        get_files(&path, &mut files)?;

        Ok(Self {
            path,
            path_type,
            list_state,
            files,
        })
    }

    fn auto_select(&mut self, selected_map: &HashMap<PathBuf, usize>) {
        if let Some(&idx) = selected_map.get(&self.path) {
            self.list_state.select(Some(idx));
        }
    }

    fn set_path(&mut self, path_buf: PathBuf) -> io::Result<()> {
        get_files(&path_buf, &mut self.files)?;
        self.path = path_buf;
        Ok(())
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

fn path_last_n(path: &Path, n: usize) -> String {
    let last_three_components: Vec<String> = path
        .components()
        .rev()
        // .skip_while(|c| c != &std::path::Component::Normal(std::ffi::OsStr::new("")))
        .take(n)
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    let last_three_components: Vec<String> = last_three_components.into_iter().rev().collect();

    let mut result = last_three_components.join("/");

    if path.is_dir() {
        result.push('/');
    }
    result
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

    ui_dir_files(frame, dir_layout[0], &mut app.dir_info.parent);
    ui_dir_files(frame, dir_layout[1], &mut app.dir_info.current);
    ui_dir_files(frame, dir_layout[2], &mut app.dir_info.child);
}

fn ui_dir_files(frame: &mut Frame, parent_dir_layout: Rect, path_info: &mut Option<PathInfo>) {
    if let Some(path_info) = path_info {
        let title = match path_info.path_type {
            PathType::Parent => "Parent",
            PathType::Current => "Current",
            PathType::Child => "Child",
        };
        let items: Vec<ListItem> = path_info
            .files
            .iter()
            .map(|p| {
                let lines = vec![path_last_n(p, 2).into()];
                ListItem::new(lines).style(Style::default().fg(COLOR_FG).bg(COLOR_BG))
            })
            .collect();
        let dir_list = List::new(items)
            .block(
                Block::bordered()
                    .title(title)
                    .style(Style::default().gray()),
            )
            .highlight_style(
                Style::default()
                    .bg(COLOR_HIGHLIGHT)
                    .add_modifier(Modifier::BOLD),
            )
            .direction(ListDirection::TopToBottom);
        frame.render_stateful_widget(dir_list, parent_dir_layout, &mut path_info.list_state);
    }
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
