//! Spring-based UI animation helpers.

/// Critically-damped spring toward a target value in 0..1 (or any range).
#[derive(Clone, Debug)]
pub struct Spring {
    pub current: f32,
    pub target: f32,
    pub speed: f32,
}

impl Spring {
    pub fn new(initial: f32) -> Self {
        Self {
            current: initial,
            target: initial,
            speed: 14.0,
        }
    }

    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    pub fn snap_to(&mut self, value: f32) {
        self.current = value;
        self.target = value;
    }

    /// Advance one frame; returns true if still animating.
    pub fn tick(&mut self, dt: f32) -> bool {
        let diff = self.target - self.current;
        if diff.abs() < 0.001 {
            self.current = self.target;
            return false;
        }
        let factor = 1.0 - (-self.speed * dt).exp();
        self.current += diff * factor;
        true
    }

    pub fn is_animating(&self) -> bool {
        (self.target - self.current).abs() >= 0.001
    }
}

pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spring_converges() {
        let mut s = Spring::new(0.0);
        s.set_target(1.0);
        for _ in 0..120 {
            s.tick(1.0 / 60.0);
        }
        assert!((s.current - 1.0).abs() < 0.02);
    }
}
