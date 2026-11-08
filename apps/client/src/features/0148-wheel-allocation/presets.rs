use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::wheel_allocation::presets::PresetBook;
use std::path::Path;
fn path() -> Result<std::path::PathBuf, String> {
    Ok(crate::native_login::settings_path()?.with_file_name("wheel-presets.json"))
}
pub(in crate::play) fn submit(mailbox: &mut Mailbox, action: usize, path: &Path) {
    let panel = &mut mailbox.wheel;
    let result: Result<(), &str> = (|| {
        match action {
            0 => {
                let mut book = panel.preset_book.clone();
                book.insert(&panel.preset_fields[0], &panel.build_code())?;
                book.save(path)?;
                panel.preset_book = book;
            }
            1 => {
                let code = panel
                    .preset_book
                    .0
                    .get(panel.preset_fields[0].trim())
                    .ok_or("Select a saved preset name")?
                    .clone();
                panel.import_build(&code)?;
            }
            2 => {
                let mut book = panel.preset_book.clone();
                if book.0.remove(panel.preset_fields[0].trim()).is_none() {
                    return Err("Select a saved preset name");
                }
                book.save(path)?;
                panel.preset_book = book;
            }
            3 => panel.preset_fields[1] = panel.build_code(),
            4 => {
                panel.preset_fields[1] =
                    panel.build_code().replacen("OW1:", "oteryn://wheel/v1/", 1)
            }
            5 => panel.import_build(&panel.preset_fields[1].clone())?,
            _ => return Err("Unknown preset action"),
        }
        Ok(())
    })();
    panel.preset_feedback = result.map_or_else(str::to_owned, |()| {
        "Preset updated. Loaded points are proposals; Save Wheel submits them.".into()
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
        let loaded = path().and_then(|path| PresetBook::load(&path).map_err(str::to_owned));
        {
            let mut mailbox = lock(&self.mailbox);
            match loaded {
                Ok(book) => mailbox.wheel.preset_book = book,
                Err(message) => mailbox.wheel.preset_feedback = message,
            }
        }
        #[cfg(windows)]
        open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).wheel.presets_open = true;
        }
    }
}
#[cfg(windows)]
fn open_window(link: &PlayLink) {
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Wheel builds (points only)",
            labels: &["Preset name", "Build code or URL (select and copy / paste)"],
            buttons: &[
                "Save named build",
                "Load named build",
                "Delete named build",
                "Export code",
                "Export URL",
                "Import code / URL",
            ],
            snapshot: |mailbox| {
                let p = &mailbox.wheel;
                (
                    p.presets_open,
                    !p.ready(),
                    p.preset_fields.to_vec(),
                    format!(
                        "{}\r\nSaved builds:\r\n{}",
                        p.preset_feedback,
                        p.preset_book
                            .0
                            .keys()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join("\r\n")
                    ),
                )
            },
            set_open: |mailbox, value| mailbox.wheel.presets_open = value,
            submit: |mailbox, _, action, values| {
                for (slot, value) in mailbox.wheel.preset_fields.iter_mut().zip(values) {
                    *slot = value;
                }
                match path() {
                    Ok(path) => submit(mailbox, action, &path),
                    Err(message) => mailbox.wheel.preset_feedback = message,
                }
            },
        },
    );
}
