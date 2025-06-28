use crate::constants::*;
use crate::types::{Vector2D, wrap_coordinate};
use crate::rendering::GameGrid;
use rand::Rng;
use log::info;

// --- Ship and Asteroid structs (modified for geometric rendering) ---
pub struct Ship {
    pub position: Vector2D,
    pub velocity: Vector2D,
    pub angle: f64, // Radians
    pub rotation_speed: f64,
    pub thrust_power: f64,
    pub friction: f64,
    pub angular_velocity: f64,
    pub angular_friction: f64,
    pub shape: Vec<(f64, f64)>, // Relative coordinates for diamond shape
    pub fire_rate_multiplier: f64,
    pub bullet_speed_multiplier: f64,
    pub bullet_size_multiplier: f64,
    pub booster_multiplier: f64,
    pub shield_count: u32,
    pub ship_size_multiplier: f64,
    pub max_health: u32,
}

impl Ship {
    pub fn new(x: f64, y: f64) -> Self {
        Ship {
            position: Vector2D::new(x, y),
            velocity: Vector2D::new(0.0, 0.0),
            angle: -std::f64::consts::FRAC_PI_2, // Facing upwards initially
            rotation_speed: SHIP_ROTATION_SPEED, 
            thrust_power: SHIP_THRUST_POWER, 
            friction: SHIP_FRICTION,
            angular_velocity: 0.0,
            angular_friction: SHIP_ANGULAR_FRICTION,
            shape: vec![
                (0.0, -1.0), // Top point
                (-1.0, 0.0), (1.0, 0.0), // Base points
            ],
            fire_rate_multiplier: 1.0,
            bullet_speed_multiplier: 1.0,
            bullet_size_multiplier: 1.0,
            booster_multiplier: 1.0,
            shield_count: 0,
            ship_size_multiplier: 1.0,
            max_health: MAX_HEALTH,
        }
    }

    pub fn get_scaled_shape(&self) -> Vec<(f64, f64)> {
        self.shape.iter().map(|&(dx, dy)| {
            (dx * self.ship_size_multiplier, dy * self.ship_size_multiplier)
        }).collect()
    }

    pub fn get_absolute_coords(&self) -> Vec<(u16, u16)> {
        self.get_scaled_shape().iter().map(|&(dx, dy)| {
            // Rotate the relative coordinates
            let rotated_x = dx * self.angle.cos() - dy * self.angle.sin();
            let rotated_y = dx * self.angle.sin() + dy * self.angle.cos();

            // Translate to absolute position and convert to u16
            ((self.position.x + rotated_x).round() as u16, (self.position.y + rotated_y).round() as u16)
        }).collect()
    }

    pub fn draw(&self, game_grid: &mut GameGrid) {
        let draw_angle = self.angle + std::f64::consts::FRAC_PI_2;
        for &(dx, dy) in &self.get_scaled_shape() {
            let rotated_x = dx * draw_angle.cos() - dy * draw_angle.sin();
            let rotated_y = dx * draw_angle.sin() + dy * draw_angle.cos();

            let draw_x = (self.position.x + rotated_x).round() as u16;
            let draw_y = (self.position.y + rotated_y).round() as u16;

            let char_to_draw = Ship::get_rotated_char(dx, dy, self.angle);
            game_grid.set_char(draw_x, draw_y, char_to_draw);
        }

        // Draw aiming indicator
        let aiming_distance = 3.0;
        let aim_x = (self.position.x + self.angle.cos() * aiming_distance * TERMINAL_ASPECT_RATIO_COMPENSATION).round() as u16;
        let aim_y = (self.position.y + self.angle.sin() * aiming_distance).round() as u16;
        game_grid.set_char(aim_x, aim_y, '●');

        // Draw shield
        if self.shield_count > 0 {
            let shield_char = '#';
            // For simplicity, let's draw the shield behind the ship for now
            // We can make this more sophisticated later to cover a specific side
            let shield_x = (self.position.x - self.angle.cos() * 2.0).round() as u16;
            let shield_y = (self.position.y - self.angle.sin() * 2.0).round() as u16;
            game_grid.set_char(shield_x, shield_y, shield_char);
        }
    }

    pub fn update(&mut self, terminal_width: u16, terminal_height: u16) {
        self.position = self.position.add(self.velocity);
        self.velocity = self.velocity.scale(self.friction);

        self.angle += self.angular_velocity;
        self.angular_velocity *= self.angular_friction;

        // Screen wrapping
        self.position.x = wrap_coordinate(self.position.x, terminal_width as f64);
        self.position.y = wrap_coordinate(self.position.y, terminal_height as f64);
    }

    pub fn thrust(&mut self) {
        let thrust_vector = Vector2D::new(self.angle.cos(), self.angle.sin()).scale(self.thrust_power * self.booster_multiplier);
        self.velocity = self.velocity.add(thrust_vector);
        info!("Thrusting: Angle = {}, Thrust Vector = ({}, {})", self.angle, thrust_vector.x, thrust_vector.y);
    }

    pub fn rotate(&mut self, direction: f64) {
        self.angular_velocity += self.rotation_speed * direction;
    }

    pub fn get_rotated_char(original_dx: f64, original_dy: f64, angle: f64) -> char {
        // Normalize angle to be between 0 and 2*PI
        let normalized_angle = angle.rem_euclid(2.0 * std::f64::consts::PI);

        // Determine the primary direction based on 8 octants
        // 0 = East (right)
        // PI/2 = South (down)
        // PI = West (left)
        // 3*PI/2 = North (up)

        match (original_dx.round() as i8, original_dy.round() as i8) {
            (0, -1) => { // Top point
                if normalized_angle >= 7.0 * std::f64::consts::FRAC_PI_4 || normalized_angle < std::f64::consts::FRAC_PI_4 {
                    '>' // Pointing right
                } else if normalized_angle >= std::f64::consts::FRAC_PI_4 && normalized_angle < 3.0 * std::f64::consts::FRAC_PI_4 {
                    'v' // Pointing down
                } else if normalized_angle >= 3.0 * std::f64::consts::FRAC_PI_4 && normalized_angle < 5.0 * std::f64::consts::FRAC_PI_4 {
                    '<' // Pointing left
                } else { // 5*PI/4 to 7*PI/4
                    '^' // Pointing up
                }
            },
            (-1, 0) => { // Left base point
                if normalized_angle >= 7.0 * std::f64::consts::FRAC_PI_4 || normalized_angle < std::f64::consts::FRAC_PI_4 {
                    '\u{005C}' // Right-pointing ship, this is bottom-left
                } else if normalized_angle >= std::f64::consts::FRAC_PI_4 && normalized_angle < 3.0 * std::f64::consts::FRAC_PI_4 {
                    '/' // Down-pointing ship, this is top-left
                } else if normalized_angle >= 3.0 * std::f64::consts::FRAC_PI_4 && normalized_angle < 5.0 * std::f64::consts::FRAC_PI_4 {
                    '\u{005C}' // Left-pointing ship, this is top-right
                } else { // 5*PI/4 to 7*PI/4
                    '/' // Up-pointing ship, this is bottom-left
                }
            },
            (1, 0) => { // Right base point
                if normalized_angle >= 7.0 * std::f64::consts::FRAC_PI_4 || normalized_angle < std::f64::consts::FRAC_PI_4 {
                    '/' // Right-pointing ship, this is bottom-right
                } else if normalized_angle >= std::f64::consts::FRAC_PI_4 && normalized_angle < 3.0 * std::f64::consts::FRAC_PI_4 {
                    '\u{005C}' // Down-pointing ship, this is top-right
                } else if normalized_angle >= 3.0 * std::f64::consts::FRAC_PI_4 && normalized_angle < 5.0 * std::f64::consts::FRAC_PI_4 {
                    '/' // Left-pointing ship, this is top-right
                } else { // 5*PI/4 to 7*PI/4
                    '\u{005C}' // Up-pointing ship, this is bottom-right
                }
            },
            _ => ' ', // Should not happen for a triangle
        }
    }
} 

pub enum AsteroidSize {
    Large,
    Medium,
    Small,
}

pub struct Asteroid {
    pub position: Vector2D,
    pub velocity: Vector2D,
    pub size: AsteroidSize,
    pub shape: Vec<(f64, f64)>, // Relative coordinates for bumpy shape
    pub display_char: char,
}

impl Asteroid {
    pub fn new(x: f64, y: f64, rng: &mut impl Rng, size: AsteroidSize, game_speed_multiplier: f64) -> Self {
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
            AsteroidSize::Small => (vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)], 'o'),
        };
        let angle = rng.gen_range(0.0..2.0 * std::f64::consts::PI);
        let speed = match size {
            AsteroidSize::Large => rng.gen_range(0.3..0.8),
            AsteroidSize::Medium => rng.gen_range(0.8..1.5),
            AsteroidSize::Small => rng.gen_range(1.5..2.5),
        } * game_speed_multiplier;
        let velocity = Vector2D::new(angle.cos() * speed, angle.sin() * speed);

        Asteroid { position: Vector2D::new(x, y), velocity, size, shape, display_char }
    }

    pub fn get_absolute_coords(&self) -> Vec<(u16, u16)> {
        self.shape.iter().map(|&(dx, dy)| {
            ((self.position.x + dx).round() as u16, (self.position.y + dy).round() as u16)
        }).collect()
    }

    pub fn draw(&self, game_grid: &mut GameGrid) {
        for &(dx, dy) in &self.shape {
            let draw_x = (self.position.x + dx).round() as u16;
            let draw_y = (self.position.y + dy).round() as u16;
            game_grid.set_char(draw_x, draw_y, self.display_char);
        }
    }

    pub fn update(&mut self, terminal_width: u16, terminal_height: u16) {
        self.position = self.position.add(self.velocity);

        // Screen wrapping
        self.position.x = wrap_coordinate(self.position.x, terminal_width as f64);
        self.position.y = wrap_coordinate(self.position.y, terminal_height as f64);
    }
}

// --- Bullet struct ---
pub struct Bullet {
    pub position: Vector2D,
    pub velocity: Vector2D,
    pub lifetime: u32,
    pub display_char: char,
    pub size: f64,
}

impl Bullet {
    pub fn new(position: Vector2D, velocity: Vector2D, size: f64) -> Self {
        Bullet {
            position,
            velocity,
            lifetime: BULLET_LIFETIME, // Bullet lasts for 30 frames
            display_char: '*',
            size,
        }
    }

    pub fn draw(&self, game_grid: &mut GameGrid) {
        let char_to_draw = match self.lifetime {
            20..=30 => '*',
            10..=19 => '+',
            _ => '.', 
        };
        for i in 0..(self.size.round() as u16) {
            for j in 0..(self.size.round() as u16) {
                game_grid.set_char(self.position.x.round() as u16 + i, self.position.y.round() as u16 + j, char_to_draw);
            }
        }
    }

    pub fn update(&mut self, terminal_width: u16, terminal_height: u16) {
        self.position = self.position.add(self.velocity);
        self.lifetime -= 1;

        // Screen wrapping
        self.position.x = wrap_coordinate(self.position.x, terminal_width as f64);
        self.position.y = wrap_coordinate(self.position.y, terminal_height as f64);
    }
}

pub struct Particle {
    pub position: Vector2D,
    pub velocity: Vector2D,
    pub lifetime: u32,
    pub display_char: char,
}

impl Particle {
    pub fn new(position: Vector2D, velocity: Vector2D, lifetime: u32, display_char: char) -> Self {
        Particle {
            position,
            velocity,
            lifetime,
            display_char,
        }
    }

    pub fn draw(&self, game_grid: &mut GameGrid) {
        game_grid.set_char(self.position.x.round() as u16, self.position.y.round() as u16, self.display_char);
    }

    pub fn update(&mut self) {
        self.position = self.position.add(self.velocity);
        self.lifetime -= 1;
    }
}