use std::{
    io::{self, stdout},
    rc::Rc,
};

use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut should_quit = false;
    while !should_quit {
        terminal.draw(ui)?;
        should_quit = handle_events()?;
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
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

fn ui(frame: &mut Frame) {
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
    frame.render_widget(
        Block::new()
            .borders(Borders::TOP)
            .title("kj, a command line file share manager"),
        title_layout,
    );
}

fn ui_status_line(frame: &mut Frame, status_layout: Rect) {
    let span1 = Span::raw("Hello ");
    let span2 = Span::styled(
        "World",
        Style::new()
            .fg(Color::Green)
            .bg(Color::White)
            .add_modifier(Modifier::BOLD),
    );
    let span3 = "!".red().on_light_yellow().italic();

    let line = Line::from(vec![span1, span2, span3]);
    let text: Text = Text::from(vec![line]);
    frame.render_widget(
        Block::new().borders(Borders::TOP).title(
            "Press 'Q' to exit, 'ctrl + h','ctrl + j' switch panel, 'h', 'j', 'k', 'l' to move cursor, '=', '-' add/remove share, 'C' clear all share",
        ),
        status_layout,
    );
}
