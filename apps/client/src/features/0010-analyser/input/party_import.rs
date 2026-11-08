//! Files are chosen by the local user; imported records never become server commands.
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::analyser::{
    Panel, Tool,
    party_import::{self, Summary},
};
use std::{
    io::{self, Read},
    path::Path,
};
pub(super) fn import(panel: &mut Panel, path: &Path) -> io::Result<()> {
    if !panel.selected {
        return Err(io::Error::other("No active analyser session"));
    }
    let mut text = String::new();
    std::fs::File::open(path)?
        .take((party_import::MAX_BYTES + 1) as u64)
        .read_to_string(&mut text)?;
    let imported = Summary::parse(&text).map_err(io::Error::other)?;
    panel.party_import = Some(imported);
    panel.tool = Tool::Party;
    panel.feedback = None;
    Ok(())
}
pub(super) fn export(panel: &Panel, path: &Path) -> io::Result<()> {
    if !panel.selected {
        return Err(io::Error::other("No active analyser session"));
    }
    let text = if let Some(imported) = &panel.party_import {
        imported.export()
    } else {
        if panel.party_pending || panel.party_feedback.is_some() {
            return Err(io::Error::other(
                "Refresh complete party measurements before export",
            ));
        }
        party_import::export_rows(&panel.party)
    }
    .map_err(io::Error::other)?;
    std::fs::write(path, text)
}
#[cfg(any(windows, test))]
pub(super) fn submit(
    mailbox: &mut crate::play::mailbox::Mailbox,
    importing: bool,
    path: Option<&str>,
) {
    let Some(path) = path.filter(|path| !path.trim().is_empty()) else {
        mailbox.analyser.feedback = Some("Choose a party summary file path");
        return;
    };
    mailbox.analyser.export_path = path.to_owned();
    let result = if importing {
        import(&mut mailbox.analyser, Path::new(path))
    } else {
        export(&mailbox.analyser, Path::new(path))
    };
    mailbox.analyser.feedback = Some(match (importing, result.is_ok()) {
        (true, true) => "Party summary imported; supplied values are advisory",
        (false, true) => "Party summary exported",
        (true, false) => "Import failed; choose a valid Oteryn party summary of at most 8 KiB",
        (false, false) => {
            "Export failed; refresh complete priced measurements and choose a writable path"
        }
    });
}
impl PlayLink {
    pub fn analyser_import_party(&self, path: &Path) -> io::Result<()> {
        import(&mut lock(&self.mailbox).analyser, path)
    }
    pub fn analyser_export_party(&self, path: &Path) -> io::Result<()> {
        export(&lock(&self.mailbox).analyser, path)
    }
}
