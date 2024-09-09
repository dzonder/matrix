//! Matrix rain in the terminal.

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{read, Event, KeyCode};
use crossterm::execute;
use crossterm::style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::terminal::{Clear, ClearType};
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use rand::Rng;
use std::cmp;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// How long to sleep between animation frames.
const FRAME_SLEEP: Duration = Duration::from_millis(50);

/// Minimum length of a droplet.
const DROPLET_MIN_LENGTH: u16 = 2;

/// Maximum length of a droplet.
const DROPLET_MAX_LENGTH: u16 = 20;

/// Minimum speed of a droplet.
const DROPLET_MIN_SPEED: f32 = 0.2;

/// Maximum speed of a droplet.
const DROPLET_MAX_SPEED: f32 = 1.0;

/// Base color of the droplet.
const BASE_COLOR: (u8, u8, u8) = (170, 255, 170);

/// Holds information about a single droplet.
struct Droplet {
    row: u16,
    len: u16,
    max_len: u16,
    frame: f32, // 1.0 -> draw next frame
    speed: f32, // (0.0, 1.0]
}

/// Generate a random character.
fn random_char() -> char {
    let mut rng = rand::thread_rng();
    let katakana_start = 0xFF66; // Half-width katakana 'ｦ'
    let katakana_end = 0xFF9D; // Half-width katakana 'ﾝ'
    let random_char = rng.gen_range(katakana_start..katakana_end);
    char::from_u32(random_char).unwrap()
}

/// Linear gradient of the droplet's color based on distance from the bottom (0) to the top
/// (len - 1).
fn color_gradient(droplet: &Droplet, distance: u16) -> Color {
    let scale = (droplet.len as f64 - distance as f64) / droplet.len as f64;
    Color::Rgb {
        r: (BASE_COLOR.0 as f64 * scale) as u8,
        g: (BASE_COLOR.1 as f64 * scale) as u8,
        b: (BASE_COLOR.2 as f64 * scale) as u8,
    }
}

/// Draw and advance to the next frame.
fn draw_next_frame(cols: u16, rows: u16, droplets: &mut Vec<Droplet>) -> io::Result<()> {
    let mut rng = rand::thread_rng();
    for col in 0..cols {
        let droplet = &mut droplets[col as usize];
        droplet.frame += droplet.speed;
        if droplet.frame < 1.0 {
            continue;
        }
        if droplet.row >= rows + droplet.len {
            // Droplet out of screen, create a new one.
            *droplet = Droplet {
                row: rng.gen_range(0..rows / 4), // New droplets at the top of the screen.
                len: 1,
                max_len: rng.gen_range(DROPLET_MIN_LENGTH..=DROPLET_MAX_LENGTH),
                frame: 1.0,
                speed: rng.gen_range(DROPLET_MIN_SPEED..=DROPLET_MAX_SPEED),
            };
            continue;
        }
        for distance in 0..droplet.len + 1 {
            if droplet.row >= distance && droplet.row - distance < rows {
                execute!(
                    io::stdout(),
                    MoveTo(col, droplet.row - distance),
                    SetForegroundColor(color_gradient(droplet, distance)),
                    Print(random_char()),
                )?;
            }
        }
        if droplet.row > droplet.len - 1 {
            // Fade totally when length reached.
            execute!(
                io::stdout(),
                MoveTo(col, droplet.row - droplet.len),
                SetForegroundColor(Color::Reset),
                Print(' '),
            )?;
        }
        // Move to next frame and extend the droplet if needed.
        droplet.frame -= 1.0;
        droplet.row += 1;
        droplet.len = cmp::min(droplet.len + 1, droplet.max_len);
    }
    Ok(())
}

/// Setup the terminal, initialize the droplets, and spawn key check and drawing loops.
fn main() -> io::Result<()> {
    let mut rng = rand::thread_rng();

    enable_raw_mode()?;
    execute!(
        io::stdout(),
        EnterAlternateScreen,
        Hide,
        SetBackgroundColor(Color::Black),
        Clear(ClearType::All)
    )?;

    let (cols, rows) = crossterm::terminal::size()?;

    let mut droplets: Vec<Droplet> = (0..cols)
        .map(|_| {
            let len = rng.gen_range(DROPLET_MIN_LENGTH..=DROPLET_MAX_LENGTH);
            Droplet {
                row: rng.gen_range(0..rows),
                len: len,
                max_len: len,
                frame: 1.0,
                speed: rng.gen_range(DROPLET_MIN_SPEED..=DROPLET_MAX_SPEED),
            }
        })
        .collect();

    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    thread::spawn(move || {
        while running_clone.load(Ordering::Relaxed) {
            if let Ok(Event::Key(key)) = read() {
                if key.code == KeyCode::Char('q') {
                    running_clone.store(false, Ordering::Relaxed);
                }
            }
        }
    });

    while running.load(Ordering::Relaxed) {
        draw_next_frame(cols, rows, &mut droplets)?;
        thread::sleep(FRAME_SLEEP);
    }

    execute!(io::stdout(), LeaveAlternateScreen, ResetColor, Show)?;
    disable_raw_mode()
}
