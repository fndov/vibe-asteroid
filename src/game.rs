use std::io::{self, Read, Write};
use std::time::Duration;
use crossterm::{ 
    cursor::{MoveTo},
    event::{self, Event, KeyCode},
};
use rand::Rng;
use log::error;

use crate::constants::*;
use crate::types::Vector2D;
use crate::rendering::{GameGrid, Minimap, OutputTarget};
use crate::entities::{Asteroid, Bullet, Particle, Ship, AsteroidSize};
use crate::upgrades::{Upgrade, UpgradeBox, UpgradeType};
use crate::terminal_io::SimulatedInput;

pub struct Game {
    pub terminal_width: u16,
    pub terminal_height: u16,
    pub stdout_target: OutputTarget,
    simulated_input: Option<SimulatedInput>,
    debug_mode_active: bool,
    max_frames: Option<u64>,
}

impl Game {
    pub fn new(
        terminal_width: u16,
        terminal_height: u16,
        stdout_target: OutputTarget,
        simulated_input: Option<SimulatedInput>,
        debug_mode_active: bool,
        max_frames: Option<u64>,
    ) -> Self {
        Game {
            terminal_width,
            terminal_height,
            stdout_target,
            simulated_input,
            debug_mode_active,
            max_frames,
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        if !self.debug_mode_active {
            self.show_title_screen()?;
        }

        let mut ship = Ship::new(self.terminal_width as f64 / 2.0, self.terminal_height as f64 / 2.0);
        let mut asteroids: Vec<Asteroid> = Vec::new();
        let mut bullets: Vec<Bullet> = Vec::new();
        let mut particles: Vec<Particle> = Vec::new();
        let mut upgrade_boxes: Vec<UpgradeBox> = Vec::new();
        let mut upgrades: Vec<Upgrade> = Vec::new();
        let mut player_health = ship.max_health;
        let mut last_shot_frame = 0;
        let mut last_hit_frame = 0;
        let mut rng = rand::thread_rng();

        let mut running = true;
        let mut frame_count = 0;
        let mut score = 0;
        let mut asteroid_spawn_rate = INITIAL_ASTEROID_SPAWN_RATE;
        let mut max_asteroids = INITIAL_MAX_ASTEROIDS;
        let mut difficulty_increase_timer = 0;
        let mut game_speed_multiplier = INITIAL_GAME_SPEED_MULTIPLIER;

        let mut game_grid = GameGrid::new(self.terminal_width, self.terminal_height);
        let mut minimap = Minimap::new(20, 20, self.terminal_width);

        let mut current_banner: Option<(String, u64)> = None;

        while running && (self.max_frames.is_none() || frame_count < self.max_frames.unwrap()) {
            game_grid.clear();
            minimap.clear();

            self.handle_input(&mut running, &mut ship, &mut bullets, &mut particles, frame_count, &mut last_shot_frame)?;

            self.update_game_state(
                &mut ship,
                &mut asteroids,
                &mut bullets,
                &mut particles,
                &mut upgrade_boxes,
                &mut upgrades,
                &mut player_health,
                &mut last_hit_frame,
                &mut score,
                &mut asteroid_spawn_rate,
                &mut max_asteroids,
                &mut difficulty_increase_timer,
                &mut game_speed_multiplier,
                &mut running,
                &mut rng,
                frame_count,
                &mut current_banner,
            );

            // Draw game state onto GameGrid
            ship.draw(&mut game_grid);
            for asteroid in &asteroids {
                asteroid.draw(&mut game_grid);
            }
            for bullet in &bullets {
                bullet.draw(&mut game_grid);
            }
            for particle in &particles {
                particle.draw(&mut game_grid);
            }
            for upgrade_box in &upgrade_boxes {
                upgrade_box.draw(&mut game_grid);
            }
            for upgrade in &upgrades {
                upgrade.draw(&mut game_grid);
            }

            self.render(&game_grid, &minimap, score, player_health, ship.max_health, &current_banner)?;

            frame_count += 1;
        }

        self.show_game_over_screen(score)?; 
        Ok(())
    }

    fn handle_input(
        &mut self,
        running: &mut bool,
        ship: &mut Ship,
        bullets: &mut Vec<Bullet>,
        particles: &mut Vec<Particle>,
        frame_count: u64,
        last_shot_frame: &mut u64,
    ) -> io::Result<()> {
        let mut current_event: Option<Event> = None;
        if self.debug_mode_active {
            if let Some(sim_input) = &mut self.simulated_input {
                if sim_input.poll(frame_count)? {
                    current_event = Some(sim_input.read()?);
                }
            }
        } else {
            if event::poll(Duration::from_millis(50)).map_err(|e| { error!("Failed to poll event: {}", e); e })? {
                current_event = Some(event::read().map_err(|e| { error!("Failed to read event: {}", e); e })?);
            }
        }

        if let Some(event) = current_event {
            match event {
                Event::Key(key_event) => match key_event.code {
                    KeyCode::Char('q') => *running = false,
                    KeyCode::Up => {
                        ship.thrust();
                        let smoke_velocity = Vector2D::new(-ship.angle.cos() * 0.5, -ship.angle.sin() * 0.5);
                        particles.push(Particle::new(ship.position, smoke_velocity, 10, '.'));
                    }
                    KeyCode::Left => ship.rotate(-1.0),
                    KeyCode::Right => ship.rotate(1.0),
                    KeyCode::Char(' ') => {
                        if frame_count - *last_shot_frame >= BULLET_COOLDOWN {
                            let bullet_speed = BULLET_SPEED * ship.bullet_speed_multiplier;
                            let bullet_velocity = Vector2D::new(ship.angle.cos() * bullet_speed, ship.angle.sin() * bullet_speed);
                            bullets.push(Bullet::new(ship.position, bullet_velocity, ship.bullet_size_multiplier));
                            *last_shot_frame = frame_count;
                        }
                    }
                    _ => {}
                },
                Event::Resize(new_width, new_height) => {
                    self.terminal_width = new_width;
                    self.terminal_height = new_height;
                }
                _ => {} 
            }
        }
        Ok(())
    }

    fn update_game_state(
        &mut self,
        ship: &mut Ship,
        asteroids: &mut Vec<Asteroid>,
        bullets: &mut Vec<Bullet>,
        particles: &mut Vec<Particle>,
        upgrade_boxes: &mut Vec<UpgradeBox>,
        upgrades: &mut Vec<Upgrade>,
        player_health: &mut u32,
        last_hit_frame: &mut u64,
        score: &mut u32,
        asteroid_spawn_rate: &mut u64,
        max_asteroids: &mut usize,
        difficulty_increase_timer: &mut u64,
        game_speed_multiplier: &mut f64,
        running: &mut bool,
        rng: &mut impl Rng,
        frame_count: u64,
        current_banner: &mut Option<(String, u64)>,
    ) {
        ship.update(self.terminal_width, self.terminal_height);

        if asteroids.len() < *max_asteroids && frame_count % *asteroid_spawn_rate == 0 {
            let side = rng.gen_range(0..4);
            let (x, y) = match side {
                0 => (rng.gen_range(0.0..self.terminal_width as f64), 0.0),
                1 => (self.terminal_width as f64 - 1.0, rng.gen_range(0.0..self.terminal_height as f64)),
                2 => (rng.gen_range(0.0..self.terminal_width as f64), self.terminal_height as f64 - 1.0),
                _ => (0.0, rng.gen_range(0.0..self.terminal_height as f64)),
            };
            asteroids.push(Asteroid::new(x, y, rng, AsteroidSize::Large, *game_speed_multiplier));
        }

        if frame_count % UPGRADE_BOX_SPAWN_RATE == 0 {
            let x = rng.gen_range(0.0..self.terminal_width as f64);
            let y = rng.gen_range(0.0..self.terminal_height as f64);
            upgrade_boxes.push(UpgradeBox::new(x, y));
        }

        *difficulty_increase_timer += 1;
        if *difficulty_increase_timer >= DIFFICULTY_INCREASE_INTERVAL_FRAMES {
            *max_asteroids += 1;
            *asteroid_spawn_rate = (*asteroid_spawn_rate as f64 * ASTEROID_SPAWN_RATE_DECREASE_FACTOR).round() as u64;
            if *asteroid_spawn_rate < MIN_ASTEROID_SPAWN_RATE {
                *asteroid_spawn_rate = MIN_ASTEROID_SPAWN_RATE;
            }
            *game_speed_multiplier += GAME_SPEED_MULTIPLIER_INCREASE;
            *difficulty_increase_timer = 0;
        }

        asteroids.retain_mut(|asteroid| {
            asteroid.update(self.terminal_width, self.terminal_height);
            let ship_coords = ship.get_absolute_coords();
            let asteroid_coords = asteroid.get_absolute_coords();
            let mut collision = false;
            for ship_point in &ship_coords {
                if asteroid_coords.contains(ship_point) {
                    collision = true;
                    break;
                }
            }
            if collision && frame_count - *last_hit_frame > INVINCIBILITY_FRAMES {
                if ship.shield_count > 0 {
                    ship.shield_count -= 1;
                } else {
                    *player_health = player_health.saturating_sub(1);
                }
                *last_hit_frame = frame_count;
                if *player_health == 0 {
                    *running = false;
                }
            }
            true
        });

        bullets.retain_mut(|bullet| {
            bullet.update(self.terminal_width, self.terminal_height);
            let mut hit_asteroid = false;
            let mut new_asteroids_to_add: Vec<Asteroid> = Vec::new();
            let bullet_pos = (bullet.position.x.round() as u16, bullet.position.y.round() as u16);
            asteroids.retain_mut(|asteroid| {
                let asteroid_coords = asteroid.get_absolute_coords();
                if asteroid_coords.contains(&bullet_pos) {
                    hit_asteroid = true;
                    match asteroid.size {
                        AsteroidSize::Large => {
                            *score += SCORE_LARGE_ASTEROID;
                            let new_x = asteroid.position.x;
                            let new_y = asteroid.position.y;
                            new_asteroids_to_add.push(Asteroid::new(new_x, new_y, rng, AsteroidSize::Medium, *game_speed_multiplier));
                            new_asteroids_to_add.push(Asteroid::new(new_x, new_y, rng, AsteroidSize::Medium, *game_speed_multiplier));
                        }
                        AsteroidSize::Medium => {
                            *score += SCORE_MEDIUM_ASTEROID;
                            let new_x = asteroid.position.x;
                            let new_y = asteroid.position.y;
                            new_asteroids_to_add.push(Asteroid::new(new_x, new_y, rng, AsteroidSize::Small, *game_speed_multiplier));
                            new_asteroids_to_add.push(Asteroid::new(new_x, new_y, rng, AsteroidSize::Small, *game_speed_multiplier));
                        }
                        AsteroidSize::Small => {
                            *score += SCORE_SMALL_ASTEROID;
                        }
                    }
                    for _ in 0..5 {
                        let angle = rng.gen_range(0.0..2.0 * std::f64::consts::PI);
                        let speed = rng.gen_range(0.5..1.5);
                        let explosion_velocity = Vector2D::new(angle.cos() * speed, angle.sin() * speed);
                        particles.push(Particle::new(asteroid.position, explosion_velocity, 15, '#'));
                    }
                    false
                } else {
                    true
                }
            });
            asteroids.extend(new_asteroids_to_add);

            let mut hit_upgrade_box = false;
            upgrade_boxes.retain_mut(|upgrade_box| {
                let upgrade_box_coords = upgrade_box.get_absolute_coords();
                if upgrade_box_coords.contains(&bullet_pos) {
                    hit_upgrade_box = true;
                    upgrade_box.hits_remaining -= 1;
                    for _ in 0..3 {
                        let angle = rng.gen_range(0.0..2.0 * std::f64::consts::PI);
                        let speed = rng.gen_range(0.2..0.8);
                        let explosion_velocity = Vector2D::new(angle.cos() * speed, angle.sin() * speed);
                        particles.push(Particle::new(upgrade_box.position, explosion_velocity, 10, '+'));
                    }
                    if upgrade_box.hits_remaining == 0 {
                        let num_upgrades = rng.gen_range(1..=3);
                        for _ in 0..num_upgrades {
                            let upgrade_type = match rng.gen_range(0..8) {
                                0 => UpgradeType::FireRate,
                                1 => UpgradeType::BulletSpeed,
                                2 => UpgradeType::BulletSize,
                                3 => UpgradeType::Booster,
                                4 => UpgradeType::Shield,
                                5 => UpgradeType::ShipSize,
                                6 => UpgradeType::Health,
                                _ => UpgradeType::HealthMax,
                            };
                            upgrades.push(Upgrade::new(upgrade_box.position, upgrade_type));
                        }
                        false
                    } else {
                        true
                    }
                } else {
                    true
                }
            });

            bullet.lifetime > 0 && !hit_asteroid && !hit_upgrade_box
        });

        particles.retain_mut(|particle| {
            particle.update();
            particle.lifetime > 0
        });

        upgrades.retain_mut(|upgrade| {
            let distance = ((ship.position.x - upgrade.position.x).powi(2) + (ship.position.y - upgrade.position.y).powi(2)).sqrt();
            if distance <= UPGRADE_COLLECTION_RADIUS {
                match upgrade.upgrade_type {
                    UpgradeType::FireRate => {
                        ship.fire_rate_multiplier *= 1.1;
                        *current_banner = Some(("Fire Rate Increased!".to_string(), frame_count + 60));
                    }
                    UpgradeType::BulletSpeed => {
                        ship.bullet_speed_multiplier *= 1.1;
                        *current_banner = Some(("Bullet Speed Increased!".to_string(), frame_count + 60));
                    }
                    UpgradeType::BulletSize => {
                        ship.bullet_size_multiplier += 0.5;
                        *current_banner = Some(("Bullet Size Increased!".to_string(), frame_count + 60));
                    }
                    UpgradeType::Booster => {
                        ship.booster_multiplier *= 1.1;
                        *current_banner = Some(("Booster Power Increased!".to_string(), frame_count + 60));
                    }
                    UpgradeType::Shield => {
                        ship.shield_count += 1;
                        *current_banner = Some(("Shield Added!".to_string(), frame_count + 60));
                    }
                    UpgradeType::ShipSize => {
                        ship.ship_size_multiplier += 0.2;
                        ship.max_health += 1;
                        *player_health = (*player_health + 1).min(ship.max_health);
                        *current_banner = Some(("Ship Size Increased!".to_string(), frame_count + 60));
                    }
                    UpgradeType::Health => {
                        *player_health = (*player_health + 1).min(ship.max_health);
                        *current_banner = Some(("Health Restored!".to_string(), frame_count + 60));
                    }
                    UpgradeType::HealthMax => {
                        *player_health = ship.max_health;
                        *current_banner = Some(("Health Maxed!".to_string(), frame_count + 60));
                    }
                }
                false
            } else {
                true
            }
        });
    }

    fn render(
        &mut self,
        game_grid: &GameGrid,
        minimap: &Minimap,
        score: u32,
        player_health: u32,
        max_health: u32,
        current_banner: &Option<(String, u64)>,
    ) -> io::Result<()> {
        if !self.debug_mode_active {
            game_grid.render(&mut self.stdout_target)?;
        } else {
            if let OutputTarget::ScreenBuffer(ref mut sb) = self.stdout_target {
                sb.clear();
                for y in 0..self.terminal_height {
                    for x in 0..self.terminal_width {
                        sb.buffer[y as usize][x as usize] = game_grid.grid[y as usize][x as usize];
                    }
                }
                sb.print_to_log();
            }
        }

        minimap.render(&mut self.stdout_target)?;

        self.stdout_target.execute_move_to(MoveTo(0, 0))?;
        write!(self.stdout_target, "Score: {}  Health: {}/{}", score, player_health, max_health)?;

        let controls_text = [
            "Controls:",
            r"  Up Arrow : Thrust",
            r"  Left Arrow : Rotate Left",
            r"  Right Arrow: Rotate Right",
            r"  Spacebar : Fire Laser",
            r"  q        : Quit",
        ];
        let controls_box_height = controls_text.len() as u16;
        let controls_start_y = self.terminal_height.saturating_sub(controls_box_height);

        for (i, line) in controls_text.iter().enumerate() {
            self.stdout_target.execute_move_to(MoveTo(0, controls_start_y.saturating_add(i as u16)))?;
            write!(self.stdout_target, "{}", line)?;
        }

        if let Some((message, display_until_frame)) = current_banner {
            if self.max_frames.is_none() || *display_until_frame > 0 {
                let banner_x = self.terminal_width / 2 - message.len() as u16 / 2;
                let banner_y = self.terminal_height / 2 - 5;
                self.stdout_target.execute_move_to(MoveTo(banner_x, banner_y))?;
                write!(self.stdout_target, "{}", message)?;
            }
        }

        self.stdout_target.flush()?;
        Ok(())
    }

    fn show_title_screen(&mut self) -> io::Result<()> {
        let title_art = [
            r"VIBE-ASTEROID",
            r" _   _ _____ _____ ____  _____ ____  _   _ ____  _",
            r"| | | | ____|_   _|  _ \| ____|  _ \| | | |  _ \| |",
            r"| |_| |  _|   | | | |_) |  _| | |_) | |_| | |_) | |",
            r"|  _  | |___  | | |  _ <| |___|  _ <|  _  |  _ <| |",
            r"|_| |_|_____| |_| |_| \|_____|_| \|_| |_|_| \|_|",
        ];

        let title_start_y = self.terminal_height / 2 - title_art.len() as u16 / 2;
        for (i, line) in title_art.iter().enumerate() {
            let x = self.terminal_width / 2 - line.len() as u16 / 2;
            self.stdout_target.execute_move_to(MoveTo(x, title_start_y + i as u16))?;
            write!(self.stdout_target, "{}", line)?;
        }

        let press_any_key_msg = "Press any key to start...";
        let msg_x = self.terminal_width / 2 - press_any_key_msg.len() as u16 / 2;
        self.stdout_target.execute_move_to(MoveTo(msg_x, self.terminal_height - 5))?;
        write!(self.stdout_target, "{}", press_any_key_msg)?;
        self.stdout_target.flush()?;

        let _ = io::stdin().read(&mut [0u8]).unwrap();

        let game_grid_dummy = GameGrid::new(self.terminal_width, self.terminal_height);
        game_grid_dummy.clear_screen_manual(&mut self.stdout_target, self.terminal_width, self.terminal_height)?;
        self.stdout_target.flush()?;
        std::thread::sleep(Duration::from_millis(100));
        Ok(())
    }

    fn show_game_over_screen(&mut self, score: u32) -> io::Result<()> {
        let game_grid_dummy = GameGrid::new(self.terminal_width, self.terminal_height);
        game_grid_dummy.clear_screen_manual(&mut self.stdout_target, self.terminal_width, self.terminal_height)?;

        let game_over_msg = "GAME OVER!";
        let score_msg = format!("Final Score: {}", score);
        let exit_msg = "Press any key to exit...";

        let go_x = self.terminal_width / 2 - game_over_msg.len() as u16 / 2;
        let score_x = self.terminal_width / 2 - score_msg.len() as u16 / 2;
        let exit_x = self.terminal_width / 2 - exit_msg.len() as u16 / 2;

        let go_y = self.terminal_height / 2 - 2;
        let score_y = self.terminal_height / 2;
        let exit_y = self.terminal_height / 2 + 2;

        self.stdout_target.execute_move_to(MoveTo(go_x, go_y))?;
        write!(self.stdout_target, "{}", game_over_msg)?;

        self.stdout_target.execute_move_to(MoveTo(score_x, score_y))?;
        write!(self.stdout_target, "{}", score_msg)?;

        self.stdout_target.execute_move_to(MoveTo(exit_x, exit_y))?;
        write!(self.stdout_target, "{}", exit_msg)?;
        self.stdout_target.flush()?;

        let _ = io::stdin().read(&mut [0u8]).unwrap();
        Ok(())
    }
}

