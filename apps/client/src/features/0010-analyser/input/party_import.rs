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
#[cfg(test)]
pub(super) fn import(panel: &mut Panel, path: &Path) -> io::Result<()> {
    if !panel.selected {
        return Err(io::Error::other(tr!("No active analyser session")));
    }
    let imported = read_summary(path)?;
    apply(panel, imported);
    Ok(())
}
fn read_summary(path: &Path) -> io::Result<Summary> {
    let mut text = String::new();
    std::fs::File::open(path)?
        .take((party_import::MAX_BYTES + 1) as u64)
        .read_to_string(&mut text)?;
    Summary::parse(&text).map_err(io::Error::other)
}
fn apply(panel: &mut Panel, imported: Summary) {
    panel.party_import = Some(imported);
    panel.tool = Tool::Party;
    panel.feedback = None;
}
pub(super) fn export(panel: &Panel, path: &Path) -> io::Result<()> {
    if !panel.selected {
        return Err(io::Error::other(tr!("No active analyser session")));
    }
    let text = if let Some(imported) = &panel.party_import {
        imported.export()
    } else {
        if panel.party_pending || panel.party_feedback.is_some() {
            return Err(io::Error::other(tr!(
                "Refresh complete party measurements before export"
            )));
        }
        party_import::export_adjusted(&panel.party, &panel.party_adjustments)
    }
    .map_err(io::Error::other)?;
    std::fs::write(path, text)
}
#[cfg(test)]
pub(super) fn submit(panel: &mut Panel, importing: bool, path: Option<&str>) -> bool {
    let Some(path) = path.filter(|path| !path.trim().is_empty()) else {
        panel.feedback = Some(tr!("Choose a party summary file path"));
        return false;
    };
    panel.export_path = path.to_owned();
    let result = if importing {
        import(panel, Path::new(path))
    } else {
        export(panel, Path::new(path))
    };
    panel.feedback = Some(match (importing, result.is_ok()) {
        (true, true) => tr!("Party summary imported; supplied values are advisory"),
        (false, true) => tr!("Party summary exported"),
        (true, false) => tr!("Import failed; choose a valid Oteryn party summary of at most 8 KiB"),
        (false, false) => {
            tr!("Export failed; refresh complete priced measurements and choose a writable path")
        }
    });
    result.is_ok()
}
impl PlayLink {
    pub fn analyser_import_party(&self, path: &Path) -> io::Result<()> {
        if !lock(&self.mailbox).analyser.selected {
            return Err(io::Error::other(tr!("No active analyser session")));
        }
        let imported = read_summary(path)?;
        let mut mailbox = lock(&self.mailbox);
        if !mailbox.analyser.selected {
            return Err(io::Error::other(tr!("No active analyser session")));
        }
        apply(&mut mailbox.analyser, imported);
        Ok(())
    }
    pub fn analyser_export_party(&self, path: &Path) -> io::Result<()> {
        let panel = lock(&self.mailbox).analyser.clone();
        export(&panel, path)
    }
}
