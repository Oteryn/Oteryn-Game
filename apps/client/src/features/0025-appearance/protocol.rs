use crate::play::feature_0115_mail::protocol::accepted;
use crate::play::{Outbox, Stepper, mailbox::lock};
use oteryn_client_runtime::{
    appearance as wire,
    raw_feature::{FeatureCommand, GatedCommandResult},
};
use oteryn_client_ui::appearance::{Intent, Panel};
pub(in crate::play) fn command(panel: &Panel, _: Intent) -> Result<FeatureCommand, String> {
    let catalogue = panel
        .catalogue
        .as_ref()
        .ok_or(tr!("Wait for the appearance catalogue"))?;
    let outfit = catalogue
        .outfits
        .iter()
        .find(|o| o.key == panel.fields[0].trim())
        .ok_or(tr!("Choose an outfit from the catalogue"))?;
    let addons: u8 = panel.fields[1]
        .trim()
        .parse()
        .map_err(|_| tr!("Enter an addon mask"))?;
    if addons & !outfit.permitted_addons != 0 {
        return Err(tr!("This outfit does not permit those addons").into());
    }
    let mount = panel.fields[2].trim();
    if !mount.is_empty() && !catalogue.mounts.iter().any(|key| key == mount) {
        return Err(tr!("Choose a mount from the catalogue").into());
    }
    let colours = panel.fields[3]
        .split(',')
        .map(|s| s.trim().parse::<u16>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| tr!("Enter four palette indices separated by commas"))?;
    let colours: [u16; 4] = colours
        .try_into()
        .map_err(|_| tr!("Enter four palette indices"))?;
    if colours
        .iter()
        .any(|c| usize::from(*c) >= catalogue.palette.len())
    {
        return Err(tr!("Choose colours from the catalogue palette").into());
    }
    let change = wire::Change {
        context: catalogue.context,
        expected_root_revision: catalogue.root_revision,
        prior: catalogue.selected.clone(),
        selection: wire::Selection {
            outfit_key: outfit.key.clone(),
            colours,
            addons,
            mount_key: (!mount.is_empty()).then(|| mount.to_owned()),
        },
        content_revision: panel.fields[4].trim().into(),
    };
    Ok(FeatureCommand {
        capability: wire::CAPABILITY_APPEARANCE_V1,
        kind: wire::COMMAND_TYPE_APPEARANCE_CHANGE,
        payload: change
            .encode()
            .map_err(|_| tr!("Enter a valid content revision"))?,
    })
}
pub(in crate::play) fn apply(
    panel: &mut Panel,
    command: &FeatureCommand,
    result: &GatedCommandResult,
) {
    if !panel.pending || panel.pending_kind != command.kind {
        return;
    }
    panel.pending = false;
    panel.feedback = if !accepted(result) {
        tr!("The server refused the appearance request").into()
    } else {
        match wire::ChangeResult::decode(&result.payload) {
            Ok(wire::ChangeResult::Committed(revision)) => {
                lfmt!(
                    "Appearance committed at revision {revision}; catalogue updates follow",
                    revision = revision
                )
            }
            Ok(wire::ChangeResult::Rejected) => {
                tr!("The server refused the appearance request").into()
            }
            Err(_) => tr!("Invalid appearance reply").into(),
        }
    };
}
fn catalogue(panel: &mut Panel, value: wire::Catalogue) {
    if let Some(before) = &panel.catalogue {
        let mut unlocked = Vec::new();
        for outfit in &value.outfits {
            match before.outfits.iter().find(|old| old.key == outfit.key) {
                None => unlocked.push(lfmt!("Unlocked outfit: {}", outfit.key)),
                Some(old) if outfit.permitted_addons & !old.permitted_addons != 0 => {
                    unlocked.push(lfmt!(
                        "Unlocked addon: {}; allowed mask: {}",
                        outfit.key,
                        outfit.permitted_addons
                    ));
                }
                _ => {}
            }
        }
        for mount in &value.mounts {
            if !before.mounts.contains(mount) {
                unlocked.push(lfmt!("Unlocked mount: {mount}", mount = mount));
            }
        }
        if !unlocked.is_empty() {
            panel.feedback = unlocked.join("\r\n");
        }
    }
    if panel.catalogue.is_none() {
        if let Some(selected) = &value.selected {
            panel.fields[0].clone_from(&selected.selection.outfit_key);
            panel.fields[1] = selected.selection.addons.to_string();
            panel.fields[2] = selected.selection.mount_key.clone().unwrap_or_default();
            let [head, body, legs, feet] = selected.selection.colours;
            panel.fields[3] = format!("{head},{body},{legs},{feet}");
        } else {
            panel.fields[0] = value
                .outfits
                .first()
                .map_or_else(String::new, |o| o.key.clone());
            panel.fields[1] = "0".into();
            panel.fields[3] = "0,0,0,0".into();
        }
    }
    panel.confirmed.clone_from(&value.selected);
    panel.summary = if let Some(selected) = &value.selected {
        lfmt!(
            "Current outfit: {}\r\nAddons: {}\r\nMount: {}\r\nColours: {:?}\r\n",
            selected.selection.outfit_key,
            selected.selection.addons,
            selected
                .selection
                .mount_key
                .as_deref()
                .unwrap_or(tr!("None")),
            selected.selection.colours
        )
    } else {
        tr!("No outfit selected\r\n").into()
    };
    for outfit in &value.outfits {
        panel.summary.push_str(&lfmt!(
            "Outfit: {}; allowed addon mask: {}\r\n",
            outfit.key,
            outfit.permitted_addons
        ));
    }
    for mount in &value.mounts {
        panel
            .summary
            .push_str(&lfmt!("Mount: {mount}\r\n", mount = mount));
    }
    panel.summary.push_str(&lfmt!(
        "Palette indices: 0..{}",
        value.palette.len().saturating_sub(1)
    ));
    panel.catalogue = Some(value);
}
impl Outbox {
    pub(in crate::play) fn appearance_initialize<T: Stepper>(&self, session: &T) {
        let mut shared = lock(&self.0);
        let panel = &mut shared.properties.appearance;
        if !panel.selected {
            return;
        }
        if let Some(snapshot) = session
            .feature_snapshot(wire::STATE_DOMAIN_APPEARANCE)
            .and_then(|bytes| wire::Snapshot::decode(&bytes).ok())
        {
            catalogue(panel, snapshot.catalogue);
        } else {
            panel.feedback = tr!("Invalid appearance catalogue").into();
        }
    }
    pub(in crate::play) fn appearance_events(&self, events: &[oteryn_session::SessionEvent]) {
        let mut shared = lock(&self.0);
        let panel = &mut shared.properties.appearance;
        if !panel.selected {
            return;
        }
        for event in events {
            match event {
                oteryn_session::SessionEvent::Appearance(delta) => {
                    if let oteryn_session::AppearanceUpdate::Catalogue(value) = &delta.value {
                        catalogue(panel, value.clone());
                    }
                }
                oteryn_session::SessionEvent::GatedSnapshotReplaced(delta)
                    if delta.value.domain_id == wire::STATE_DOMAIN_APPEARANCE =>
                {
                    if let Ok(snapshot) = wire::Snapshot::decode(&delta.value.payload) {
                        catalogue(panel, snapshot.catalogue);
                    }
                }
                _ => {}
            }
        }
    }
}
#[cfg(test)]
#[path = "tests/e2e_s06.rs"]
mod e2e_s06;
