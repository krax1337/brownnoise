mod audio;
mod config;
mod dsp;
mod ui;

use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use clap::Parser;

use audio::Player;
use dsp::{Color, Params};
use ui::{Action, Status, Ui};

const KEYS: &str = "\
Keys while playing:
  ↑/↓  +/-   volume           ←/→   tone (deeper/brighter)
  r          reset tone       w p b white/pink/brown, c cycles
  space      pause/resume     q Esc Ctrl+C  quit (fades out)

Color, volume and tone are remembered between runs.";

/// Endless, gapless white/pink/brown noise, generated live — no loops, no pauses.
#[derive(Parser)]
#[command(version, after_help = KEYS)]
struct Cli {
    /// Noise color [default: last used, else brown]
    #[arg(short, long, value_enum)]
    color: Option<Color>,

    /// Starting volume 0-100 [default: last used, else 50]
    #[arg(short, long, value_parser = clap::value_parser!(u8).range(0..=100))]
    volume: Option<u8>,

    /// 0 = deepest, 50 = neutral, 100 = brightest [default: last used, else 50]
    #[arg(long, value_parser = clap::value_parser!(u8).range(0..=100))]
    tone: Option<u8>,

    /// Stop after this long, fading out over the last 10s: 45m, 1h30m, 90s, 20 (= minutes)
    #[arg(short, long, value_parser = parse_duration)]
    timer: Option<Duration>,
}

fn parse_duration(s: &str) -> Result<Duration, String> {
    let bad = || format!("invalid duration '{s}' (try 45m, 1h30m, 90s, or minutes like 20)");
    if let Ok(minutes) = s.parse::<f64>() {
        return (minutes > 0.0 && minutes.is_finite())
            .then(|| Duration::from_secs_f64(minutes * 60.0))
            .ok_or_else(bad);
    }
    let mut total = 0.0;
    let mut number = String::new();
    for ch in s.to_ascii_lowercase().chars() {
        if ch.is_ascii_digit() || ch == '.' {
            number.push(ch);
            continue;
        }
        let n: f64 = number.parse().map_err(|_| bad())?;
        total += n * match ch {
            'h' => 3600.0,
            'm' => 60.0,
            's' => 1.0,
            _ => return Err(bad()),
        };
        number.clear();
    }
    (number.is_empty() && total > 0.0)
        .then(|| Duration::from_secs_f64(total))
        .ok_or_else(bad)
}

/// Perceived-even volume steps: amplitude grows with the square of the slider.
fn amplitude(volume: u8) -> f32 {
    const FULL_SCALE_RMS: f32 = 0.2;
    (volume as f32 / 100.0).powi(2) * FULL_SCALE_RMS
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let (mut settings, warning) = config::load();
    if let Some(w) = warning {
        eprintln!("brownnoise: {w}");
    }
    settings.color = cli.color.unwrap_or(settings.color);
    settings.volume = cli.volume.unwrap_or(settings.volume);
    settings.tone = cli.tone.unwrap_or(settings.tone);

    // Ctrl+C outside raw mode, SIGTERM, SIGHUP: all end in the same graceful fade.
    let signalled = Arc::new(AtomicBool::new(false));
    {
        let signalled = signalled.clone();
        let _ = ctrlc::set_handler(move || signalled.store(true, Ordering::Relaxed));
    }

    let params = Arc::new(Params::new(settings.color, settings.tone, 48_000.0));
    let mut player = match Player::start(params.clone()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("brownnoise: cannot open audio output: {e}");
            return ExitCode::FAILURE;
        }
    };
    params.set_tone(settings.tone, player.sample_rate);
    params.fade_to(amplitude(settings.volume), 2.0);

    let mut ui = Ui::new();
    if let Err(e) = ui.start() {
        eprintln!("brownnoise: terminal: {e}");
    }

    let mut paused = false;
    let mut quit_at: Option<Instant> = None;
    let mut played = Duration::ZERO;
    let mut last_tick = Instant::now();
    let mut last_rebuild_try = Instant::now();

    loop {
        let action = ui.poll(Duration::from_millis(100));
        let now = Instant::now();

        if quit_at.is_none() {
            if signalled.load(Ordering::Relaxed) {
                quit_at = Some(begin_quit(&params, 1.0));
            }
            match action {
                Some(Action::Volume(delta)) => {
                    settings.volume = settings.volume.saturating_add_signed(delta).min(100);
                    if !paused {
                        params.fade_to(amplitude(settings.volume), 0.15);
                    }
                }
                Some(Action::Tone(delta)) => {
                    settings.tone = settings.tone.saturating_add_signed(delta).min(100);
                    params.set_tone(settings.tone, player.sample_rate);
                }
                Some(Action::ResetTone) => {
                    settings.tone = 50;
                    params.set_tone(settings.tone, player.sample_rate);
                }
                Some(Action::SetColor(color)) => settings.color = color,
                Some(Action::CycleColor) => settings.color = settings.color.next(),
                Some(Action::TogglePause) => {
                    paused = !paused;
                    params.fade_to(
                        if paused {
                            0.0
                        } else {
                            amplitude(settings.volume)
                        },
                        0.6,
                    );
                }
                Some(Action::Quit) => quit_at = Some(begin_quit(&params, 1.5)),
                None => {}
            }
            params.set_color(settings.color);
        }

        if !paused {
            played += now - last_tick;
        }
        last_tick = now;

        if let (Some(limit), None) = (cli.timer, quit_at) {
            let fade = limit.min(Duration::from_secs(10));
            if played + fade >= limit {
                quit_at = Some(begin_quit(&params, fade.as_secs_f32()));
            }
        }

        if quit_at.is_some_and(|deadline| now >= deadline) {
            break;
        }

        // Device unplugged on a backend that doesn't reroute: reopen the (new) default.
        if player.is_lost() && now - last_rebuild_try >= Duration::from_secs(1) {
            last_rebuild_try = now;
            if let Ok(fresh) = Player::start(params.clone()) {
                player = fresh;
                params.set_tone(settings.tone, player.sample_rate);
                if quit_at.is_none() && !paused {
                    params.fade_to(amplitude(settings.volume), 0.5);
                }
            }
        }

        ui.draw(&Status {
            color: settings.color,
            volume: settings.volume,
            tone: settings.tone,
            paused,
            remaining: cli.timer.map(|t| t.saturating_sub(played)),
            quitting: quit_at.map(|deadline| deadline.saturating_duration_since(now)),
        });
    }

    ui.finish();
    drop(player);
    if let Err(e) = config::save(&settings) {
        eprintln!("brownnoise: {e}");
    }
    ExitCode::SUCCESS
}

fn begin_quit(params: &Params, fade_secs: f32) -> Instant {
    params.fade_to(0.0, fade_secs);
    Instant::now() + Duration::from_secs_f32(fade_secs + 0.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations() {
        assert_eq!(parse_duration("20"), Ok(Duration::from_secs(1200)));
        assert_eq!(parse_duration("90s"), Ok(Duration::from_secs(90)));
        assert_eq!(parse_duration("1h30m"), Ok(Duration::from_secs(5400)));
        assert_eq!(parse_duration("1H5M10S"), Ok(Duration::from_secs(3910)));
        assert_eq!(parse_duration("1.5h"), Ok(Duration::from_secs(5400)));
    }

    #[test]
    fn bad_durations() {
        for s in ["", "0", "-5", "abc", "10x", "m", "5m3", "inf", "0s"] {
            assert!(parse_duration(s).is_err(), "{s:?} should be rejected");
        }
    }
}
