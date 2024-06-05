use std::{
    collections::HashMap,
    io,
    net::IpAddr,
    path::{Path, PathBuf},
    sync::Arc,
    thread::sleep,
    time::Duration,
};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use local_ip_address::local_ip;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::{mpsc::Sender, RwLock};

use crate::{consts::*, utils::sort_files};

pub fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    let local_ip_addr = local_ip().unwrap();

    loop {
        sleep(Duration::from_millis(50));
        terminal.draw(|f| ui(f, &mut app, local_ip_addr))?;

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
                        } else if app.current_block == CurrentBlock::Dir {
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
                        } else if app.current_block == CurrentBlock::Dir {
                            app.dir_info.set_current_to_child()?;
                        }
                    }
                    KeyCode::Char('j') => match app.current_block {
                        CurrentBlock::Dir => {
                            app.dir_info.set_current_list_state_next()?;
                        }
                        CurrentBlock::Shares => {
                            app.share_info.next();
                        }
                    },
                    KeyCode::Char('k') => match app.current_block {
                        CurrentBlock::Dir => {
                            app.dir_info.set_current_list_state_prev()?;
                        }
                        CurrentBlock::Shares => {
                            app.share_info.prev();
                        }
                    },
                    KeyCode::Char('=') => match app.current_block {
                        CurrentBlock::Dir => {
                            if let Some(file) = app.get_current_select_file() {
                                if file.is_file() {
                                    app.share_info.add(file);
                                    let _ = app.tx.blocking_send(());
                                }
                            }
                        }
                        CurrentBlock::Shares => {}
                    },
                    KeyCode::Char('-') => match app.current_block {
                        CurrentBlock::Dir => {}
                        CurrentBlock::Shares => {
                            app.share_info.remove();
                            let _ = app.tx.blocking_send(());
                        }
                    },
                    KeyCode::Char('C') => {
                        app.share_info.clear();
                        let _ = app.tx.blocking_send(());
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

pub struct App {
    current_block: CurrentBlock,
    dir_info: DirInfo,
    share_info: ShareInfo,
    // 发送share info change
    tx: Sender<()>,
}

impl App {
    pub fn new(
        tx: Sender<()>,
        current_dir: PathBuf,
        path_arr: Arc<RwLock<Vec<PathBuf>>>,
    ) -> io::Result<Self> {
        let s = Self {
            current_block: CurrentBlock::Dir,
            dir_info: DirInfo::new(current_dir)?,
            share_info: ShareInfo::new(path_arr),
            tx,
        };
        Ok(s)
    }

    fn get_current_block(&self) -> CurrentBlock {
        self.current_block
    }

    fn set_current_block(&mut self, target_block: CurrentBlock) {
        self.current_block = target_block
    }

    fn get_current_select_file(&self) -> Option<PathBuf> {
        if let Some(current) = &self.dir_info.current {
            if let Some(idx) = current.list_state.selected() {
                return Some(current.files[idx].clone());
            }
        }
        None
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
            if child.path.is_dir() && std::fs::read_dir(&child.path).is_ok() {
                self.set_current_dir(child.path.clone())?;
            }
        }
        Ok(())
    }

    fn set_current_list_state(&mut self, idx: usize) -> io::Result<()> {
        if let Some(ref mut current) = self.current {
            current.list_state.select(Some(idx));
            self.selected_map.insert(current.path.clone(), idx);
            if let Some(ref mut child) = self.child {
                child.set_path(current.files[idx].clone())?;
            }
        }
        Ok(())
    }

    fn set_current_list_state_prev(&mut self) -> io::Result<()> {
        if let Some(ref mut current) = self.current {
            let len = current.files.len();
            if let Some(idx) = current.list_state.selected() {
                if idx > 0 {
                    self.set_current_list_state(idx - 1)?;
                } else {
                    self.set_current_list_state(len - 1)?;
                }
            }
        }
        Ok(())
    }

    fn set_current_list_state_next(&mut self) -> io::Result<()> {
        if let Some(ref mut current) = self.current {
            let len = current.files.len();
            if let Some(idx) = current.list_state.selected() {
                if idx < len - 1 {
                    self.set_current_list_state(idx + 1)?;
                } else {
                    self.set_current_list_state(0)?;
                }
            }
        }
        Ok(())
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
    let child = if !current.files.is_empty() {
        let selected_idx = if let Some(selected_idx) = current.list_state.selected() {
            selected_idx
        } else {
            0
        };
        let file = &current.files[selected_idx];

        if file.is_dir() {
            let mut path_info = PathInfo::new(file.clone(), PathType::Child)?;
            path_info.auto_select(selected_map);
            Some(path_info)
        } else {
            None
        }
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

struct ShareInfo {
    path_arr: Arc<RwLock<Vec<PathBuf>>>,
    list_state: ListState,
}

impl ShareInfo {
    fn new(path_arr: Arc<RwLock<Vec<PathBuf>>>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            path_arr,
            list_state,
        }
    }

    fn add(&mut self, path_buf: PathBuf) {
        let mut path_arr = self.path_arr.blocking_write();
        if path_arr.contains(&path_buf) {
            return;
        }
        path_arr.push(path_buf);
        sort_files(&mut path_arr);
        if self.list_state.selected().is_none() {
            self.list_state.select(Some(0));
        }
    }

    fn remove(&mut self) {
        let mut path_arr = self.path_arr.blocking_write();
        if let Some(idx) = self.list_state.selected() {
            path_arr.remove(idx);
            let len = path_arr.len();
            if idx >= len {
                if len > 0 {
                    self.list_state.select(Some(len - 1));
                } else {
                    self.list_state.select(None);
                }
            }
        }
    }

    fn prev(&mut self) {
        let path_arr = self.path_arr.blocking_read();
        if let Some(idx) = self.list_state.selected() {
            let len = path_arr.len();
            if idx > 0 {
                self.list_state.select(Some(idx - 1));
            } else {
                self.list_state.select(Some(len - 1));
            }
        }
    }

    fn next(&mut self) {
        let path_arr = self.path_arr.blocking_read();
        if let Some(idx) = self.list_state.selected() {
            let len = path_arr.len();
            if len <= 1 {
                return;
            }
            if idx < len - 1 {
                self.list_state.select(Some(idx + 1));
            } else {
                self.list_state.select(Some(0));
            }
        }
    }

    fn clear(&mut self) {
        let mut path_arr = self.path_arr.blocking_write();
        path_arr.clear();
        self.list_state.select(None);
    }
}

fn get_files(dir: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    files.clear();
    if dir.is_dir() {
        if let Ok(children) = std::fs::read_dir(dir) {
            for entry in children.flatten() {
                let path = entry.path();
                files.push(path);
            }
        }

        sort_files(files);
    }

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

fn ui(frame: &mut Frame, app: &mut App, local_ip_addr: IpAddr) {
    let main_layout = Layout::new(
        Direction::Vertical,
        [
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ],
    )
    .split(frame.size());

    ui_title(frame, main_layout[0], local_ip_addr);

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

fn ui_dir_files(frame: &mut Frame, dir_layout: Rect, path_info: &mut Option<PathInfo>) {
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
        frame.render_stateful_widget(dir_list, dir_layout, &mut path_info.list_state);
    }
}

fn ui_shares(frame: &mut Frame, share_layout: Rect, app: &mut App) {
    let mut block = Block::bordered().title("Shares");
    if app.get_current_block() == CurrentBlock::Shares {
        block = block.style(Style::new().fg(Color::Yellow).bold());
    }
    let items: Vec<ListItem> = app
        .share_info
        .path_arr
        .blocking_read()
        .iter()
        .map(|p| {
            let lines = vec![path_last_n(p, 2).into()];
            ListItem::new(lines).style(Style::default().fg(COLOR_FG).bg(COLOR_BG))
        })
        .collect();
    let dir_list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(COLOR_HIGHLIGHT)
                .add_modifier(Modifier::BOLD),
        )
        .direction(ListDirection::TopToBottom);
    frame.render_stateful_widget(dir_list, share_layout, &mut app.share_info.list_state);
}

fn ui_title(frame: &mut Frame, title_layout: Rect, local_ip_addr: IpAddr) {
    let title = Span::styled(
        format!("Visit {}:{PORT}", local_ip_addr),
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
