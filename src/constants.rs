// --- Game Constants ---
pub const INITIAL_ASTEROID_SPAWN_RATE: u64 = 100; // Frames per asteroid spawn
pub const INITIAL_MAX_ASTEROIDS: usize = 4;
pub const DIFFICULTY_INCREASE_INTERVAL_FRAMES: u64 = 60 * 60; // Every 60 seconds (assuming 60 FPS)
pub const ASTEROID_SPAWN_RATE_DECREASE_FACTOR: f64 = 0.9; // Decrease spawn rate by 10%
pub const MIN_ASTEROID_SPAWN_RATE: u64 = 10;
pub const INITIAL_GAME_SPEED_MULTIPLIER: f64 = 0.1;
pub const GAME_SPEED_MULTIPLIER_INCREASE: f64 = 0.05;

pub const SHIP_ROTATION_SPEED: f64 = 0.1;
pub const SHIP_THRUST_POWER: f64 = 0.05;
pub const SHIP_FRICTION: f64 = 0.98;
pub const SHIP_ANGULAR_FRICTION: f64 = 0.9;

pub const BULLET_SPEED: f64 = 2.0;
pub const BULLET_LIFETIME: u32 = 30; // Frames

pub const SCORE_LARGE_ASTEROID: u32 = 20;
pub const SCORE_MEDIUM_ASTEROID: u32 = 50;
pub const SCORE_SMALL_ASTEROID: u32 = 100;

pub const BULLET_COOLDOWN: u64 = 10; // Frames between shots
pub const MAX_HEALTH: u32 = 1;
pub const UPGRADE_COLLECTION_RADIUS: f64 = 2.0; // Ship can collect upgrade within this radius
pub const TERMINAL_ASPECT_RATIO_COMPENSATION: f64 = 2.0; // Adjust this based on terminal character aspect ratio (height/width)

pub const INVINCIBILITY_FRAMES: u64 = 60 * 2; // 2 seconds of invincibility

pub const UPGRADE_BOX_SPAWN_RATE: u64 = 60 * 10; // Every 10 seconds