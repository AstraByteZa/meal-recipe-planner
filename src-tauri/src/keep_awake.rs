use std::sync::Mutex;

use crate::error::AppError;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
type Inhibitor = keepawake::KeepAwake;

/// Phones and tablets have no inhibitor wired up, so cook mode leaves the screen alone there.
#[cfg(any(target_os = "android", target_os = "ios"))]
type Inhibitor = ();

/// Holds the OS inhibitor that keeps the screen on while cook mode is open. Dropping the inhibitor releases it.
#[derive(Default)]
pub struct KeepAwakeLock(Mutex<Option<Inhibitor>>);

impl KeepAwakeLock {
    /// Turns the inhibitor on or off. Turning it on while it is already on keeps the existing one.
    pub fn set(&self, enabled: bool) -> Result<(), AppError> {
        let mut held = self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if !enabled {
            *held = None;
            return Ok(());
        }
        if held.is_some() {
            return Ok(());
        }
        *held = Some(acquire()?);
        Ok(())
    }

    #[cfg(test)]
    fn is_held(&self) -> bool {
        self.0.lock().unwrap().is_some()
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn acquire() -> Result<Inhibitor, AppError> {
    keepawake::Builder::default()
        .display(true)
        .reason("Cooking a recipe")
        .app_name("Mise")
        .app_reverse_domain("com.roelof.planner-app")
        .create()
        .map_err(|error| AppError::Platform(format!("Could not keep the screen awake: {error}")))
}

#[cfg(any(target_os = "android", target_os = "ios"))]
fn acquire() -> Result<Inhibitor, AppError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turning_off_when_not_held_is_a_no_op() {
        let lock = KeepAwakeLock::default();
        lock.set(false).unwrap();
        assert!(!lock.is_held());
    }

    // Needs a session bus and an inhibitor service, so it is ignored in CI. Run it on a desktop with
    // `cargo test keep_awake -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn holds_and_releases_the_inhibitor() {
        let lock = KeepAwakeLock::default();
        lock.set(true).unwrap();
        assert!(lock.is_held());
        lock.set(true).unwrap();
        assert!(lock.is_held());
        std::thread::sleep(std::time::Duration::from_secs(10));
        lock.set(false).unwrap();
        assert!(!lock.is_held());
    }
}
