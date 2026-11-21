use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::wheel_allocation::presets::PresetBook;
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::UnboundedSender;
fn path() -> Result<std::path::PathBuf, String> {
    Ok(crate::native_login::settings_path()?.with_file_name("wheel-presets.json"))
}
#[cfg(bevy_shell)]
pub(in crate::play) fn submit_named(shared: &Arc<Mutex<Mailbox>>, action: usize, selected: String) {
    match path() {
        Ok(path) => submit_shared(shared, action, selected, path, true),
        Err(message) => lock(shared).wheel.preset_feedback = message,
    }
}

#[cfg(test)]
pub(in crate::play) fn submit(mailbox: &mut Mailbox, action: usize, path: &Path) {
    submit_at(mailbox, action, "", path);
}
fn submit_at(mailbox: &mut Mailbox, action: usize, selected: &str, path: &Path) {
    submit_panel(&mut mailbox.wheel, action, selected, path);
}

fn submit_panel(
    panel: &mut oteryn_client_ui::wheel_allocation::WheelPanel,
    action: usize,
    selected: &str,
    path: &Path,
) {
    let result: Result<(), &str> = (|| {
        match action {
            0 => {
                let book = update_book(path, |book| {
                    book.insert(&panel.preset_fields[0], &panel.build_code())?;
                    Ok(())
                })?;
                panel.preset_fields[0] = panel.preset_fields[0].trim().into();
                panel.preset_book = book;
            }
            1 => {
                let code = panel
                    .preset_book
                    .0
                    .get(panel.preset_fields[0].trim())
                    .ok_or(tr!("Select a saved preset name"))?
                    .clone();
                panel.import_build(&code)?;
                panel.diagram.reset_requested = false;
            }
            2 => {
                let book = update_book(path, |book| {
                    if book.0.remove(panel.preset_fields[0].trim()).is_none() {
                        return Err(tr!("Select a saved preset name"));
                    }
                    Ok(())
                })?;
                panel.preset_book = book;
                panel.preset_fields[0].clear();
            }
            3 => panel.preset_fields[1] = panel.build_code(),
            4 => {
                panel.preset_fields[1] =
                    panel.build_code().replacen("OW1:", "oteryn://wheel/v1/", 1)
            }
            5 => {
                panel.import_build(&panel.preset_fields[1].clone())?;
                panel.diagram.reset_requested = false;
            }
            6 => {
                let name = panel.preset_fields[0].trim();
                let book = update_book(path, |book| {
                    if name != selected && book.0.contains_key(name) {
                        return Err("A preset with that name already exists");
                    }
                    let code = book
                        .0
                        .remove(selected)
                        .ok_or("Select a saved preset name")?;
                    book.insert(name, &code)?;
                    Ok(())
                })?;
                panel.preset_fields[0] = name.into();
                panel.preset_book = book;
            }
            _ => return Err(tr!("Unknown preset action")),
        }
        Ok(())
    })();
    if action == 6 && result.is_err() {
        panel.preset_fields[0] = selected.into();
    }
    panel.preset_feedback = result.map_or_else(str::to_owned, |()| {
        tr!("Preset updated. Loaded points are proposals; Save Wheel submits them.").into()
    });
}
fn update_book(
    path: &Path,
    edit: impl FnOnce(&mut PresetBook) -> Result<(), &'static str>,
) -> Result<PresetBook, &'static str> {
    let error = || tr!("Cannot save presets");
    let _lock = crate::atomic_file::lock(path).map_err(|_| error())?;
    let mut book = match PresetBook::load(path) {
        // The book API returns display text. These errors represent I/O or a future schema.
        Err(message)
            if [
                tr!("Cannot inspect presets"),
                tr!("Cannot read presets"),
                tr!("Unsupported preset version"),
            ]
            .contains(&message) =>
        {
            return Err(message);
        }
        Err(_) => {
            crate::atomic_file::preserve_malformed(path).map_err(|_| error())?;
            PresetBook::default()
        }
        Ok(book) => book,
    };
    edit(&mut book)?;
    let bytes = serde_json::to_vec_pretty(&serde_json::json!({"version": 1, "presets": book.0}))
        .map_err(|_| error())?;
    crate::atomic_file::replace(path, &bytes, true).map_err(|_| error())?;
    Ok(book)
}

fn submit_shared(
    shared: &Arc<Mutex<Mailbox>>,
    action: usize,
    selected: String,
    path: std::path::PathBuf,
    background: bool,
) {
    if !matches!(action, 0 | 2 | 6) {
        submit_at(&mut lock(shared), action, &selected, &path);
        return;
    }
    let mut snapshot = {
        let mut mailbox = lock(shared);
        if mailbox.wheel_preset_save_pending {
            return;
        }
        mailbox.wheel_preset_save_pending = true;
        mailbox.wheel.clone()
    };
    let field = snapshot.preset_fields[0].clone();
    let shared = shared.clone();
    let job = move || {
        submit_panel(&mut snapshot, action, &selected, &path);
        let mut mailbox = lock(&shared);
        mailbox.wheel_preset_save_pending = false;
        mailbox.wheel.preset_book = snapshot.preset_book;
        mailbox.wheel.preset_feedback = snapshot.preset_feedback;
        if mailbox.wheel.preset_fields[0] == field {
            mailbox.wheel.preset_fields[0] = snapshot.preset_fields[0].clone();
        }
    };
    #[cfg(bevy_shell)]
    if background {
        bevy_tasks::IoTaskPool::get_or_init(|| {
            bevy_tasks::TaskPoolBuilder::new().num_threads(1).build()
        })
        .spawn(async move { job() })
        .detach();
        return;
    }
    #[cfg(not(bevy_shell))]
    let _ = background;
    job();
}

impl PlayLink {
    pub fn wheel_presets_request(&self, action: usize) {
        match path() {
            Ok(path) => submit_shared(&self.mailbox, action, String::new(), path, cfg!(bevy_shell)),
            Err(message) => lock(&self.mailbox).wheel.preset_feedback = message,
        }
    }

    pub fn wheel_presets_open(&self) {
        open_shared(&self.mailbox, &self.commands);
    }
}
pub(in crate::play) fn window_open(mailbox: &Mailbox) -> bool {
    mailbox.wheel.presets_open && !mailbox.wheel_shutdown
}
pub(in crate::play) fn open_shared(
    shared: &Arc<Mutex<Mailbox>>,
    commands: &UnboundedSender<PlayCommand>,
) {
    {
        let mailbox = lock(shared);
        if mailbox.wheel_shutdown || window_open(&mailbox) {
            return;
        }
    }
    let loaded = path().and_then(|path| PresetBook::load(&path).map_err(str::to_owned));
    {
        let mut mailbox = lock(shared);
        match loaded {
            Ok(book) => mailbox.wheel.preset_book = book,
            Err(message) => mailbox.wheel.preset_feedback = message,
        }
    }
    let _ = commands;
    lock(shared).wheel.presets_open = true;
}

#[cfg(test)]
#[path = "tests/e2e_n02.rs"]
mod e2e_n02;

#[cfg(test)]
mod management_tests {
    use super::*;

    #[test]
    fn review_3376_malformed_wheel_book_is_recovered_on_save()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("wheel.json");
        std::fs::write(&path, b"{")?;
        let (link, _, _) = crate::play::play_channel();
        let mut mailbox = lock(&link.mailbox);
        mailbox.wheel.preset_fields[0] = "Recovered".into();
        submit_at(&mut mailbox, 0, "", &path);
        assert!(PresetBook::load(&path).is_ok_and(|book| book.0.contains_key("Recovered")));
        Ok(())
    }

    #[test]
    fn wheel_presets_create_rename_delete_are_durable_and_collision_safe()
    -> Result<(), Box<dyn std::error::Error>> {
        let (link, _commands, _outbox) = crate::play::play_channel();
        let mut m = lock(&link.mailbox);
        let fixture = tempfile::tempdir()?;
        let path = fixture.path().join("oteryn-gap38.json");
        m.wheel.points[14] = 7;
        m.wheel.preset_fields[0] = "Hunting".into();
        submit_at(&mut m, 0, "", &path);
        let code = m.wheel.build_code();
        assert_eq!(PresetBook::load(&path)?.0.get("Hunting"), Some(&code));
        m.wheel.points[14] = 9;
        m.wheel.preset_fields[0] = "Other".into();
        submit_at(&mut m, 0, "", &path);
        let before = m.wheel.preset_book.clone();
        submit_at(&mut m, 6, "Hunting", &path);
        assert_eq!(
            m.wheel.preset_feedback,
            "A preset with that name already exists"
        );
        assert_eq!(m.wheel.preset_book, before);
        assert_eq!(PresetBook::load(&path)?, before);
        m.wheel.preset_fields[0] = "  Bosses  ".into();
        submit_at(&mut m, 6, "Hunting", &path);
        assert_eq!(m.wheel.preset_fields[0], "Bosses");
        let loaded = PresetBook::load(&path)?;
        assert!(!loaded.0.contains_key("Hunting"));
        assert_eq!(loaded.0.get("Bosses"), Some(&code));
        assert_eq!(m.wheel.points[14], 9, "rename keeps the saved allocation");
        m.wheel.preset_fields[0] = " ".into();
        submit_at(&mut m, 6, "Bosses", &path);
        assert_eq!(m.wheel.preset_book, loaded);
        m.wheel.preset_fields[0] = "Failed write".into();
        submit_at(&mut m, 6, "Bosses", &path.join("not-a-directory"));
        assert_eq!(m.wheel.preset_book, loaded);
        assert_eq!(PresetBook::load(&path)?, loaded);
        m.wheel.preset_fields[0] = "Bosses".into();
        submit_at(&mut m, 2, "", &path);
        assert!(!PresetBook::load(&path)?.0.contains_key("Bosses"));
        std::fs::remove_file(path)?;
        Ok(())
    }
}
