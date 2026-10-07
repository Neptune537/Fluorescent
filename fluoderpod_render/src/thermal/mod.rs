use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};

/// FPS tiers shared with Lite.
pub const FPS_INTERACTIVE: u32 = 60;
pub const FPS_STATIC: u32 = 24;
pub const FPS_PAUSED: u32 = 0;
/// Upper bound accepted by [`set_target_fps`].
pub const FPS_MAX: u32 = 240;

static TARGET_FPS: AtomicU32 = AtomicU32::new(FPS_INTERACTIVE);
static IDLE: AtomicBool = AtomicBool::new(false);

/// Set the host-requested target FPS (0 = paused). Clamped to [`FPS_MAX`]. Thread-safe.
pub fn set_target_fps(fps: u32) {
    TARGET_FPS.store(fps.min(FPS_MAX), Ordering::Relaxed);
}

/// Current host-requested target FPS (ignores the idle flag).
pub fn target_fps() -> u32 {
    TARGET_FPS.load(Ordering::Relaxed)
}

/// Mark the host idle; render loops should not draw while idle. Thread-safe.
pub fn set_idle(idle: bool) {
    IDLE.store(idle, Ordering::Relaxed);
}

pub fn is_idle() -> bool {
    IDLE.load(Ordering::Relaxed)
}

/// FPS the render loop should honor: 0 when idle, else [`target_fps`].
pub fn effective_fps() -> u32 {
    if is_idle() { FPS_PAUSED } else { target_fps() }
}

/// FFI: set target FPS (0 = paused).
#[no_mangle]
pub extern "C" fn fluoderpod_set_target_fps(fps: u32) {
    set_target_fps(fps);
}

/// FFI: set idle flag.
#[no_mangle]
pub extern "C" fn fluoderpod_set_idle(idle: bool) {
    set_idle(idle);
}

pub struct ThermalScaler {
    pub target_fps: u32,
    pub last_frame_time: Instant,
    pub is_static: bool,
}

impl ThermalScaler {
    pub fn new() -> Self {
        Self {
            target_fps: target_fps(),
            last_frame_time: Instant::now(),
            is_static: false,
        }
    }

    pub fn update_state(&mut self, is_static: bool) {
        self.is_static = is_static;
        self.target_fps = if is_static { FPS_STATIC } else { FPS_INTERACTIVE };
        set_target_fps(self.target_fps);
    }

    pub fn throttle(&mut self) {
        let fps = effective_fps();
        self.target_fps = fps;
        // Paused/idle: poll at a low rate rather than dividing by zero.
        let target_duration = if fps == 0 {
            Duration::from_millis(100)
        } else {
            Duration::from_secs_f32(1.0 / fps as f32)
        };
        let elapsed = self.last_frame_time.elapsed();

        if elapsed < target_duration {
            std::thread::sleep(target_duration - elapsed);
        }
        self.last_frame_time = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Globals are process-wide; keep all state assertions in one test.
    #[test]
    fn host_fps_and_idle() {
        assert_eq!(FPS_INTERACTIVE, 60);
        assert_eq!(FPS_STATIC, 24);
        set_target_fps(24);
        assert_eq!(target_fps(), 24);
        set_target_fps(0);
        assert_eq!(target_fps(), 0);
        set_target_fps(10_000);
        assert_eq!(target_fps(), FPS_MAX);
        set_target_fps(60);
        set_idle(true);
        assert!(is_idle());
        assert_eq!(effective_fps(), 0);
        fluoderpod_set_idle(false);
        assert!(!is_idle());
        assert_eq!(effective_fps(), 60);
        fluoderpod_set_target_fps(24);
        assert_eq!(target_fps(), 24);

        let mut s = ThermalScaler::new();
        s.update_state(true);
        assert_eq!(target_fps(), FPS_STATIC);
        s.update_state(false);
        assert_eq!(target_fps(), FPS_INTERACTIVE);

        set_idle(true);
        s.throttle(); // must not panic on zero fps
        set_idle(false);
        set_target_fps(60);
    }

    #[test]
    fn thread_safe() {
        let h = std::thread::spawn(|| {
            for _ in 0..100 {
                let _ = target_fps();
                let _ = is_idle();
            }
        });
        h.join().unwrap();
    }
}
