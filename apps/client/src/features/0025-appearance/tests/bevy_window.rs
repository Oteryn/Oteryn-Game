use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayLink, play_channel},
};
use oteryn_client_runtime::appearance as wire;
use oteryn_client_ui::appearance::Panel;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;

pub(crate) fn catalogue() -> wire::Catalogue {
    wire::Catalogue {
        context: wire::Context {
            content_generation: [1; 32],
            cosmetics_digest: [2; 32],
        },
        root_revision: 7,
        selected: Some(wire::Selected {
            selection: wire::Selection {
                outfit_key: "earned".into(),
                colours: [0; 4],
                addons: 0,
                mount_key: None,
            },
            revision: 7,
            provenance: "player".into(),
        }),
        palette: vec![[20, 40, 60], [80, 100, 120]],
        outfits: vec![
            wire::OutfitOption {
                key: "earned".into(),
                look_type: 128,
                permitted_addons: 1,
            },
            wire::OutfitOption {
                key: "reward".into(),
                look_type: 129,
                permitted_addons: 2,
            },
        ],
        mounts: vec!["earned-mount".into()],
    }
}
pub(crate) fn publish(outbox: &Outbox, value: wire::Catalogue) {
    let mut mailbox = crate::play::mailbox::lock(&outbox.0);
    mailbox.properties.appearance.selected = true;
    super::super::protocol::catalogue_for_bevy_test(&mut mailbox.properties.appearance, value);
}
fn panel(link: &PlayLink) -> Panel {
    link.appearance_panel()
}
#[test]
fn appearance_apply_is_bound_without_optimistic_change() -> Result {
    use super::super::view::AppearanceControl as Control;
    let (link, mut commands, outbox) = play_channel();
    publish(&outbox, catalogue());
    link.appearance_open();
    let before = link.appearance_snapshot();
    assert!(link.appearance_dispatch_bound(&before, Control::Outfit("reward".into())));
    let draft = link.appearance_snapshot();
    assert_eq!(draft.panel.confirmed, before.panel.confirmed);
    assert!(link.appearance_dispatch_bound(&draft, Control::Apply("r1".into())));
    assert!(!link.appearance_dispatch_bound(&draft, Control::Apply("r1".into())));
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("wrong command".into());
    };
    let change = wire::Change::decode(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(change.selection.outfit_key, "reward");
    assert_eq!(change.prior, before.panel.confirmed);
    assert_eq!(change.context, catalogue().context);
    assert_eq!(change.expected_root_revision, 7);
    assert!(commands.try_recv().is_err());
    assert_eq!(panel(&link).confirmed, before.panel.confirmed);
    outbox.bevy_appearance_reply(&command);
    assert!(!panel(&link).pending);
    assert_eq!(panel(&link).confirmed, before.panel.confirmed);
    let mut reply = catalogue();
    reply.root_revision = 8;
    reply.selected = Some(wire::Selected {
        selection: change.selection,
        revision: 8,
        provenance: "player".into(),
    });
    publish(&outbox, reply.clone());
    assert_eq!(panel(&link).confirmed, reply.selected);
    Ok(())
}
#[test]
fn appearance_apply_rejected_when_session_replaced() -> Result {
    use super::super::view::AppearanceControl as Control;
    let (link, mut commands, outbox) = play_channel();
    publish(&outbox, catalogue());
    link.appearance_open();
    let snapshot = link.appearance_snapshot();
    let (replacement, mut other_commands, other_outbox) = play_channel();
    publish(&other_outbox, catalogue());
    replacement.appearance_open();
    let other = panel(&replacement);
    assert!(!replacement.appearance_dispatch_bound(&snapshot, Control::Apply("r1".into())));
    assert_eq!(panel(&replacement), other);
    assert!(other_commands.try_recv().is_err());
    let modal = link.bevy_server_modal_fixture();
    let blocked = panel(&link);
    assert!(!link.appearance_dispatch_bound(&snapshot, Control::Apply("r1".into())));
    assert_eq!(panel(&link), blocked);
    drop(modal);
    assert!(link.appearance_dispatch_bound(&snapshot, Control::Close));
    let closed = panel(&link);
    assert!(!link.appearance_dispatch_bound(&snapshot, Control::Apply("r1".into())));
    assert_eq!(panel(&link), closed);
    link.appearance_open();
    let snapshot = link.appearance_snapshot();
    outbox.bevy_appearance_end();
    let ended = panel(&link);
    assert!(!link.appearance_dispatch_bound(&snapshot, Control::Apply("r1".into())));
    assert_eq!(panel(&link), ended);
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn appearance_bound_rejects_ungranted_and_stale_drafts() -> Result {
    use super::super::view::AppearanceControl as Control;
    let (link, mut commands, outbox) = play_channel();
    publish(&outbox, catalogue());
    link.appearance_open();
    let snapshot = link.appearance_snapshot();
    for control in [
        Control::Outfit("locked".into()),
        Control::Mount(Some("locked-mount".into())),
        Control::Addon(2),
        Control::Colour { part: 0, index: 2 },
        Control::Colour { part: 4, index: 0 },
    ] {
        assert!(!link.appearance_dispatch_bound(&snapshot, control));
        assert_eq!(panel(&link), snapshot.panel);
    }
    assert!(link.appearance_colour(0));
    let changed = panel(&link);
    assert!(!link.appearance_dispatch_bound(&snapshot, Control::Apply("r1".into())));
    assert_eq!(panel(&link), changed);
    assert!(commands.try_recv().is_err());
    drop(commands);
    let snapshot = link.appearance_snapshot();
    assert!(!link.appearance_dispatch_bound(&snapshot, Control::Apply("r1".into())));
    assert_eq!(panel(&link), snapshot.panel);
    Ok(())
}
