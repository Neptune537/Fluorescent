use std::time::{Duration, Instant};

pub struct ThermalScaler {
    pub target_fps: u32,
    pub last_frame_time: Instant,
    pub is_static: bool,
}

impl ThermalScaler {
    pub fn new() -> Self {
        Self {
            target_fps: 60,
            last_frame_time: Instant::now(),
            is_static: false,
        }
    }

    pub fn update_state(&mut self, is_static: bool) {
        self.is_static = is_static;
        if self.is_static {
            self.target_fps = 15;
        } else {
            self.target_fps = 60;
        }
    }

    pub fn throttle(&mut self) {
        let target_duration = Duration::from_secs_f32(1.0 / self.target_fps as f32);
        let elapsed = self.last_frame_time.elapsed();

        if elapsed < target_duration {
            std::thread::sleep(target_duration - elapsed);
        }
        self.last_frame_time = Instant::now();
    }
}
