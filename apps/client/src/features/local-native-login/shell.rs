//! Native login lifecycle adapter for the Windows shell.
use super::*;

impl Application {
    pub(super) fn poll_entry(&mut self) {
        if let Some(config) = self.login.as_ref().and_then(LoginWindow::poll) {
            let started = self
                .entry
                .as_mut()
                .is_some_and(|entry| entry.configure(config) && entry.request());
            if !started && let Some(login) = &self.login {
                login.finish(Err(GameplayEntryError::Rejected(
                    PublicClass::SessionUnavailable,
                )));
            }
        }
        let Some(result) = self.entry.as_mut().and_then(Reentry::poll) else {
            return;
        };
        if let Some(login) = &self.login {
            login.finish(result.as_ref().map(|_| ()).map_err(|error| *error));
        }
        match result {
            Ok((mut view, link)) => {
                let center = view.map_center();
                let (world, reason) = World::from_env(center);
                if let Some(reason) = reason {
                    println!("Oteryn: drawing without map sprites: {reason}");
                }
                let shown = world.map(Arc::new).and_then(|world| {
                    view.set_world(Arc::clone(&world))?;
                    self.world = Some(world);
                    Ok(())
                });
                if shown.is_err() {
                    self.return_to_login(PublicClass::SessionUnavailable);
                    return;
                }
                self.scene = None;
                self.play = Some(Play {
                    view,
                    link,
                    status: String::new(),
                });
            }
            Err(error) => self.leave_play(error_text(error)),
        }
    }
}
