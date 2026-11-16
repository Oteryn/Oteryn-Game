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
pub(in crate::play) fn submit_named(mailbox: &mut Mailbox, action: usize, selected: &str) {
    match path() {
        Ok(path) => submit_at(mailbox, action, selected, &path),
        Err(message) => mailbox.wheel.preset_feedback = message,
    }
}
pub(in crate::play) fn submit(mailbox: &mut Mailbox, action: usize, path: &Path) {
    submit_at(mailbox, action, "", path);
}
fn submit_at(mailbox: &mut Mailbox, action: usize, selected: &str, path: &Path) {
    let panel = &mut mailbox.wheel;
    let result: Result<(), &str> = (|| {
        match action {
            0 => {
                let mut book = panel.preset_book.clone();
                book.insert(&panel.preset_fields[0], &panel.build_code())?;
                book.save(path)?;
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
                let mut book = panel.preset_book.clone();
                if book.0.remove(panel.preset_fields[0].trim()).is_none() {
                    return Err(tr!("Select a saved preset name"));
                }
                book.save(path)?;
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
                let mut book = panel.preset_book.clone();
                let name = panel.preset_fields[0].trim();
                if name != selected && book.0.contains_key(name) {
                    return Err("A preset with that name already exists");
                }
                let code = book
                    .0
                    .remove(selected)
                    .ok_or("Select a saved preset name")?;
                book.insert(name, &code)?;
                book.save(path)?;
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
impl PlayLink {
    pub fn wheel_presets_request(&self, action: usize) {
        let mut mailbox = lock(&self.mailbox);
        match path() {
            Ok(path) => submit(&mut mailbox, action, &path),
            Err(message) => mailbox.wheel.preset_feedback = message,
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
    fn wheel_presets_create_rename_delete_are_durable_and_collision_safe()
    -> Result<(), Box<dyn std::error::Error>> {
        let (link, _commands, _outbox) = crate::play::play_channel();
        let mut m = lock(&link.mailbox);
        let path = std::env::temp_dir().join(format!("oteryn-gap38-{}.json", std::process::id()));
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
