use std::io::{self, Write, Read};
use std::collections::HashMap;
use std::time::Duration;
use crossterm::{
    terminal::{enable_raw_mode, disable_raw_mode, size},
    execute,
    cursor::{MoveTo, Hide, Show},
    event::{self, Event, KeyCode},
};
use rand::Rng;
use log::{info, error};
use simple_logging;
use std::env;



// --- ScreenBuffer for simulated rendering ---
struct ScreenBuffer {
    buffer: Vec<Vec<char>>,
    width: u16,
    height: u16,
    cursor_x: u16,
    cursor_y: u16,
}

impl ScreenBuffer {
    fn new(width: u16, height: u16) -> Self {
        ScreenBuffer {
            buffer: vec![vec![' '; width as usize]; height as usize],
            width,
            height,
            cursor_x: 0,
            cursor_y: 0,
        }
    }

    fn move_to(&mut self, x: u16, y: u16) {
        self.cursor_x = x;
        self.cursor_y = y;
    }

    fn write_char(&mut self, c: char) {
        if self.cursor_y < self.height && self.cursor_x < self.width {
            self.buffer[self.cursor_y as usize][self.cursor_x as usize] = c;
        }
    }

    fn write_str(&mut self, s: &str) {
        for c in s.chars() {
            self.write_char(c);
            self.cursor_x += 1;
        }
    }

    fn set_char(&mut self, x: u16, y: u16, c: char) {
        if y < self.height && x < self.width {
            self.buffer[y as usize][x as usize] = c;
        }
    }

    fn clear(&mut self) {
        self.buffer = vec![vec![' '; self.width as usize]; self.height as usize];
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    fn print_to_log(&self) {
        info!("--- Screen Buffer ---");
        for row in &self.buffer {
            info!("{}", row.iter().collect::<String>());
        }
        info!("---------------------");
    }
}

impl Write for ScreenBuffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s = String::from_utf8_lossy(buf);
        self.write_str(&s);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

// --- OutputTarget enum to handle stdout or ScreenBuffer ---
enum OutputTarget {
    Stdout(io::Stdout),
    ScreenBuffer(ScreenBuffer),
}

impl OutputTarget {
    fn execute_move_to(&mut self, command: crossterm::cursor::MoveTo) -> io::Result<()> {
        match self {
            OutputTarget::Stdout(s) => execute!(s, command),
            OutputTarget::ScreenBuffer(sb) => {
                sb.move_to(command.0, command.1);
                Ok(())
            },
        }
    }

    fn execute_other_command(&mut self, command: impl crossterm::Command) -> io::Result<()> {
        match self {
            OutputTarget::Stdout(s) => execute!(s, command),
            OutputTarget::ScreenBuffer(_) => Ok(()), // Ignore in debug mode
        }
    }
}

impl Write for OutputTarget {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            OutputTarget::Stdout(s) => s.write(buf),
            OutputTarget::ScreenBuffer(sb) => {
                let s = String::from_utf8_lossy(buf);
                sb.write_str(&s);
                Ok(buf.len())
            },
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            OutputTarget::Stdout(s) => s.flush(),
            OutputTarget::ScreenBuffer(sb) => sb.flush(),
        }
    }
}

// --- SimulatedInput for debugging ---
struct SimulatedInput {
    events: HashMap<u64, Event>,
    current_frame: u64,
}

impl SimulatedInput {
    fn new(events: HashMap<u64, Event>) -> Self {
        SimulatedInput { events, current_frame: 0 }
    }

    fn poll(&mut self, frame_count: u64) -> io::Result<bool> {
        self.current_frame = frame_count;
        Ok(self.events.contains_key(&frame_count))
    }

    fn read(&mut self) -> io::Result<Event> {
        if let Some(event) = self.events.remove(&self.current_frame) {
            Ok(event)
        } else {
            Ok(Event::Key(KeyCode::Null.into()))
        }
    }
}

// --- GameGrid for geometric rendering ---
struct GameGrid {
    grid: Vec<Vec<char>>,
    width: u16,
    height: u16,
}

impl GameGrid {
    fn new(width: u16, height: u16) -> Self {
        GameGrid {
            grid: vec![vec![' '; width as usize]; height as usize],
            width,
            height,
        }
    }

    fn set_char(&mut self, x: u16, y: u16, c: char) {
        if y < self.height && x < self.width {
            self.grid[y as usize][x as usize] = c;
        }
    }

    fn clear(&mut self) {
        self.grid = vec![vec![' '; self.width as usize]; self.height as usize];
    }

    fn render(&self, stdout: &mut OutputTarget) -> io::Result<()> {
        for y in 0..self.height {
            stdout.execute_move_to(MoveTo(0, y))?;
            write!(stdout, "{}", self.grid[y as usize].iter().collect::<String>())?;
        }
        Ok(())
    }

    fn clear_screen_manual(&self, stdout: &mut OutputTarget, terminal_width: u16, terminal_height: u16) -> io::Result<()> {
        for y in 0..terminal_height {
            stdout.execute_move_to(MoveTo(0, y))?;
            write!(stdout, "{}", " ".repeat(terminal_width as usize))?;
        }
        stdout.execute_move_to(MoveTo(0, 0))?;
        Ok(())
    }
}

// --- Ship and Asteroid structs (modified for geometric rendering) ---
struct Ship {
    x: u16,
    y: u16,
    shape: Vec<(i16, i16)>, // Relative coordinates for diamond shape
    display_char: char,
}

impl Ship {
    fn new(x: u16, y: u16) -> Self {
        let shape = vec![
            (0, 0), // Center
            (-1, 0), (1, 0), (0, -1), (0, 1), // Diamond points
        ];
        Ship { x, y, shape, display_char: '#' }
    }

    fn get_absolute_coords(&self) -> Vec<(u16, u16)> {
        self.shape.iter().map(|&(dx, dy)| {
            ((self.x as i16 + dx) as u16, (self.y as i16 + dy) as u16)
        }).collect()
    }

    fn draw(&self, game_grid: &mut GameGrid) {
        for &(dx, dy) in &self.shape {
            let draw_x = (self.x as i16 + dx) as u16;
            let draw_y = (self.y as i16 + dy) as u16;
            game_grid.set_char(draw_x, draw_y, self.display_char);
        }
    }

    fn update(&mut self, direction: KeyCode, terminal_width: u16) {
        self.x = self.x.saturating_add(match direction {
            KeyCode::Left => -1,
            KeyCode::Right => 1,
            _ => 0,
        } as u16);
        self.x = self.x.min(terminal_width - 1).max(0);
    }
}

struct Asteroid {
    x: u16,
    y: u16,
    shape: Vec<(i16, i16)>, // Relative coordinates for bumpy shape
    display_char: char,
}

impl Asteroid {
    fn new(x: u16, y: u16, rng: &mut impl Rng) -> Self {
        let shape = match rng.gen_range(0..3) {
            0 => vec![(0, 0)], // Small asteroid (single point)
            1 => vec![(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)], // Medium bumpy
            _ => vec![(0, 0), (-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)], // Large bumpy
        };
        Asteroid { x, y, shape, display_char: '@' }
    }

    fn get_absolute_coords(&self) -> Vec<(u16, u16)> {
        self.shape.iter().map(|&(dx, dy)| {
            ((self.x as i16 + dx) as u16, (self.y as i16 + dy) as u16)
        }).collect()
    }

    fn draw(&self, game_grid: &mut GameGrid) {
        for &(dx, dy) in &self.shape {
            let draw_x = (self.x as i16 + dx) as u16;
            let draw_y = (self.y as i16 + dy) as u16;
            game_grid.set_char(draw_x, draw_y, self.display_char);
        }
    }

    fn update(&mut self) {
        self.y += 1;
    }
}

// --- Main function (modified for debug mode and simulated rendering/input) ---
fn main() -> io::Result<()> {
    simple_logging::log_to_file("vibe-asteroid.log", log::LevelFilter::Info).unwrap();
    info!("Starting Vibe-asteroid application.");

    let mut stdout_target: OutputTarget;
    let mut simulated_input: Option<SimulatedInput> = None;

    let (terminal_width, terminal_height);

    let args: Vec<String> = env::args().collect();
    let debug_mode_active = args.len() > 1 && args[1] == "--debug";

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
        sim_events.insert(1, Event::Key(KeyCode::Right.into()));
        sim_events.insert(2, Event::Key(KeyCode::Right.into()));
        sim_events.insert(3, Event::Key(KeyCode::Left.into()));
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
    }

    let _max_frames: Option<u64> = if !debug_mode_active && args.len() > 1 {
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
    // Initial clear using crossterm for real terminal, or just clear buffer for debug
    if !debug_mode_active {
        let game_grid_dummy = GameGrid::new(terminal_width, terminal_height); // Dummy for clear_screen_manual
        game_grid_dummy.clear_screen_manual(&mut stdout_target, terminal_width, terminal_height).map_err(|e| { error!("Failed to clear screen manually: {}", e); e })?;
        stdout_target.execute_other_command(Hide).map_err(|e| { error!("Failed to hide cursor: {}", e); e })?;
    }
    stdout_target.flush().map_err(|e| { error!("Failed to flush stdout after initial clear: {}", e); e })?;
    info!("Screen cleared and cursor hidden.");

    // Title Screen (only in non-debug mode)
    if !debug_mode_active {
        let title_art = [
            r"VIBE-STROID",
            r" _   _ _____ _____ ____  _____ ____  _   _ ____  _",
            r"| | | | ____|_   _|  _ \| ____|  _ \| | | |  _ \| |",
            r"| |_| |  _|   | | | |_) |  _| | |_) | |_| | |_) | |",
            r"|  _  | |___  | | |  _ <| |___|  _ <|  _  |  _ <| |",
            r"|_| |_|_____| |_| |_| \|_____|_| \|_| |_|_| \|_|",
        ];

        let title_start_y = terminal_height / 2 - title_art.len() as u16 / 2;
        for (i, line) in title_art.iter().enumerate() {
            let x = terminal_width / 2 - line.len() as u16 / 2;
            stdout_target.execute_move_to(MoveTo(x, title_start_y + i as u16)).map_err(|e| { error!("Failed to move cursor for title art: {}", e); e })?;
            write!(stdout_target, "{}", line).map_err(|e| { error!("Failed to write title art: {}", e); e })?;
            stdout_target.flush().map_err(|e| { error!("Failed to flush stdout after title art: {}", e); e })?;
        }

        let press_any_key_msg = "Press any key to start...";
        let msg_x = terminal_width / 2 - press_any_key_msg.len() as u16 / 2;
        stdout_target.execute_move_to(MoveTo(msg_x, terminal_height - 5)).map_err(|e| { error!("Failed to move cursor for start message: {}", e); e })?;
        write!(stdout_target, "{}", press_any_key_msg).map_err(|e| { error!("Failed to write start message: {}", e); e })?;
        stdout_target.flush().map_err(|e| { error!("Failed to flush stdout after title screen: {}", e); e })?;
        info!("Title screen displayed. Waiting for key press.");

        let _ = io::stdin().bytes().next(); // Wait for key press
        info!("Key pressed. Starting game loop.");

        // Clear title screen
        let game_grid_dummy = GameGrid::new(terminal_width, terminal_height); // Dummy for clear_screen_manual
        game_grid_dummy.clear_screen_manual(&mut stdout_target, terminal_width, terminal_height).map_err(|e| { error!("Failed to clear screen manually after title screen: {}", e); e })?;
        stdout_target.flush().map_err(|e| { error!("Failed to flush stdout after clearing title screen: {}", e); e })?;
        std::thread::sleep(Duration::from_millis(100)); // Small delay before game loop starts
        info!("Title screen cleared.");
    }

    let mut ship = Ship::new(terminal_width / 2, terminal_height - 2);
    let mut asteroids: Vec<Asteroid> = Vec::new();
    let mut rng = rand::thread_rng();

    let _max_frames: Option<u64> = if !debug_mode_active && args.len() > 1 {
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

    let mut running = true;
    let mut frame_count = 0;
    let mut score = 0;
    let mut asteroid_spawn_rate = 10; // Start with 10 frames per asteroid

    let mut game_grid = GameGrid::new(terminal_width, terminal_height);

    while running && (_max_frames.is_none() || frame_count < _max_frames.unwrap()) {
        // Clear game grid
        game_grid.clear();

        // Update game state
        let mut current_key_event: Option<Event> = None;
        if debug_mode_active {
            if let Some(sim_input) = &mut simulated_input {
                if sim_input.poll(frame_count)? {
                    current_key_event = Some(sim_input.read()?);
                }
            }
        } else {
            if event::poll(Duration::from_millis(50)).map_err(|e| { error!("Failed to poll event: {}", e); e })? {
                current_key_event = Some(event::read().map_err(|e| { error!("Failed to read event: {}", e); e })?);
            }
        }

        if let Some(key_event) = current_key_event {
            if let Event::Key(key_event) = key_event {
                match key_event.code {
                    KeyCode::Char('q') => {
                        info!("Quit key 'q' pressed. Exiting game loop.");
                        running = false;
                    },
                    KeyCode::Left | KeyCode::Right => {
                        ship.update(key_event.code, terminal_width);
                        info!("Ship moved to x: {}", ship.x);
                    },
                    _ => {},
                }
            }
        }

        // Generate new asteroids
        if frame_count % asteroid_spawn_rate == 0 { // Generate a new asteroid every `asteroid_spawn_rate` frames
            let x = rng.gen_range(0..terminal_width);
            asteroids.push(Asteroid::new(x, 0, &mut rng));
            info!("New asteroid spawned at x: {}", x);
        }

        // Increase difficulty
        if frame_count % 500 == 0 && asteroid_spawn_rate > 1 { // Increase difficulty every 500 frames
            asteroid_spawn_rate -= 1;
            info!("Difficulty increased. New asteroid spawn rate: {}", asteroid_spawn_rate);
        }

        // Update and draw asteroids
        asteroids.retain_mut(|asteroid| {
            asteroid.update();
            // Collision detection
            // Collision detection based on geometric shapes
            let ship_coords = ship.get_absolute_coords();
            let asteroid_coords = asteroid.get_absolute_coords();

            let mut collision = false;
            for ship_point in &ship_coords {
                if asteroid_coords.contains(ship_point) {
                    collision = true;
                    break;
                }
            }

            if collision {
                info!("Collision detected. Game over.");
                running = false;
            }
            let on_screen = asteroid.y < terminal_height;
            if !on_screen {
                score += 1;
                info!("Asteroid went off screen. Score: {}", score);
            }
            on_screen // Keep asteroids that are still on screen
        });

        // Draw game state onto GameGrid
        ship.draw(&mut game_grid);
        for asteroid in &asteroids {
            asteroid.draw(&mut game_grid);
        }

        // Render GameGrid to stdout
        if !debug_mode_active {
            game_grid.render(&mut stdout_target).map_err(|e| { error!("Failed to render game grid: {}", e); e })?;
            stdout_target.flush().map_err(|e| { error!("Failed to flush stdout after rendering: {}", e); e })?;
        } else {
            if let OutputTarget::ScreenBuffer(ref mut sb) = stdout_target {
                // Copy GameGrid to ScreenBuffer for logging
                sb.clear();
                for y in 0..terminal_height {
                    for x in 0..terminal_width {
                        sb.set_char(x, y, game_grid.grid[y as usize][x as usize]);
                    }
                }
                sb.print_to_log();
            }
        }

        // Draw score and controls (always to stdout_target, which handles ScreenBuffer in debug mode)
        stdout_target.execute_move_to(MoveTo(0, 0)).map_err(|e| { error!("Failed to move cursor for score: {}", e); e })?;
        write!(stdout_target, "Score: {}", score).map_err(|e| { error!("Failed to write score: {}", e); e })?;
        stdout_target.flush().map_err(|e| { error!("Failed to flush stdout after score: {}", e); e })?;

        let controls_text = [
            "Controls:",
            r"  <- : Move Left",
            r"  -> : Move Right",
            r"  q  : Quit",
        ];
        let _controls_box_width = controls_text.iter().map(|s| s.len()).max().unwrap_or(0) as u16;
        let controls_box_height = controls_text.len() as u16;
        let controls_start_x = 0;
        let controls_start_y = terminal_height.saturating_sub(controls_box_height);

        for (i, line) in controls_text.iter().enumerate() {
            stdout_target.execute_move_to(MoveTo(controls_start_x, controls_start_y.saturating_add(i as u16))).map_err(|e| { error!("Failed to move cursor for controls: {}", e); e })?;
            write!(stdout_target, "{}", line).map_err(|e| { error!("Failed to write controls: {}", e); e })?;
            stdout_target.flush().map_err(|e| { error!("Failed to flush stdout after controls: {}", e); e })?;
        }

        stdout_target.flush().map_err(|e| { error!("Failed to flush stdout during game loop: {}", e); e })?;

        frame_count += 1;

        if debug_mode_active {
            if let OutputTarget::ScreenBuffer(sb) = &mut stdout_target {
                sb.print_to_log();
            }
        }
    }

    info!("Game loop ended. Displaying game over screen.");
    // Game Over screen
    game_grid.clear_screen_manual(&mut stdout_target, terminal_width, terminal_height).map_err(|e| { error!("Failed to clear screen manually for game over: {}", e); e })?;
    let game_over_msg = "GAME OVER!";
    let score_msg = format!("Final Score: {}", score);
    let exit_msg = "Press any key to exit...";

    let go_x = terminal_width / 2 - game_over_msg.len() as u16 / 2;
    let score_x = terminal_width / 2 - score_msg.len() as u16 / 2;
    let exit_x = terminal_width / 2 - exit_msg.len() as u16 / 2;

    let go_y = terminal_height / 2 - 2;
    let score_y = terminal_height / 2;
    let exit_y = terminal_height / 2 + 2;

    stdout_target.execute_move_to(MoveTo(go_x, go_y)).map_err(|e| { error!("Failed to move cursor for game over: {}", e); e })?;
    write!(stdout_target, "{}", game_over_msg).map_err(|e| { error!("Failed to write GAME OVER: {}", e); e })?;

    stdout_target.execute_move_to(MoveTo(score_x, score_y)).map_err(|e| { error!("Failed to move cursor for final score: {}", e); e })?;
    write!(stdout_target, "{}", score_msg).map_err(|e| { error!("Failed to write final score: {}", e); e })?;

    stdout_target.execute_move_to(MoveTo(exit_x, exit_y)).map_err(|e| { error!("Failed to move cursor for exit message: {}", e); e })?;
    write!(stdout_target, "{}", exit_msg).map_err(|e| { error!("Failed to write exit message: {}", e); e })?;
    stdout_target.flush().map_err(|e| { error!("Failed to flush stdout after game over: {}", e); e })?;
    info!("Game over screen displayed. Waiting for final key press.");

    // Wait for a key press to exit
    let _ = io::stdin().bytes().next();
    info!("Final key pressed. Exiting application.");

    
        if !debug_mode_active {
        stdout_target.execute_other_command(Show).map_err(|e| { error!("Failed to show cursor on exit: {}", e); e })?;
        disable_raw_mode().map_err(|e| { error!("Failed to disable raw mode on exit: {}", e); e })?;
    }

    Ok(())
}