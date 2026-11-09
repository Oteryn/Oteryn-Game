use crate::{combat_input::PlayCommand, play::play_channel};
use oteryn_client_runtime::{
    cyclopedia_character::gallery::{Entry, Kind, Page},
    raw_feature::{CommandStatus, GatedCommandResult},
};
use oteryn_client_ui::cyclopedia_character::Intent;
#[test]
fn t093_gallery_uses_confirmed_server_looks_and_filters_all_three_kinds() -> Result<(), &'static str>
{
    let (link, mut commands, outbox) = play_channel();
    outbox.cyclopedia_character_initialize(true);
    link.cyclopedia_character_request(Intent::Appearances);
    let Ok(PlayCommand::Feature(command)) = commands.try_recv() else {
        return Err("command");
    };
    assert_eq!((command.capability, command.kind), (199, 388));
    assert_eq!(command.payload, [129, 0, 0]);
    let entries = [Kind::Outfit, Kind::Mount, Kind::Familiar]
        .into_iter()
        .enumerate()
        .map(|(i, kind)| Entry {
            kind,
            key: format!("oteryn:appearance/entry_{i}"),
            look_type: i as u32 + 100,
        })
        .collect();
    let reply = GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: Page {
            offset: 0,
            total: 3,
            entries,
        }
        .encode()
        .map_err(|_| "encode")?,
    };
    outbox.cyclopedia_character_result(&reply);
    let mut panel = link.cyclopedia_character_panel();
    assert_eq!(
        panel.gallery.page.as_ref().ok_or("gallery")?.entries.len(),
        3
    );
    for kind in [Kind::Outfit, Kind::Mount, Kind::Familiar] {
        panel.gallery.filter = Some(kind);
        let rows = panel.gallery.rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, kind);
    }
    link.cyclopedia_character_request(Intent::GalleryNext);
    assert!(commands.try_recv().is_err());
    let confirmed = link.cyclopedia_character_panel().gallery.page;
    link.cyclopedia_character_request(Intent::Appearances);
    assert!(commands.try_recv().is_ok());
    outbox.cyclopedia_character_result(&GatedCommandResult {
        payload: vec![129],
        ..reply
    });
    let panel = link.cyclopedia_character_panel();
    assert_eq!(panel.gallery.page, confirmed);
    assert!(panel.feedback.is_some());
    Ok(())
}
