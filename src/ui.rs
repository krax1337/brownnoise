//! Two-line terminal UI: a static key hint line and a live status line.

use std::io::{self, IsTerminal, Write};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::style::{Color as TermColor, Stylize};
use crossterm::{cursor, execute, terminal};

use crate::dsp::Color;

pub enum Action {
    Volume(i8),
    Tone(i8),
    ResetTone,
    SetColor(Color),
    CycleColor,
    TogglePause,
    Quit,
}

pub struct Status {
    pub color: Color,
    pub volume: u8,
    pub tone: u8,
    pub paused: bool,
    pub remaining: Option<Duration>,
    /// Fade-out time left once quitting has started.
    pub quitting: Option<Duration>,
}

const HINTS: &str = "↑↓ volume · ←→ tone · r reset tone · w/p/b color · space pause · q quit";

pub struct Ui {
    interactive: bool,
    last: String,
}

impl Ui {
    /// Interactive only when both stdin and stdout are terminals.
    pub fn new() -> Ui {
        Ui {
            interactive: io::stdin().is_terminal() && io::stdout().is_terminal(),
            last: String::new(),
        }
    }

    pub fn start(&mut self) -> io::Result<()> {
        if !self.interactive {
            return Ok(());
        }
        terminal::enable_raw_mode()?;
        let hints = truncate(HINTS, width());
        let mut out = io::stdout();
        execute!(out, cursor::Hide)?;
        write!(out, "{}\r\n", hints.dim())?;
        out.flush()
    }

    pub fn finish(&mut self) {
        if !self.interactive {
            return;
        }
        let mut out = io::stdout();
        // Erase both our lines; the shell prompt returns where the hints were.
        let _ = execute!(
            out,
            cursor::MoveToColumn(0),
            cursor::MoveUp(1),
            terminal::Clear(terminal::ClearType::FromCursorDown),
            cursor::Show
        );
        let _ = terminal::disable_raw_mode();
    }

    /// Wait up to `timeout` for a key. Non-interactive runs just sleep.
    pub fn poll(&mut self, timeout: Duration) -> Option<Action> {
        if !self.interactive {
            std::thread::sleep(timeout);
            return None;
        }
        if !event::poll(timeout).ok()? {
            return None;
        }
        match event::read().ok()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => map_key(key),
            Event::Resize(..) => {
                self.last.clear(); // force redraw at the new width
                None
            }
            _ => None,
        }
    }

    pub fn draw(&mut self, status: &Status) {
        if !self.interactive {
            return;
        }
        let name = format!("{:<5}", status.color.name());
        let line = name.clone() + &status_line(status, width());
        if line == self.last {
            return;
        }
        let styled = name.as_str().with(term_color(status.color)).bold();
        let mut out = io::stdout();
        let _ = write!(out, "\r\x1b[2K{styled}{}", &line[name.len()..]);
        let _ = out.flush();
        self.last = line;
    }
}

fn map_key(key: KeyEvent) -> Option<Action> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return matches!(key.code, KeyCode::Char('c') | KeyCode::Char('d')).then_some(Action::Quit);
    }
    Some(match key.code {
        KeyCode::Up | KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Char('k') => {
            Action::Volume(5)
        }
        KeyCode::Down | KeyCode::Char('-') | KeyCode::Char('_') | KeyCode::Char('j') => {
            Action::Volume(-5)
        }
        KeyCode::Right | KeyCode::Char('l') => Action::Tone(5),
        KeyCode::Left | KeyCode::Char('h') => Action::Tone(-5),
        KeyCode::Char('r') => Action::ResetTone,
        KeyCode::Char('w') => Action::SetColor(Color::White),
        KeyCode::Char('p') => Action::SetColor(Color::Pink),
        KeyCode::Char('b') => Action::SetColor(Color::Brown),
        KeyCode::Char('c') | KeyCode::Tab => Action::CycleColor,
        KeyCode::Char(' ') => Action::TogglePause,
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::Quit,
        _ => return None,
    })
}

fn term_color(color: Color) -> TermColor {
    match color {
        Color::White => TermColor::White,
        Color::Pink => TermColor::Magenta,
        Color::Brown => TermColor::DarkYellow,
    }
}

/// Everything after the colored name. Uses the widest layout that fits `width`;
/// at 20 cells every 5-point volume/tone step moves the bar by exactly one cell.
fn status_line(s: &Status, width: usize) -> String {
    let mut tail = match (s.quitting, s.paused) {
        (Some(left), _) => format!("⏹ quitting, fading out {:.1}s", left.as_secs_f32()),
        (None, true) => "⏸ paused".to_string(),
        (None, false) => "▶ playing".to_string(),
    };
    if let (Some(left), None) = (s.remaining, s.quitting) {
        let secs = left.as_secs();
        tail += &format!("  ⏱ {}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60);
    }
    let layouts = [(20, true), (20, false), (10, false)];
    let mut line = String::new();
    for (cells, labels) in layouts {
        line = layout(s, cells, labels, &tail);
        if line.chars().count() + 6 <= width {
            return line;
        }
    }
    truncate(&line, width.saturating_sub(5))
}

fn layout(s: &Status, cells: usize, labels: bool, tail: &str) -> String {
    let filled = (s.volume as usize * cells + 50) / 100;
    let knob = (s.tone as usize * cells + 50) / 100;
    // cells + 1 positions so neutral (50) sits exactly on the ┼ mark.
    let slider: String = (0..=cells)
        .map(|i| match i {
            _ if i == knob => '●',
            _ if i == cells / 2 => '┼',
            _ => '─',
        })
        .collect();
    let (deep, bright) = if labels {
        ("deep ", " bright")
    } else {
        ("", "")
    };
    format!(
        "  vol {}{} {:>3}%   tone {deep}{slider}{bright} {:>3}   {tail}",
        "█".repeat(filled),
        "░".repeat(cells - filled),
        s.volume,
        s.tone,
    )
}

fn width() -> usize {
    terminal::size().map(|(w, _)| w as usize).unwrap_or(80)
}

/// Cut by characters; every glyph used here is one column wide.
fn truncate(s: &str, max: usize) -> String {
    s.chars().take(max.saturating_sub(1)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(volume: u8, tone: u8) -> Status {
        Status {
            color: Color::Brown,
            volume,
            tone,
            paused: false,
            remaining: None,
            quitting: None,
        }
    }

    #[test]
    fn every_step_moves_the_sliders() {
        for width in [200, 70] {
            let tones: Vec<String> = (0..=100)
                .step_by(5)
                .map(|t| status_line(&status(50, t), width))
                .collect();
            let vols: Vec<String> = (0..=100)
                .step_by(5)
                .map(|v| status_line(&status(v, 50), width))
                .collect();
            for pair in tones.windows(2).chain(vols.windows(2)) {
                assert_ne!(pair[0], pair[1], "width {width}");
            }
        }
        // On a wide terminal the knob itself (not just the number) moves each step.
        let knob = |t| {
            status_line(&status(50, t), 200)
                .chars()
                .position(|c| c == '●')
                .unwrap()
        };
        assert!(
            (0..100)
                .step_by(5)
                .all(|t| knob(t as u8) < knob(t as u8 + 5))
        );
    }

    #[test]
    fn neutral_tone_sits_on_the_center_mark() {
        let line = status_line(&status(50, 50), 200);
        assert!(
            !line.contains('┼'),
            "knob should cover the center mark: {line}"
        );
        assert!(status_line(&status(50, 55), 200).contains('┼'));
    }

    #[test]
    fn fits_narrow_terminals() {
        let s = Status {
            remaining: Some(std::time::Duration::from_secs(3600)),
            ..status(50, 50)
        };
        for width in [40, 60, 80, 100, 120] {
            assert!(
                status_line(&s, width).chars().count() + 5 < width,
                "width {width}"
            );
        }
    }
}
