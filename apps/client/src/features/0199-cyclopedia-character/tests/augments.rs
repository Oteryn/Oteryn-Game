use crate::combat_input::PlayCommand;
use crate::play::play_channel;
use oteryn_client_runtime::{
    CommandStatus,
    cyclopedia_character::{
        CharacterTab,
        combat::CombatStats,
        offence::{Augment, Effect, Page, Query, Snapshot, Source, SourceKind},
    },
    raw_feature::GatedCommandResult,
};
use oteryn_client_ui::cyclopedia_character::{
    AugmentIntent, CyclopediaCharacterPanel, Intent, augment_heading, augment_rows,
};
#[test]
fn spell_augments_reach_the_native_view_through_real_playlink_and_outbox()
-> Result<(), &'static str> {
    let (link, mut commands, outbox) = play_channel();
    outbox.cyclopedia_character_initialize(true);
    link.cyclopedia_character_request(Intent::SpellAugments(AugmentIntent::Refresh));
    let Ok(PlayCommand::Feature(command)) = commands.try_recv() else {
        return Err("spell query");
    };
    assert_eq!(command.kind, 388);
    assert_eq!(command.payload, vec![131, 0, 7]);
    let stats = CombatStats::new(CharacterTab::Offence, 1234, 2, 3).map_err(|_| "stats")?;
    let snapshot = Snapshot {
        known_sources: 7,
        sources: vec![Source {
            kind: SourceKind::Equipment,
            slot: 5,
            key: "oteryn:item/weapon".into(),
        }],
        rows: vec![Augment {
            source: 0,
            spell: "A spell".into(),
            effect: Effect::BaseDamage,
            value: 1250,
        }],
    };
    let page = Page::from_snapshot([1; 16], Query::default(), stats.clone(), Some(snapshot))
        .map_err(|_| "page")?;
    outbox.cyclopedia_character_result(&GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: page.encode().map_err(|_| "payload")?,
    });
    let panel = link.cyclopedia_character_panel();
    assert_eq!(panel.combat, Some(stats));
    assert!(panel.augments.open);
    assert!(panel.feedback.is_none());
    let rows = augment_rows(&panel);
    assert_eq!(rows.len(), 1);
    assert!(rows[0].label.contains("12.50%"));
    assert!(rows[0].detail.contains("oteryn:item/weapon"));
    assert!(augment_heading(&panel).contains("gear revision 3"));
    Ok(())
}
#[test]
fn native_modifier_source_buttons_parse_target_and_keep_pending_fence() -> Result<(), &'static str>
{
    use crate::play::feature_0199_cyclopedia_character::view::native;
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        ..Default::default()
    };
    let command = native::submit(
        &mut p,
        184,
        ["ignored", "ignored", "00112233-4455-6677-8899-aabbccddeeff"],
    )
    .ok_or("wheel")?;
    let query = Query::decode(&command.payload).map_err(|_| "query")?;
    assert_eq!(query.sources, 4);
    assert_eq!(query.target, p.target);
    assert_eq!(command.payload.len(), 19);
    assert!(p.augments.open);
    assert!(native::submit(&mut p, 181, ["0", "20", ""]).is_none());
    assert_eq!(p.target, query.target);
    p.pending = false;
    assert!(native::submit(&mut p, 180, ["0", "20", "invalid"]).is_none());
    assert!(!p.pending);
    let inspection = native::submit(
        &mut p,
        107,
        ["0", "20", "00112233-4455-6677-8899-aabbccddeeff"],
    )
    .ok_or("leave modifiers for inspection")?;
    assert_eq!(inspection.payload.first(), Some(&129));
    assert!(!p.augments.open);
    Ok(())
}

#[test]
fn activity_handoff_cancels_the_source_modifier_query_and_observation() -> Result<(), &'static str>
{
    let (link, mut commands, outbox) = play_channel();
    outbox.cyclopedia_character_initialize(true);
    link.cyclopedia_character_request(Intent::SpellAugments(AugmentIntent::Refresh));
    assert!(matches!(commands.try_recv(), Ok(PlayCommand::Feature(_))));
    assert!(
        link.cyclopedia_character_panel()
            .augments
            .requested
            .is_some()
    );
    crate::play::mailbox::lock(&link.mailbox).cancel_activity_requests();
    let panel = link.cyclopedia_character_panel();
    assert!(!panel.pending);
    assert!(panel.augments.requested.is_none());
    assert!(panel.augments.page.is_none());
    assert!(!panel.augments.open);
    Ok(())
}

#[test]
fn native_modifiers_wait_for_the_independent_blessings_reply() -> Result<(), &'static str> {
    use oteryn_client_ui::cyclopedia_character::{CyclopediaCharacterPanel, request_blessings};
    let mut panel = CyclopediaCharacterPanel {
        selected: true,
        ..Default::default()
    };
    request_blessings(&mut panel).ok_or("blessings query")?;
    assert!(crate::play::feature_0199_cyclopedia_character::view::native::display(&panel).contains("Waiting for the server"));
    assert!(crate::play::feature_0199_cyclopedia_character::view::native::submit(&mut panel, 180, ["0", "20", ""]).is_none());
    assert!(panel.blessings.pending);
    assert!(!panel.pending);
    assert!(panel.augments.requested.is_none());
    assert!(oteryn_client_ui::cyclopedia_character::augment_request(&mut panel, AugmentIntent::Refresh).is_none());
    assert!(panel.augments.requested.is_none());
    Ok(())
}
