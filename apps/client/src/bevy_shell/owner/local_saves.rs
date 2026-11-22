use super::*;
use std::time::{Duration, Instant};

const SAVE_FLUSH_TIMEOUT: Duration = Duration::from_secs(3);

impl HostOwner {
    pub(super) fn retire_play(&mut self) {
        if let Some(play) = self.play.take() {
            let saves = play.view.minimap_saves();
            drop(play);
            if saves.poll() {
                self.retired_minimaps.push(saves);
            }
        }
    }

    pub(super) fn poll_retired_minimaps(&mut self) {
        self.retired_minimaps
            .retain(crate::play::MinimapSaves::poll);
    }

    pub(super) fn flush_local_saves(&mut self) {
        self.retire_play();
        let deadline = Instant::now() + SAVE_FLUSH_TIMEOUT;
        loop {
            self.poll_retired_minimaps();
            if let Some(error) = self.settings.poll_background_save().error {
                eprintln!("{error}");
            }
            if self.retired_minimaps.is_empty() && !self.settings.local_preference_writes_pending()
            {
                return;
            }
            if Instant::now() >= deadline {
                eprintln!("Timed out while saving local settings and minimap");
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

#[cfg(test)]
#[test]
fn review_3376_recovery_notice_reaches_the_player_once() -> Result<(), Box<dyn std::error::Error>> {
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("profile.json");
    let backup = path.with_added_extension("malformed");
    let mut owner = HostOwner::new(None)?;
    let _lock = crate::atomic_file::lock(&path)?;
    std::fs::write(&path, b"{first")?;
    crate::atomic_file::preserve_malformed(&path)?;
    owner.poll([640, 480]);
    assert!(owner.login.visible);
    assert!(
        owner
            .login
            .form
            .alert()
            .is_some_and(|(_, message)| message.contains("profile.json.malformed"))
    );
    owner.login.form.dismiss_alert();
    std::fs::write(&path, b"{second")?;
    crate::atomic_file::preserve_malformed(&path)?;
    owner.poll([640, 480]);
    assert!(owner.login.form.alert().is_none());
    assert_eq!(std::fs::read(backup)?, b"{first");
    Ok(())
}
