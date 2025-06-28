#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vector2D {
    pub x: f64,
    pub y: f64,
}

impl Vector2D {
    pub fn new(x: f64, y: f64) -> Self {
        Vector2D { x, y }
    }

    pub fn scale(&self, scalar: f64) -> Self {
        Vector2D::new(self.x * scalar, self.y * scalar)
    }

    pub fn add(&self, other: Vector2D) -> Self {
        Vector2D::new(self.x + other.x, self.y + other.y)
    }
}

pub fn wrap_coordinate(value: f64, max: f64) -> f64 {
    let wrapped = value % max;
    if wrapped < 0.0 {
        wrapped + max
    } else {
        wrapped
    }
}