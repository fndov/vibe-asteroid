use std::io::{self, Write};
use std::collections::HashMap;
use crossterm::{
    terminal::{enable_raw_mode, disable_raw_mode, size},
    cursor::{Hide, Show},
    event::{Event, KeyCode},
};
use log::{info, error};
use simple_logging;
use std::env;

pub mod constants;
pub mod types;
pub mod rendering;
pub mod entities;
pub mod upgrades;
pub mod terminal_io;
pub mod game;

use crate::rendering::{OutputTarget, ScreenBuffer};
use crate::terminal_io::SimulatedInput;
use crate::game::Game;

fn main() -> io::Result<()> {
    simple_logging::log_to_file("vibe-asteroid.log", log::LevelFilter::Info).unwrap();
    info!("Starting Vibe-asteroid application.");

    let mut stdout_target;
    let simulated_input: Option<SimulatedInput>;

    let args: Vec<String> = env::args().collect();
    let debug_mode_active = args.len() > 1 && args[1] == "--debug";

    let terminal_width: u16;
    let terminal_height: u16;

    if debug_mode_active {
        info!("Debug mode enabled.");
        let mut debug_width = 80;
        let mut debug_height = 24;
        if args.len() >= 4 {
            debug_width = args[2].parse::<u16>().unwrap_or(80);
            debug_height = args[3].parse::<u16>().unwrap_or(24);
        }
        terminal_width = debug_width;
        terminal_height = debug_height;
        info!("Debug resolution set to {}x{}", terminal_width, terminal_height);
        stdout_target = OutputTarget::ScreenBuffer(ScreenBuffer::new(terminal_width, terminal_height));
        let mut sim_events = HashMap::new();
        sim_events.insert(1, Event::Key(KeyCode::Up.into()));
        sim_events.insert(2, Event::Key(KeyCode::Right.into()));
        sim_events.insert(3, Event::Key(KeyCode::Char(' ').into()));
        sim_events.insert(4, Event::Key(KeyCode::Left.into()));
        sim_events.insert(10, Event::Key(KeyCode::Char('q').into())); // Quit after 10 frames
        simulated_input = Some(SimulatedInput::new(sim_events));
    } else {
        info!("Attempting to enable raw mode.");
        enable_raw_mode().map_err(|e| { error!("Failed to enable raw mode: {}", e); e })?;
        info!("Raw mode enabled.");
        let (width, height) = size().map_err(|e| { error!("Failed to get terminal size: {}", e); e })?;
        terminal_width = width;
        terminal_height = height;
        stdout_target = OutputTarget::Stdout(io::stdout());
        info!("Terminal size: {}x{}", terminal_width, terminal_height);
        simulated_input = None; // No simulated input in non-debug mode
    }

    let max_frames: Option<u64> = if !debug_mode_active && args.len() > 1 {
        match args[1].parse::<u64>() {
            Ok(num) => Some(num),
            Err(_) => None,
        }
    } else if debug_mode_active && args.len() > 4 {
        match args[4].parse::<u64>() {
            Ok(num) => Some(num),
            Err(_) => None,
        }
    } else {
        None
    };

    info!("Attempting to clear screen and hide cursor.");
    if !debug_mode_active {
        stdout_target.execute_other_command(Hide)?;
    }
    stdout_target.flush()?;
    info!("Screen cleared and cursor hidden.");

    let mut game = Game::new(
        terminal_width,
        terminal_height,
        stdout_target,
        simulated_input,
        debug_mode_active,
        max_frames,
    );

    game.run()?;

    info!("Game loop ended. Displaying game over screen.");

    if !debug_mode_active {
        game.stdout_target.execute_other_command(Show)?;
        disable_raw_mode()?;
    }

    Ok(())
}