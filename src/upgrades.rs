use crate::types::Vector2D;
use crate::rendering::GameGrid;

#[derive(Debug)]
pub enum UpgradeType {
    // Beam Upgrades
    FireRate,
    BulletSpeed,
    BulletSize,
    // Ship Upgrades
    Booster,
    Shield,
    ShipSize,
    // Health Upgrades
    Health,
    HealthMax,
}

pub struct Upgrade {
    pub position: Vector2D,
    pub upgrade_type: UpgradeType,
    pub display_char: char,
}

impl Upgrade {
    pub fn new(position: Vector2D, upgrade_type: UpgradeType) -> Self {
        let display_char = match upgrade_type {
            UpgradeType::FireRate => 'B',
            UpgradeType::BulletSpeed => 'B',
            UpgradeType::BulletSize => 'B',
            UpgradeType::Booster => 'S',
            UpgradeType::Shield => 'S',
            UpgradeType::ShipSize => 'S',
            UpgradeType::Health => 'H',
            UpgradeType::HealthMax => 'H',
        };
        Upgrade { position, upgrade_type, display_char }
    }

    pub fn draw(&self, game_grid: &mut GameGrid) {
        game_grid.set_char(self.position.x.round() as u16, self.position.y.round() as u16, self.display_char);
    }
}

pub struct UpgradeBox {
    pub position: Vector2D,
    pub hits_remaining: u32,
    pub shape: Vec<(f64, f64)>,
    pub display_char: char,
}

impl UpgradeBox {
    pub fn new(x: f64, y: f64) -> Self {
        UpgradeBox {
            position: Vector2D::new(x, y),
            hits_remaining: 3, // Example health
            shape: vec![
                (-1.0, -1.0), (0.0, -1.0), (1.0, -1.0),
                (-1.0, 0.0), (0.0, 0.0), (1.0, 0.0),
                (-1.0, 1.0), (0.0, 1.0), (1.0, 1.0),
            ],
            display_char: 'U',
        }
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
}