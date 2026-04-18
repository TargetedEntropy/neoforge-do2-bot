use azalea::prelude::Client;
use azalea::{Vec3, WalkDirection};

use crate::config::MovementConfig;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MovementMode {
    Wander,
}

impl MovementMode {
    pub fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "wander" => Some(Self::Wander),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wander => "wander",
        }
    }
}

pub struct AutonomousMovement {
    enabled: bool,
    mode: MovementMode,
    cfg: MovementConfig,
    rng_state: u64,
    step_ticks_left: u32,
    idle_ticks_left: u32,
    jump_cooldown_ticks_left: u32,
    stuck_ticks: u32,
    walk_direction: WalkDirection,
    last_position: Option<Vec3>,
}

impl AutonomousMovement {
    pub fn new(cfg: MovementConfig) -> Self {
        Self {
            enabled: cfg.enabled,
            mode: cfg.mode,
            cfg,
            rng_state: 0x5eed_cafe_fade_beef,
            step_ticks_left: 0,
            idle_ticks_left: 0,
            jump_cooldown_ticks_left: 0,
            stuck_ticks: 0,
            walk_direction: WalkDirection::None,
            last_position: None,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.step_ticks_left = 0;
            self.idle_ticks_left = 0;
            self.stuck_ticks = 0;
            self.walk_direction = WalkDirection::None;
        }
    }

    pub fn set_mode(&mut self, mode: MovementMode) {
        self.mode = mode;
        self.step_ticks_left = 0;
        self.idle_ticks_left = 0;
    }

    pub fn tick(&mut self, bot: &mut Client) {
        if self.jump_cooldown_ticks_left > 0 {
            self.jump_cooldown_ticks_left -= 1;
        }

        if !self.enabled {
            bot.walk(WalkDirection::None);
            bot.set_jumping(false);
            return;
        }

        match self.mode {
            MovementMode::Wander => self.tick_wander(bot),
        }
    }

    fn tick_wander(&mut self, bot: &mut Client) {
        let mut should_jump = false;

        if self.step_ticks_left > 0 {
            self.step_ticks_left -= 1;
            bot.walk(self.walk_direction);

            if self.is_stuck(bot.position()) {
                self.stuck_ticks += 1;
                if self.stuck_ticks >= self.cfg.unstuck_ticks && self.jump_cooldown_ticks_left == 0
                {
                    should_jump = true;
                    self.stuck_ticks = 0;
                    self.jump_cooldown_ticks_left = self.cfg.jump_cooldown_ticks;
                }
            } else {
                self.stuck_ticks = 0;
            }

            if self.step_ticks_left == 0 {
                self.idle_ticks_left =
                    self.rand_range(self.cfg.min_idle_ticks, self.cfg.max_idle_ticks);
                bot.walk(WalkDirection::None);
            }
        } else if self.idle_ticks_left > 0 {
            self.idle_ticks_left -= 1;
            self.stuck_ticks = 0;
            bot.walk(WalkDirection::None);
        } else {
            self.choose_next_step(bot);
            bot.walk(self.walk_direction);
            self.step_ticks_left =
                self.rand_range(self.cfg.min_step_ticks, self.cfg.max_step_ticks);
        }

        bot.set_jumping(should_jump);
    }

    fn choose_next_step(&mut self, bot: &mut Client) {
        let (yaw, _pitch) = bot.direction();
        let turn = self.rand_range_f32(-self.cfg.turn_degrees, self.cfg.turn_degrees);
        bot.set_direction(yaw + turn, 0.0);

        self.walk_direction = match self.rand_u32() % 3 {
            0 => WalkDirection::Forward,
            1 => WalkDirection::ForwardLeft,
            _ => WalkDirection::ForwardRight,
        };
    }

    fn is_stuck(&mut self, current: Vec3) -> bool {
        let stuck = if let Some(last) = self.last_position {
            let dx = current.x - last.x;
            let dz = current.z - last.z;
            (dx * dx + dz * dz) < 0.0004
        } else {
            false
        };
        self.last_position = Some(current);
        stuck
    }

    fn rand_u32(&mut self) -> u32 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        (self.rng_state >> 16) as u32
    }

    fn rand_range(&mut self, min: u32, max: u32) -> u32 {
        if min >= max {
            return min;
        }
        min + (self.rand_u32() % (max - min + 1))
    }

    fn rand_range_f32(&mut self, min: f32, max: f32) -> f32 {
        let unit = (self.rand_u32() as f32) / (u32::MAX as f32);
        min + (max - min) * unit
    }
}
