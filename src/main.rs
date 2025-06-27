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
// --- Vector2D for physics calculations ---
#[derive(Clone, Copy, Debug, PartialEq)]
struct Vector2D {
    x: f64,
    y: f64,
}

impl Vector2D {
    fn new(x: f64, y: f64) -> Self {
        Vector2D { x, y }
    }

    fn magnitude(&self) -> f64 {
        (self.x.powi(2) + self.y.powi(2)).sqrt()
    }

    fn normalize(&self) -> Self {
        let mag = self.magnitude();
        if mag > 0.0 {
            Vector2D::new(self.x / mag, self.y / mag)
        } else {
            Vector2D::new(0.0, 0.0)
        }
    }

    fn scale(&self, scalar: f64) -> Self {
        Vector2D::new(self.x * scalar, self.y * scalar)
    }

    fn add(&self, other: Vector2D) -> Self {
        Vector2D::new(self.x + other.x, self.y + other.y)
    }

    fn subtract(&self, other: Vector2D) -> Self {
        Vector2D::new(self.x - other.x, self.y - other.y)
    }
}

fn wrap_coordinate(value: f64, max: f64) -> f64 {
    let wrapped = value % max;
    if wrapped < 0.0 {
        wrapped + max
    } else {
        wrapped
    }
}

// --- Ship and Asteroid structs (modified for geometric rendering) ---
struct Ship {
    position: Vector2D,
    velocity: Vector2D,
    angle: f64, // Radians
    rotation_speed: f64,
    thrust_power: f64,
    friction: f64,
    shape: Vec<(f64, f64)>, // Relative coordinates for diamond shape
    display_char: char,
}

impl Ship {
    fn new(x: f64, y: f64) -> Self {
        let shape = vec![
            (0.0, 0.0), // Center
            (-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0), // Diamond points
        ];
        Ship {
            position: Vector2D::new(x, y),
            velocity: Vector2D::new(0.0, 0.0),
            angle: 0.0, // Facing right initially
            rotation_speed: 0.1,
            thrust_power: 0.2,
            friction: 0.98, // Reduces velocity by 2% each frame
            shape,
            display_char: '#',
        }
    }

    fn get_absolute_coords(&self) -> Vec<(u16, u16)> {
        self.shape.iter().map(|&(dx, dy)| {
            // Rotate the relative coordinates
            let rotated_x = dx * self.angle.cos() - dy * self.angle.sin();
            let rotated_y = dx * self.angle.sin() + dy * self.angle.cos();

            // Translate to absolute position and convert to u16
            ((self.position.x + rotated_x).round() as u16, (self.position.y + rotated_y).round() as u16)
        }).collect()
    }

    fn draw(&self, game_grid: &mut GameGrid) {
        for &(dx, dy) in &self.shape {
            let rotated_x = dx * self.angle.cos() - dy * self.angle.sin();
            let rotated_y = dx * self.angle.sin() + dy * self.angle.cos();

            let draw_x = (self.position.x + rotated_x).round() as u16;
            let draw_y = (self.position.y + rotated_y).round() as u16;
            game_grid.set_char(draw_x, draw_y, self.display_char);
        }
    }

    fn update(&mut self, terminal_width: u16, terminal_height: u16) {
        self.position = self.position.add(self.velocity);

        // Screen wrapping
        self.position.x = wrap_coordinate(self.position.x, terminal_width as f64);
        self.position.y = wrap_coordinate(self.position.y, terminal_height as f64);
    }

    fn thrust(&mut self) {
        let thrust_vector = Vector2D::new(self.angle.cos(), self.angle.sin()).scale(self.thrust_power);
        self.velocity = self.velocity.add(thrust_vector);
    }

    fn rotate(&mut self, direction: f64) {
        self.angle += self.rotation_speed * direction;
    }
}

enum AsteroidSize {
    Large,
    Medium,
    Small,
}

struct Asteroid {
    position: Vector2D,
    velocity: Vector2D,
    size: AsteroidSize,
    shape: Vec<(f64, f64)>, // Relative coordinates for bumpy shape
    display_char: char,
}

impl Asteroid {
    fn new(x: f64, y: f64, rng: &mut impl Rng, size: AsteroidSize) -> Self {
        let (shape, display_char) = match size {
            AsteroidSize::Large => (
                vec![
                    (0.0, 0.0), (-2.0, -1.0), (-1.0, -2.0), (1.0, -2.0), (2.0, -1.0),
                    (2.0, 1.0), (1.0, 2.0), (-1.0, 2.0), (-2.0, 1.0),
                ],
                '@',
            ),
            AsteroidSize::Medium => (
                vec![
                    (0.0, 0.0), (-1.0, -1.0), (0.0, -1.0), (1.0, -1.0),
                    (-1.0, 0.0), (1.0, 0.0), (-1.0, 1.0), (0.0, 1.0), (1.0, 1.0),
                ],
                'O',
            ),
            AsteroidSize::Small => (vec![(0.0, 0.0)], 'o'),
        };
        let angle = rng.gen_range(0.0..2.0 * std::f64::consts::PI);
        let speed = match size {
            AsteroidSize::Large => rng.gen_range(0.3..0.8),
            AsteroidSize::Medium => rng.gen_range(0.8..1.5),
            AsteroidSize::Small => rng.gen_range(1.5..2.5),
        };
        let velocity = Vector2D::new(angle.cos() * speed, angle.sin() * speed);

        Asteroid { position: Vector2D::new(x, y), velocity, size, shape, display_char }
    }

    fn get_absolute_coords(&self) -> Vec<(u16, u16)> {
        self.shape.iter().map(|&(dx, dy)| {
            ((self.position.x + dx).round() as u16, (self.position.y + dy).round() as u16)
        }).collect()
    }

    fn draw(&self, game_grid: &mut GameGrid) {
        for &(dx, dy) in &self.shape {
            let draw_x = (self.position.x + dx).round() as u16;
            let draw_y = (self.position.y + dy).round() as u16;
            game_grid.set_char(draw_x, draw_y, self.display_char);
        }
    }

    fn update(&mut self, terminal_width: u16, terminal_height: u16) {
        self.position = self.position.add(self.velocity);

        // Screen wrapping
        self.position.x = wrap_coordinate(self.position.x, terminal_width as f64);
        self.position.y = wrap_coordinate(self.position.y, terminal_height as f64);
    }
}

// --- Bullet struct ---
struct Bullet {
    position: Vector2D,
    velocity: Vector2D,
    lifetime: u32,
    display_char: char,
}

impl Bullet {
    fn new(position: Vector2D, velocity: Vector2D) -> Self {
        Bullet {
            position,
            velocity,
            lifetime: 30, // Bullet lasts for 30 frames
            display_char: '*'
        }
    }

    fn draw(&self, game_grid: &mut GameGrid) {
        game_grid.set_char(self.position.x.round() as u16, self.position.y.round() as u16, self.display_char);
    }

    fn update(&mut self, terminal_width: u16, terminal_height: u16) {
        self.position = self.position.add(self.velocity);
        self.lifetime -= 1;

        // Screen wrapping
        self.position.x = wrap_coordinate(self.position.x, terminal_width as f64);
        self.position.y = wrap_coordinate(self.position.y, terminal_height as f64);
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

    let mut ship = Ship::new(terminal_width as f64 / 2.0, terminal_height as f64 / 2.0);
    let mut asteroids: Vec<Asteroid> = Vec::new();
    let mut bullets: Vec<Bullet> = Vec::new();
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
                    KeyCode::Up => {
                        ship.thrust();
                        info!("Ship thrusting.");
                    },
                    KeyCode::Left => {
                        ship.rotate(-1.0); // Rotate left
                        info!("Ship rotating left.");
                    },
                    KeyCode::Right => {
                        ship.rotate(1.0); // Rotate right
                        info!("Ship rotating right.");
                    },
                    KeyCode::Char(' ') => {
                        let bullet_speed = 2.0;
                        let bullet_velocity = Vector2D::new(ship.angle.cos() * bullet_speed, ship.angle.sin() * bullet_speed);
                        bullets.push(Bullet::new(ship.position, bullet_velocity));
                        info!("Bullet fired.");
                    },
                    _ => {},
                }
            }
        }

        // Update ship position
        ship.update(terminal_width, terminal_height);

        // Generate new asteroids (from edges)
        if frame_count % asteroid_spawn_rate == 0 {
            let side = rng.gen_range(0..4); // 0: top, 1: right, 2: bottom, 3: left
            let (x, y) = match side {
                0 => (rng.gen_range(0.0..terminal_width as f64), 0.0), // Top
                1 => (terminal_width as f64 - 1.0, rng.gen_range(0.0..terminal_height as f64)), // Right
                2 => (rng.gen_range(0.0..terminal_width as f64), terminal_height as f64 - 1.0), // Bottom
                _ => (0.0, rng.gen_range(0.0..terminal_height as f64)), // Left
            };
            asteroids.push(Asteroid::new(x, y, &mut rng, AsteroidSize::Large));
            info!("New asteroid spawned at x: {}, y: {}", x, y);
        }

        // Increase difficulty
        if frame_count % 500 == 0 && asteroid_spawn_rate > 1 {
            asteroid_spawn_rate -= 1;
            info!("Difficulty increased. New asteroid spawn rate: {}", asteroid_spawn_rate);
        }

        // Update asteroids and check for collisions with ship
        asteroids.retain_mut(|asteroid| {
            asteroid.update(terminal_width, terminal_height);

            // Collision detection (Ship-Asteroid)
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
                info!("Collision detected between ship and asteroid. Game over.");
                running = false;
            }
            true // Keep all asteroids for now, will handle removal on bullet collision later
        });

        // Update and draw bullets
        bullets.retain_mut(|bullet| {
            bullet.update(terminal_width, terminal_height);

            // Bullet-asteroid collision
            let mut hit_asteroid = false;
            let mut new_asteroids_to_add: Vec<Asteroid> = Vec::new();
            asteroids.retain_mut(|asteroid| {
                let asteroid_coords = asteroid.get_absolute_coords();
                let bullet_pos = (bullet.position.x.round() as u16, bullet.position.y.round() as u16);

                if asteroid_coords.contains(&bullet_pos) {
                    hit_asteroid = true;
                    match asteroid.size {
                        AsteroidSize::Large => {
                            score += 20;
                            let new_x = asteroid.position.x;
                            let new_y = asteroid.position.y;
                            new_asteroids_to_add.push(Asteroid::new(new_x, new_y, &mut rng, AsteroidSize::Medium));
                            new_asteroids_to_add.push(Asteroid::new(new_x, new_y, &mut rng, AsteroidSize::Medium));
                        },
                        AsteroidSize::Medium => {
                            score += 50;
                            let new_x = asteroid.position.x;
                            let new_y = asteroid.position.y;
                            new_asteroids_to_add.push(Asteroid::new(new_x, new_y, &mut rng, AsteroidSize::Small));
                            new_asteroids_to_add.push(Asteroid::new(new_x, new_y, &mut rng, AsteroidSize::Small));
                        },
                        AsteroidSize::Small => {
                            score += 100;
                        },
                    }
                    info!("Bullet hit asteroid. Score: {}", score);
                    false // Remove asteroid
                } else {
                    true // Keep asteroid
                }
            });
            asteroids.extend(new_asteroids_to_add);
            bullet.lifetime > 0 && !hit_asteroid // Keep bullet if still alive and hasn't hit an asteroid
        });

        // Draw game state onto GameGrid
        ship.draw(&mut game_grid);
        for asteroid in &asteroids {
            asteroid.draw(&mut game_grid);
        }
        for bullet in &bullets {
            bullet.draw(&mut game_grid);
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
            r"  Up Arrow : Thrust",
            r"  Left Arrow : Rotate Left",
            r"  Right Arrow: Rotate Right",
            r"  Spacebar : Fire Laser",
            r"  q        : Quit",
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