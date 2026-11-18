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
    let crop = preferences.game_only.then_some(viewport);
    owner.status = "Capturing screenshot…".into();
    commands.spawn(Screenshot::primary_window()).observe(
        move |event: On<ScreenshotCaptured>, mut owner: NonSendMut<HostOwner>| {
            let result = save(&event.image, &preferences.directory, crop);
            owner.status = match result {
                Ok(path) => format!("Screenshot saved: {}", path.display()),
                Err(error) => format!("Screenshot failed: {error}"),
            };
        },
    );
}
fn save(image: &Image, folder: &str, crop: Option<WorldViewport>) -> io::Result<PathBuf> {
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
    Pixels::new(rgba.width(), rgba.height(), rgba.into_raw())?.save(folder)
}

/// Audit mode keeps its fixed PNG name, replacing it only after encoding completes.
pub(super) fn request_audit(commands: &mut Commands, path: PathBuf) {
    commands
        .spawn(Screenshot::primary_window())
        .observe(move |event: On<ScreenshotCaptured>| {
            if let Err(error) = save_audit(&event.image, &path) {
                bevy::log::error!("Could not save audit screenshot: {error}");
            }
        });
}

fn save_audit(image: &Image, path: &std::path::Path) -> io::Result<()> {
    #[cfg(not(test))]
    use crate::play::screenshot::create_capture_file as open_audit_capture;
    use crate::play::screenshot::retry_capture_name;
    use std::sync::atomic::{AtomicU64, Ordering};
    #[cfg(test)]
    use tests::open_audit_capture;

    static NEXT: AtomicU64 = AtomicU64::new(0);
    let image = image.clone().try_into_dynamic().map_err(io::Error::other)?;
    let format = bevy::image::ImageFormat::Png
        .as_image_crate_format()
        .ok_or_else(|| io::Error::other("PNG support is unavailable"))?;
    let (temporary, mut file) = retry_capture_name(|| {
        let candidate = path.with_file_name(format!(
            ".oteryn-audit-{}-{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        let file = open_audit_capture(&candidate)?;
        Ok((candidate, file))
    })?;
    let result = image
        .into_rgb8()
        .write_to(&mut file, format)
        .map_err(io::Error::other);
    drop(file);
    let result = result.and_then(|()| std::fs::rename(&temporary, path));
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}

#[cfg(test)]
mod tests;
