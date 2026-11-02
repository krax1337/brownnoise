# brownnoise

Endless, gapless white, pink and brown noise in your terminal.

[![CI](https://github.com/krax1337/brownnoise/actions/workflows/ci.yml/badge.svg)](https://github.com/krax1337/brownnoise/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/krax1337/brownnoise)](https://github.com/krax1337/brownnoise/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Why

Most noise apps and videos play a recorded clip on repeat. You can hear the
seam every time it loops, and once you notice it you keep noticing it.

`brownnoise` has no clip and no loop point. It generates the noise sample by
sample on the audio thread for as long as it runs, so the sound never repeats
and never pauses. The left and right channels are independent, which gives a
wide stereo image instead of a sound in the middle of your head. If you plug in
headphones or connect AirPods, playback follows the new output device on its
own.

## Install

### Homebrew (macOS, Linux)

```sh
brew install krax1337/tap/brownnoise
```

### Prebuilt binaries

Download the archive for your platform from the
[latest release](https://github.com/krax1337/brownnoise/releases/latest),
extract it and put `brownnoise` somewhere on your `PATH`.

| Platform              | Archive                                              |
| --------------------- | ---------------------------------------------------- |
| macOS (Apple Silicon) | `brownnoise-vX.Y.Z-aarch64-apple-darwin.tar.gz`      |
| macOS (Intel)         | `brownnoise-vX.Y.Z-x86_64-apple-darwin.tar.gz`       |
| Linux (x86_64)        | `brownnoise-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz`  |
| Linux (aarch64)       | `brownnoise-vX.Y.Z-aarch64-unknown-linux-gnu.tar.gz` |
| Windows (x86_64)      | `brownnoise-vX.Y.Z-x86_64-pc-windows-msvc.zip`       |

Checksums are in `SHA256SUMS` next to the archives:

```sh
sha256sum --check --ignore-missing SHA256SUMS
```

On Linux the binary needs the ALSA runtime library (`libasound2` on
Debian/Ubuntu, `alsa-lib` on Fedora and Arch), which most desktop systems
already have.

### Cargo

Until the first crates.io release, install from Git (Rust 1.85 or newer):

```sh
cargo install --locked --git https://github.com/krax1337/brownnoise
```

### Build from source

```sh
git clone https://github.com/krax1337/brownnoise
cd brownnoise
cargo build --release
./target/release/brownnoise
```

On Linux, install the ALSA headers and `pkg-config` first:

```sh
sudo apt install libasound2-dev pkg-config   # Debian, Ubuntu
sudo dnf install alsa-lib-devel pkgconf      # Fedora
sudo pacman -S alsa-lib pkgconf              # Arch
```

## Usage

```sh
brownnoise                      # last used settings (brown, 50% on first run)
brownnoise -c pink -v 30        # pink noise at 30% volume
brownnoise --tone 20            # deeper, darker sound
brownnoise -t 45m               # stop after 45 minutes
brownnoise -c brown -t 1h30m    # fall asleep to brown noise
```

### Options

| Option                  | Description                                                                 | Default                    |
| ----------------------- | --------------------------------------------------------------------------- | -------------------------- |
| `-c, --color <COLOR>`   | `white`, `pink` or `brown`                                                  | last used, else `brown`    |
| `-v, --volume <0-100>`  | Starting volume                                                             | last used, else `50`       |
| `--tone <0-100>`        | `0` is deepest, `50` neutral, `100` brightest                               | last used, else `50`       |
| `-t, --timer <DUR>`     | Stop after `DUR`, fading out over the last 10 s: `45m`, `1h30m`, `90s`, `20` (bare number = minutes) | none (play until you quit) |
| `-h, --help`            | Print help                                                                  |                            |
| `-V, --version`         | Print version                                                               |                            |

### Keys

| Key                       | Action                             |
| ------------------------- | ---------------------------------- |
| `↑` / `↓`, `+` / `-` or `k` / `j` | Volume up / down           |
| `←` / `→` or `h` / `l`    | Tone deeper / brighter             |
| `r`                       | Reset tone to neutral (50)         |
| `w` / `p` / `b`           | White / pink / brown noise         |
| `c` or `Tab`              | Cycle color                        |
| `space`                   | Pause / resume                     |
| `q`, `Esc` or `Ctrl+C`    | Quit (with a short fade-out)       |

When `brownnoise` is not attached to a terminal (for example
`brownnoise -t 1h &`), it plays without the UI until it receives `SIGINT` or
`SIGTERM`, or the timer ends.

### Colors

- **White** has equal energy at every frequency: a bright, hissy sound like
  TV static.
- **Pink** loses energy as frequency rises (−3 dB per octave): balanced and
  soft, like steady rain.
- **Brown** drops off faster (−6 dB per octave): a deep, low rumble like a
  waterfall or distant surf.

The tone control tilts any color darker or brighter on top of that.

## Configuration

`brownnoise` remembers the color, volume and tone you had when you quit and
starts with them next time. Command-line flags override the saved values for
that run only.

| Platform      | Path                                                                         |
| ------------- | ---------------------------------------------------------------------------- |
| macOS, Linux  | `$XDG_CONFIG_HOME/brownnoise/config.toml`, else `~/.config/brownnoise/config.toml` |
| Windows       | `%APPDATA%\brownnoise\config.toml`                                           |

Delete the file to go back to the defaults.

## License

[MIT](LICENSE) © 2026 Asanali Bulatov
