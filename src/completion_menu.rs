use anyhow::{Context, Result, ensure};
use clap_complete::engine::CompletionCandidate;
use crossterm::{
    cursor::{Hide, MoveDown, MoveToColumn, MoveUp, RestorePosition, SavePosition, Show},
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, disable_raw_mode, enable_raw_mode},
};
use ratatui::text::Span;
use std::{
    ffi::OsString,
    fs::{File, OpenOptions},
    io::Write,
};
use unicode_segmentation::UnicodeSegmentation;

struct Menu {
    tty: File,
    height: u16,
    width: u16,
}

impl Menu {
    fn open(count: usize) -> Result<Self> {
        let mut tty = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")
            .context("Path completion requires a terminal")?;
        let (width, rows) = terminal::size()?;
        ensure!(
            rows >= 3,
            "Path completion needs at least three terminal rows"
        );
        let height = (count.min(5) as u16 + 1).min(rows - 1);
        enable_raw_mode()?;
        let result = (|| {
            // Raw line feeds retain the input's column and scroll when necessary.
            // Save the prompt's position after reserving space so scrolling cannot
            // leave the saved cursor pointing at the old screen row.
            queue!(tty, Hide)?;
            write!(tty, "{}", "\n".repeat(usize::from(height)))?;
            execute!(tty, MoveUp(height), SavePosition)?;
            Ok(Self { tty, height, width })
        })();
        if result.is_err() {
            let _ = disable_raw_mode();
        }
        result
    }

    fn line(&mut self, text: &str) -> Result<()> {
        let mut available = usize::from(self.width.saturating_sub(1));
        queue!(self.tty, MoveToColumn(0), Clear(ClearType::CurrentLine))?;
        for grapheme in text.graphemes(true) {
            let width = Span::raw(grapheme).width();
            if width > available {
                break;
            }
            queue!(self.tty, Print(grapheme))?;
            available -= width;
        }
        Ok(())
    }

    fn draw(&mut self, candidates: &[CompletionCandidate], selected: usize) -> Result<()> {
        queue!(self.tty, RestorePosition, SavePosition, MoveDown(1))?;
        let visible = usize::from(self.height - 1);
        let offset = selected.saturating_sub(visible - 1);
        for (index, candidate) in candidates.iter().enumerate().skip(offset).take(visible) {
            let value = candidate.get_value().to_string_lossy();
            let label: String = value
                .chars()
                .flat_map(|c| {
                    if c.is_control() {
                        c.escape_default().collect::<Vec<_>>()
                    } else {
                        vec![c]
                    }
                })
                .collect();
            let marker = if index == selected { "› " } else { "  " };
            if index == selected {
                queue!(self.tty, SetForegroundColor(Color::Cyan))?;
            }
            self.line(&format!("{marker}{label}"))?;
            queue!(self.tty, ResetColor, MoveDown(1))?;
        }
        queue!(self.tty, SetForegroundColor(Color::DarkGrey))?;
        self.line(&format!(
            "{} / {}  ↑↓/Tab select · Enter insert · Esc cancel",
            selected + 1,
            candidates.len()
        ))?;
        execute!(self.tty, ResetColor)?;
        Ok(())
    }
}

impl Drop for Menu {
    fn drop(&mut self) {
        let _ = queue!(self.tty, RestorePosition, SavePosition);
        for _ in 0..self.height {
            let _ = queue!(
                self.tty,
                MoveDown(1),
                MoveToColumn(0),
                Clear(ClearType::CurrentLine)
            );
        }
        let _ = execute!(self.tty, RestorePosition, ResetColor, Show);
        let _ = disable_raw_mode();
    }
}

pub fn select(candidates: &[CompletionCandidate]) -> Result<Option<OsString>> {
    if candidates.len() <= 1 {
        return Ok(candidates
            .first()
            .map(|candidate| candidate.get_value().to_owned()));
    }
    let mut menu = Menu::open(candidates.len())?;
    let mut selected = 0;
    loop {
        menu.draw(candidates, selected)?;
        let key = match event::read()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => key,
            Event::Resize(..) => return Ok(None),
            _ => continue,
        };
        match key.code {
            KeyCode::Esc => return Ok(None),
            KeyCode::Char('c' | 'd') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(None);
            }
            KeyCode::Enter => return Ok(Some(candidates[selected].get_value().to_owned())),
            KeyCode::Down | KeyCode::Tab => selected = (selected + 1) % candidates.len(),
            KeyCode::Up | KeyCode::BackTab => {
                selected = (selected + candidates.len() - 1) % candidates.len()
            }
            _ => continue,
        }
    }
}
