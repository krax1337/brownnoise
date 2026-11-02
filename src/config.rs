//! Remembered settings: the color, volume and tone you last quit with.

use std::path::PathBuf;
use std::{env, fs};

use serde::{Deserialize, Serialize};

use crate::dsp::Color;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub color: Color,
    pub volume: u8,
    pub tone: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            color: Color::Brown,
            volume: 50,
            tone: 50,
        }
    }
}

impl Settings {
    fn clamped(mut self) -> Self {
        self.volume = self.volume.min(100);
        self.tone = self.tone.min(100);
        self
    }
}

/// `$XDG_CONFIG_HOME/brownnoise/config.toml`, else `~/.config/...` on Unix
/// (macOS included — CLI users expect it there), `%APPDATA%\...` on Windows.
pub fn path() -> Option<PathBuf> {
    let base = match env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None if cfg!(windows) => dirs::config_dir()?,
        None => dirs::home_dir()?.join(".config"),
    };
    Some(base.join("brownnoise").join("config.toml"))
}

/// Missing file → defaults. Unreadable or invalid file → defaults plus a warning.
pub fn load() -> (Settings, Option<String>) {
    let Some(path) = path() else {
        return (Settings::default(), None);
    };
    match fs::read_to_string(&path) {
        Ok(text) => parse(&text).map_or_else(
            |e| {
                (
                    Settings::default(),
                    Some(format!("ignoring {}: {e}", path.display())),
                )
            },
            |s| (s, None),
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Settings::default(), None),
        Err(e) => (
            Settings::default(),
            Some(format!("cannot read {}: {e}", path.display())),
        ),
    }
}

fn parse(text: &str) -> Result<Settings, String> {
    toml::from_str::<Settings>(text)
        .map(Settings::clamped)
        .map_err(|e| e.message().to_string())
}

pub fn save(settings: &Settings) -> Result<(), String> {
    let path = path().ok_or("no config directory")?;
    let fail = |e: std::io::Error| format!("cannot save {}: {e}", path.display());
    fs::create_dir_all(path.parent().expect("path has a parent")).map_err(fail)?;
    let text = toml::to_string(settings).map_err(|e| e.to_string())?;
    fs::write(&path, text).map_err(fail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_file_keeps_defaults_for_missing_keys() {
        let s = parse("volume = 30\n").unwrap();
        assert_eq!(
            s,
            Settings {
                volume: 30,
                ..Settings::default()
            }
        );
    }

    #[test]
    fn out_of_range_values_are_clamped() {
        let s = parse("volume = 250\ntone = 101\ncolor = \"pink\"\n").unwrap();
        assert_eq!(
            s,
            Settings {
                color: Color::Pink,
                volume: 100,
                tone: 100
            }
        );
    }

    #[test]
    fn invalid_color_is_an_error() {
        assert!(parse("color = \"purple\"\n").is_err());
    }

    #[test]
    fn round_trips() {
        let s = Settings {
            color: Color::White,
            volume: 7,
            tone: 93,
        };
        assert_eq!(parse(&toml::to_string(&s).unwrap()).unwrap(), s);
    }
}
