//! Ordinary player captures use Bevy's rendered window and retained screenshot preferences.
use super::{game_ui::viewport::WorldViewport, owner::HostOwner};
use crate::play::screenshot::Pixels;
use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, ScreenshotCaptured},
};
use bevy_tasks::{IoTaskPool, Task, futures::check_ready};
use std::{collections::VecDeque, io, path::PathBuf};
mod automatic;
pub(super) use automatic::{State as AutomaticState, observe};

type Job = Box<dyn FnOnce() -> Completion + Send>;
const MAX_QUEUED_CAPTURES: usize = 4;

#[derive(Default)]
pub(super) struct Writes {
    queued: VecDeque<Job>,
    active: Option<Task<Completion>>,
    automatic: bool,
}

enum Completion {
    Manual(u64, io::Result<PathBuf>),
    Audit(io::Result<()>),
    Automatic(automatic::Completed),
}

impl Writes {
    pub(super) fn is_full(&self) -> bool {
        self.queued.len() >= MAX_QUEUED_CAPTURES
    }

    // Serialize completed readbacks so a later audit PNG cannot be overwritten by an earlier one.
    fn push(&mut self, job: Job) -> bool {
        if self.is_full() {
            return false;
        }
        self.queued.push_back(job);
        true
    }

    pub(super) fn poll(&mut self, owner: &mut HostOwner) {
        if let Some(task) = &mut self.active {
            let Some(result) = check_ready(task) else {
                return;
            };
            self.active = None;
            match result {
                Completion::Manual(generation, result) if generation == owner.generation() => {
                    owner.status = match result {
                        Ok(path) => format!("Screenshot saved: {}", path.display()),
                        Err(error) => format!("Screenshot failed: {error}"),
                    }
                }
                Completion::Audit(Err(error)) => {
                    bevy::log::error!("Could not save audit screenshot: {error}")
                }
                Completion::Manual(_, _) | Completion::Audit(Ok(())) => {}
                Completion::Automatic(result) => {
                    self.automatic = false;
                    result.apply(owner);
                }
            }
        }
        if let Some(job) = self.queued.pop_front() {
            self.active = Some(IoTaskPool::get().spawn(async move { job() }));
        }
    }
}

pub(super) struct PlayerCapture {
    pub(super) directory: String,
    pub(super) game_only: bool,
    pub(super) backlog: bool,
}

pub(super) fn request(
    commands: &mut Commands,
    readback: Option<Entity>,
    owner: &mut HostOwner,
    generation: u64,
    request: PlayerCapture,
    viewport: WorldViewport,
) {
    let PlayerCapture {
        directory,
        game_only,
        ..
    } = request;
    let crop = game_only.then_some(viewport);
    owner.status = "Capturing screenshot…".into();
    let entity = readback.unwrap_or_else(|| commands.spawn(Screenshot::primary_window()).id());
    commands.entity(entity).observe(
        move |event: On<ScreenshotCaptured>,
              mut owner: NonSendMut<HostOwner>,
              mut writes: NonSendMut<Writes>| {
            if owner.generation() != generation {
                return;
            }
            let image = event.image.clone();
            let directory = directory.clone();
            if !writes.push(Box::new(move || {
                Completion::Manual(generation, save(image, &directory, crop))
            })) {
                owner.status = "Screenshot failed: write queue is full".into();
            }
        },
    );
}

fn save(image: Image, directory: &str, crop: Option<WorldViewport>) -> io::Result<PathBuf> {
    pixels(image, crop)?.save(directory)
}

fn pixels(image: Image, crop: Option<WorldViewport>) -> io::Result<Pixels> {
    let mut image = image.try_into_dynamic().map_err(io::Error::other)?;
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
    Pixels::new(rgba.width(), rgba.height(), rgba.into_raw())
}

/// Audit mode keeps its fixed PNG name, replacing it only after encoding completes.
pub(super) fn request_audit(
    commands: &mut Commands,
    readback: Option<Entity>,
    generation: u64,
    path: PathBuf,
) {
    let entity = readback.unwrap_or_else(|| commands.spawn(Screenshot::primary_window()).id());
    commands.entity(entity).observe(
        move |event: On<ScreenshotCaptured>,
              owner: NonSend<HostOwner>,
              mut writes: NonSendMut<Writes>| {
            if owner.generation() != generation {
                return;
            }
            let image = event.image.clone();
            let path = path.clone();
            #[cfg(test)]
            let mut open = tests::audit_open();
            #[cfg(not(test))]
            let mut open = crate::play::screenshot::create_capture_file;
            if !writes.push(Box::new(move || {
                Completion::Audit(save_audit(image, &path, &mut open))
            })) {
                bevy::log::error!("Could not save audit screenshot: write queue is full");
            }
        },
    );
}

fn save_audit(
    image: Image,
    path: &std::path::Path,
    open: &mut impl FnMut(&std::path::Path) -> io::Result<std::fs::File>,
) -> io::Result<()> {
    use crate::play::screenshot::retry_capture_name;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);
    let image = image.try_into_dynamic().map_err(io::Error::other)?;
    let format = bevy::image::ImageFormat::Png
        .as_image_crate_format()
        .ok_or_else(|| io::Error::other("PNG support is unavailable"))?;
    let (temporary, mut file) = retry_capture_name(|| {
        let candidate = path.with_file_name(format!(
            ".oteryn-audit-{}-{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        let file = open(&candidate)?;
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

#[cfg(all(test, unix))]
mod audit_tests;
#[cfg(test)]
mod observer_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod deferred_tests;
