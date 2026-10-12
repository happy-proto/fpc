use crate::paths::{self, Candidate};
use anyhow::{Context, Result};
use crossterm::{
    event::{
        self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEvent, KeyEventKind,
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
    widgets::{List, ListDirection, ListItem, ListState, Paragraph},
};
use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::Duration,
};
use tui_input::{Input, InputRequest, backend::crossterm::to_input_request};

struct Screen(Terminal<CrosstermBackend<File>>);

fn input_request(key: KeyEvent) -> Option<InputRequest> {
    // Crossterm distinguishes ALT from META; accept both for word editing.
    match (key.code, key.modifiers) {
        (KeyCode::Left | KeyCode::Char('b'), KeyModifiers::ALT | KeyModifiers::META) => {
            Some(InputRequest::GoToPrevWord)
        }
        (KeyCode::Right | KeyCode::Char('f'), KeyModifiers::ALT | KeyModifiers::META) => {
            Some(InputRequest::GoToNextWord)
        }
        (KeyCode::Char('d'), KeyModifiers::ALT | KeyModifiers::META) => {
            Some(InputRequest::DeleteNextWord)
        }
        _ => to_input_request(&Event::Key(key)),
    }
}

fn render_query(query: &Input, frame: &mut Frame, area: Rect) {
    if area.is_empty() {
        return;
    }
    let style = Style::default().fg(Color::Cyan);
    frame.render_widget(Paragraph::new("> ").style(style), area);
    if area.width <= 2 {
        frame.set_cursor_position((area.right() - 1, area.y));
        return;
    }
    let text_area = Rect::new(area.x + 2, area.y, area.width - 2, 1);
    let scroll = query
        .visual_scroll(usize::from(text_area.width - 1))
        .min(usize::from(u16::MAX));
    frame.render_widget(
        Paragraph::new(query.value())
            .style(style)
            .scroll((0, scroll as u16)),
        text_area,
    );
    let column = query
        .visual_cursor()
        .saturating_sub(scroll)
        .min(usize::from(text_area.width - 1));
    frame.set_cursor_position((text_area.x + column as u16, text_area.y));
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
    let mut query = Input::default();
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
            let previous = if query.value() == last_query {
                matches.get(selected).copied()
            } else {
                None
            };
            matches = paths::rank(&candidates, query.value());
            selected = previous
                .and_then(|index| matches.iter().position(|candidate| *candidate == index))
                .unwrap_or(0);
            last_query = query.value().to_owned();
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
                render_query(&query, frame, areas[3]);
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
                _ => {
                    if let Some(change) =
                        input_request(key).and_then(|request| query.handle(request))
                        && change.value
                    {
                        selected = 0;
                        dirty = true;
                    }
                }
            },
            Event::Paste(text) => {
                for c in text.chars().filter(|c| !c.is_control()) {
                    query.handle(InputRequest::InsertChar(c));
                    selected = 0;
                    dirty = true;
                }
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
    fn readline_keys_edit_at_the_cursor() {
        for modifier in [KeyModifiers::ALT, KeyModifiers::META] {
            let mut query = Input::new("中文 test".to_owned());
            for (code, modifiers) in [
                (KeyCode::Char('b'), modifier),
                (KeyCode::Char('d'), modifier),
                (KeyCode::Char('X'), KeyModifiers::SHIFT),
                (KeyCode::Char('a'), KeyModifiers::CONTROL),
                (KeyCode::Char('!'), KeyModifiers::NONE),
                (KeyCode::Char('e'), KeyModifiers::CONTROL),
            ] {
                query.handle(input_request(KeyEvent::new(code, modifiers)).unwrap());
            }
            assert_eq!(query.value(), "!中文 X");
            assert_eq!(query.cursor(), query.value().chars().count());
        }
        let mut query = Input::new("a👩‍💻b".to_owned());
        for code in [KeyCode::Left, KeyCode::Backspace] {
            query.handle(input_request(KeyEvent::new(code, KeyModifiers::NONE)).unwrap());
        }
        assert_eq!(query.value(), "ab");
        assert_eq!(query.cursor(), 1);
    }

    #[test]
    fn query_scrolls_to_keep_the_cursor_visible() {
        let mut query = Input::new("ab中文z".to_owned());
        let mut terminal = Terminal::new(TestBackend::new(8, 1)).unwrap();
        terminal
            .draw(|frame| render_query(&query, frame, frame.area()))
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

        query.handle(InputRequest::GoToStart);
        terminal
            .draw(|frame| render_query(&query, frame, frame.area()))
            .unwrap();
        assert_eq!(terminal.backend().buffer()[(2, 0)].symbol(), "a");
        assert_eq!(
            terminal.backend_mut().get_cursor_position().unwrap(),
            (2, 0).into()
        );
        for width in 1..=3 {
            let mut terminal = Terminal::new(TestBackend::new(width, 1)).unwrap();
            terminal
                .draw(|frame| render_query(&query, frame, frame.area()))
                .unwrap();
            assert!(terminal.backend_mut().get_cursor_position().unwrap().x < width);
        }
    }
}
