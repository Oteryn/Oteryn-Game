//! Ordinary player captures use Bevy's rendered window and retained screenshot preferences.
use super::{game_ui::viewport::WorldViewport, owner::HostOwner};
use crate::play::screenshot::Pixels;
use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, ScreenshotCaptured},
};
use std::{io, path::PathBuf};
mod automatic;
pub(super) use automatic::{Capture, State as AutomaticState, observe};

pub(super) fn request(commands: &mut Commands, owner: &mut HostOwner, viewport: WorldViewport) {
    let preferences = owner.settings.preferences.screenshots.effective();
    let folder = if preferences.directory.is_empty() {
        crate::play::PreferenceStore::path()
            .and_then(|path| path.parent().map(|parent| parent.join("screenshots")))
            .unwrap_or_default()
    } else {
        PathBuf::from(&preferences.directory)
    };
    let crop = preferences.game_only.then_some(viewport);
    owner.status = "Capturing screenshot…".into();
    commands.spawn(Screenshot::primary_window()).observe(
        move |event: On<ScreenshotCaptured>, mut owner: NonSendMut<HostOwner>| {
            let result = save(&event.image, &folder, crop);
            owner.status = match result {
                Ok(path) => format!("Screenshot saved: {}", path.display()),
                Err(error) => format!("Screenshot failed: {error}"),
            };
        },
    );
}
fn save(
    image: &Image,
    folder: &std::path::Path,
    crop: Option<WorldViewport>,
) -> io::Result<PathBuf> {
    let mut image = image.clone().try_into_dynamic().map_err(io::Error::other)?;
    if let Some(crop) = crop {
        let end = crop
            .origin
            .checked_add(crop.size)
            .ok_or_else(|| io::Error::other("Invalid game viewport"))?;
        if crop.size.min_element() == 0 || end.x > image.width() || end.y > image.height() {
            return Err(io::Error::other("Game viewport is unavailable"));
        }
        image = image.crop_imm(crop.origin.x, crop.origin.y, crop.size.x, crop.size.y);
    }
    let rgba = image.to_rgba8();
    Pixels::new(rgba.width(), rgba.height(), rgba.into_raw())?.save(
        folder
            .to_str()
            .ok_or_else(|| io::Error::other("Invalid screenshot folder"))?,
    )
}

#[cfg(test)]
mod tests;
