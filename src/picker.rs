use crate::paths::{self, Candidate};
use anyhow::{Context, Result};
use crossterm::{
    event::{
        self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEventKind,
        KeyModifiers,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{List, ListItem, ListState, Paragraph},
};
use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::Duration,
};

struct Screen(Terminal<CrosstermBackend<File>>);
impl Screen {
    fn open() -> Result<Self> {
        let tty = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")
            .context("The picker requires a terminal; pass a path directly instead")?;
        enable_raw_mode()?;
        let result = (|| {
            let mut terminal = Terminal::new(CrosstermBackend::new(tty))?;
            execute!(
                terminal.backend_mut(),
                EnterAlternateScreen,
                EnableBracketedPaste
            )?;
            Ok(Self(terminal))
        })();
        if result.is_err() {
            let _ = disable_raw_mode();
        }
        result
    }
}
impl Drop for Screen {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.0.backend_mut(),
            DisableBracketedPaste,
            LeaveAlternateScreen
        );
        let _ = self.0.show_cursor();
    }
}

pub fn pick(walker: ignore::Walk, root: &Path) -> Result<Option<PathBuf>> {
    let mut screen = Screen::open()?;
    let (send, receive) = mpsc::sync_channel(1024);
    let root = root.to_owned();
    thread::spawn(move || {
        for entry in walker {
            let message = match entry {
                Ok(entry) => {
                    let directory = entry.file_type().is_some_and(|kind| kind.is_dir());
                    Ok(Candidate::new(entry.into_path(), &root, directory))
                }
                Err(error) => Err(error.to_string()),
            };
            if send.send(message).is_err() {
                break;
            }
        }
    });
    let mut candidates = Vec::new();
    let mut matches = Vec::new();
    let mut query = String::new();
    let mut selected = 0usize;
    let mut list_state = ListState::default();
    let mut scanning = true;
    let mut warnings = 0;
    let mut last_warning = String::new();
    let mut dirty = true;
    let mut last_query = String::new();
    let mut redraw = true;
    loop {
        for _ in 0..2048 {
            match receive.try_recv() {
                Ok(Ok(candidate)) => {
                    candidates.push(candidate);
                    dirty = true;
                }
                Ok(Err(error)) => {
                    warnings += 1;
                    last_warning = error;
                    redraw = true;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    redraw |= scanning;
                    scanning = false;
                    break;
                }
            }
        }
        if dirty {
            redraw = true;
            let previous = if query == last_query {
                matches.get(selected).copied()
            } else {
                None
            };
            matches = paths::rank(&candidates, &query);
            selected = previous
                .and_then(|index| matches.iter().position(|candidate| *candidate == index))
                .unwrap_or(0);
            last_query.clone_from(&query);
            dirty = false;
        }
        list_state.select(if matches.is_empty() {
            None
        } else {
            Some(selected)
        });
        if redraw {
            screen.0.draw(|frame| {
                let areas = Layout::vertical([
                    Constraint::Length(1),
                    Constraint::Min(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(frame.area());
                frame.render_widget(
                    Paragraph::new(format!(
                        "fpc  {} / {}{}   Enter copy · ↑↓ select · Esc cancel",
                        matches.len(),
                        candidates.len(),
                        if scanning { " (scanning)" } else { "" }
                    )),
                    areas[0],
                );
                frame.render_widget(
                    Paragraph::new(format!("> {query}")).style(Style::default().fg(Color::Cyan)),
                    areas[3],
                );
                let items: Vec<_> = matches
                    .iter()
                    .map(|&index| ListItem::new(candidates[index].label.as_str()))
                    .collect();
                frame.render_stateful_widget(
                    List::new(items).highlight_symbol("› ").highlight_style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    areas[1],
                    &mut list_state,
                );
                if warnings > 0 {
                    let safe: String = last_warning
                        .chars()
                        .flat_map(char::escape_default)
                        .collect();
                    frame.render_widget(
                        Paragraph::new(format!("{warnings} scan warning(s): {safe}"))
                            .style(Style::default().fg(Color::Yellow)),
                        areas[2],
                    );
                }
            })?;
            redraw = false;
        }
        if !event::poll(Duration::from_millis(40))? {
            continue;
        }
        redraw = true;
        match event::read()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => match key.code {
                KeyCode::Esc => return Ok(None),
                KeyCode::Char('c' | 'd') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(None);
                }
                KeyCode::Enter => {
                    if let Some(&index) = matches.get(selected) {
                        return Ok(Some(candidates[index].path.clone()));
                    }
                }
                KeyCode::Down | KeyCode::Tab => {
                    if !matches.is_empty() {
                        selected = (selected + 1) % matches.len();
                    }
                }
                KeyCode::Up | KeyCode::BackTab => {
                    if !matches.is_empty() {
                        selected = (selected + matches.len() - 1) % matches.len();
                    }
                }
                KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if !matches.is_empty() {
                        selected = (selected + 1) % matches.len();
                    }
                }
                KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if !matches.is_empty() {
                        selected = (selected + matches.len() - 1) % matches.len();
                    }
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    query.clear();
                    selected = 0;
                    dirty = true;
                }
                KeyCode::Backspace => {
                    query.pop();
                    selected = 0;
                    dirty = true;
                }
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    query.push(c);
                    selected = 0;
                    dirty = true;
                }
                _ => {}
            },
            Event::Paste(text) => {
                query.extend(text.chars().filter(|c| !c.is_control()));
                selected = 0;
                dirty = true;
            }
            _ => {}
        }
    }
}
