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
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{List, ListDirection, ListItem, ListState, Paragraph},
};
use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::Duration,
};

struct Screen(Terminal<CrosstermBackend<File>>);

#[derive(Default)]
struct Query {
    text: String,
    cursor: usize,
}

impl Query {
    fn insert(&mut self, text: &str) {
        self.text.insert_str(self.cursor, text);
        self.cursor += text.len();
    }

    fn left(&mut self) {
        if let Some(c) = self.text[..self.cursor].chars().next_back() {
            self.cursor -= c.len_utf8();
        }
    }

    fn right(&mut self) {
        if let Some(c) = self.text[self.cursor..].chars().next() {
            self.cursor += c.len_utf8();
        }
    }

    fn backspace(&mut self) {
        let end = self.cursor;
        self.left();
        self.text.drain(self.cursor..end);
    }

    fn delete(&mut self) {
        if self.cursor < self.text.len() {
            self.text.remove(self.cursor);
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }
        let available = usize::from(area.width.saturating_sub(3));
        let mut start = 0;
        let mut cursor_width = Line::from(&self.text[..self.cursor]).width();
        for (index, c) in self.text[..self.cursor].char_indices() {
            if cursor_width <= available {
                break;
            }
            start = index + c.len_utf8();
            cursor_width = Line::from(&self.text[start..self.cursor]).width();
        }
        frame.render_widget(
            Paragraph::new(format!("> {}", &self.text[start..]))
                .style(Style::default().fg(Color::Cyan)),
            area,
        );
        let column = (cursor_width + 2).min(usize::from(area.width - 1));
        frame.set_cursor_position((area.x + column as u16, area.y));
    }
}

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
    let mut query = Query::default();
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
            let previous = if query.text == last_query {
                matches.get(selected).copied()
            } else {
                None
            };
            matches = paths::rank(&candidates, &query.text);
            selected = previous
                .and_then(|index| matches.iter().position(|candidate| *candidate == index))
                .unwrap_or(0);
            last_query.clone_from(&query.text);
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
                    Constraint::Min(1),
                    Constraint::Length(u16::from(warnings > 0)),
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
                    areas[2],
                );
                query.render(frame, areas[3]);
                let items: Vec<_> = matches
                    .iter()
                    .map(|&index| ListItem::new(candidates[index].label.as_str()))
                    .collect();
                frame.render_stateful_widget(
                    List::new(items)
                        .direction(ListDirection::BottomToTop)
                        .highlight_symbol("› ")
                        .highlight_style(
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                    areas[0],
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
                        areas[1],
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
                KeyCode::Up | KeyCode::Tab => {
                    if !matches.is_empty() {
                        selected = (selected + 1) % matches.len();
                    }
                }
                KeyCode::Down | KeyCode::BackTab => {
                    if !matches.is_empty() {
                        selected = (selected + matches.len() - 1) % matches.len();
                    }
                }
                KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if !matches.is_empty() {
                        selected = (selected + 1) % matches.len();
                    }
                }
                KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if !matches.is_empty() {
                        selected = (selected + matches.len() - 1) % matches.len();
                    }
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    query = Query::default();
                    selected = 0;
                    dirty = true;
                }
                KeyCode::Backspace => {
                    query.backspace();
                    selected = 0;
                    dirty = true;
                }
                KeyCode::Delete => {
                    query.delete();
                    selected = 0;
                    dirty = true;
                }
                KeyCode::Left => query.left(),
                KeyCode::Right => query.right(),
                KeyCode::Home => query.cursor = 0,
                KeyCode::End => query.cursor = query.text.len(),
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    query.insert(c.encode_utf8(&mut [0; 4]));
                    selected = 0;
                    dirty = true;
                }
                _ => {}
            },
            Event::Paste(text) => {
                let text: String = text.chars().filter(|c| !c.is_control()).collect();
                query.insert(&text);
                selected = 0;
                dirty = true;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::{Backend, TestBackend};

    #[test]
    fn query_edits_at_utf8_boundaries() {
        let mut query = Query::default();
        query.insert("中文");
        query.left();
        query.insert("🙂");
        assert_eq!(query.text, "中🙂文");
        query.backspace();
        assert_eq!(query.text, "中文");
        query.delete();
        assert_eq!(query.text, "中");
        query.right();
        query.delete();
        assert_eq!(query.cursor, query.text.len());
        query.left();
        query.left();
        query.backspace();
        assert_eq!(query.cursor, 0);
        assert_eq!(query.text, "中");
        query.insert("ab");
        query.right();
        query.backspace();
        assert_eq!(query.text, "ab");
    }

    #[test]
    fn query_scrolls_to_keep_the_cursor_visible() {
        let mut query = Query::default();
        query.insert("ab中文z");
        let mut terminal = Terminal::new(TestBackend::new(8, 1)).unwrap();
        terminal
            .draw(|frame| query.render(frame, frame.area()))
            .unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(0, 0)].symbol(), ">");
        assert_eq!(buffer[(2, 0)].symbol(), "中");
        assert_eq!(buffer[(4, 0)].symbol(), "文");
        assert_eq!(buffer[(6, 0)].symbol(), "z");
        assert_eq!(
            terminal.backend_mut().get_cursor_position().unwrap(),
            (7, 0).into()
        );

        query.cursor = 0;
        terminal
            .draw(|frame| query.render(frame, frame.area()))
            .unwrap();
        assert_eq!(terminal.backend().buffer()[(2, 0)].symbol(), "a");
        assert_eq!(
            terminal.backend_mut().get_cursor_position().unwrap(),
            (2, 0).into()
        );
        for width in 1..=3 {
            let mut terminal = Terminal::new(TestBackend::new(width, 1)).unwrap();
            terminal
                .draw(|frame| query.render(frame, frame.area()))
                .unwrap();
            assert!(terminal.backend_mut().get_cursor_position().unwrap().x < width);
        }
    }
}
