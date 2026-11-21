use super::*;
use crate::combat_input::PlayCommand;
use oteryn_client_ui::server_info::{Query, Reply, SpellInfo};
use oteryn_session::{CommandStatus, GatedCommandResult};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
fn result(payload: Vec<u8>) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload,
    }
}
#[test]
fn server_info_quest_reply_reaches_bevy_rows_and_search() -> Result {
    let (host, mut commands, outbox) = super::tests::host()?;
    outbox.bevy_journal_quest_state(super::tests::quest_state());
    outbox.bevy_server_info_initialize();
    let link = &host.play.as_ref().ok_or("play")?.link;
    link.server_info_request(Query::Quests);
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("title request".into());
    };
    let query = Query::decode(&command.payload).ok_or("query")?;
    assert_eq!(query, Query::Quests);
    let reply = Reply::Quests(vec![(73, "Rat Investigation".into())]);
    outbox.bevy_server_info_result(&query, &result(reply.encode().ok_or("encode")?));
    assert!(link.quest_select_mission(73, 1));
    let panel = link.quest_panel();
    let projection = model::quests(&panel, 0, false);
    assert_eq!(projection.rows[0].label, "Rat Investigation");
    let mut panel = panel;
    panel.set_list_options("Investigation", Default::default());
    assert_eq!(model::quests(&panel, 0, false).rows.len(), 1);
    assert!(projection.detail.contains("Return to the island guide."));
    Ok(())
}
#[test]
fn server_info_spell_reply_reaches_bevy_detail_and_cooldown_bar() -> Result {
    let (host, mut commands, outbox) = super::tests::host()?;
    let link = &host.play.as_ref().ok_or("play")?.link;
    let spell = oteryn_session::spell_catalog::SpellCatalogEntry {
        cast_index: 7,
        name: "Light Healing".into(),
        words: "exura".into(),
        level: 8,
        mana: 20,
        mana_percent: false,
        soul: 0,
        learned: true,
        available: true,
        cooldown_groups: vec![2],
        rune_item: None,
    };
    outbox.bevy_journal_catalogue(
        oteryn_protocol_oteryn::actor_spell_catalog::encode_catalog(&[spell])
            .map_err(|e| format!("{e:?}"))?,
    )?;
    outbox.bevy_server_info_initialize();
    let query = Query::Spells(vec![7]);
    link.server_info_request(query.clone());
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("spell info request".into());
    };
    assert_eq!(Query::decode(&command.payload), Some(query.clone()));
    let info = SpellInfo {
        cast_index: 7,
        description: Some("Heals the caster.".into()),
        cooldown_ms: 1000,
        group_cooldowns_ms: vec![(2, 1000)],
    };
    outbox.bevy_server_info_result(
        &query,
        &result(Reply::Spells(vec![info]).encode().ok_or("spell encode")?),
    );
    let panel = link.spell_catalog_panel();
    let detail = model::project(
        Tab::Spells,
        &link.quest_panel(),
        &Default::default(),
        &panel,
        Some(7),
        0,
    )
    .detail;
    assert!(detail.contains("Heals the caster."));
    assert!(detail.contains("Base spell cooldown: 1000 ms"));
    assert!(detail.contains("Base group 2 cooldown: 1000 ms"));
    assert!(
        link.cooldown_bar_icons()
            .iter()
            .any(|i| i.group && i.id == 2 && i.tooltip.contains("1000"))
    );
    Ok(())
}

#[test]
fn server_info_unsolicited_quest_revision_refreshes_titles_once() -> Result {
    use oteryn_client_ui::quest_log::{QuestListEntry, QuestLogView};
    let (host, _commands, outbox) = super::tests::host()?;
    let link = &host.play.as_ref().ok_or("play")?.link;
    let mut state = super::tests::quest_state();
    outbox.bevy_journal_quest_state(state.clone());
    assert!(
        outbox
            .bevy_server_info_quest_observation_command()
            .is_none()
    );
    outbox.bevy_server_info_initialize();
    let command = outbox
        .bevy_server_info_quest_observation_command()
        .ok_or("initial titles")?;
    assert_eq!(Query::decode(&command.payload), Some(Query::Quests));
    outbox.bevy_server_info_result(
        &Query::Quests,
        &result(
            Reply::Quests(vec![(73, "Rat Investigation".into())])
                .encode()
                .ok_or("encode")?,
        ),
    );
    assert!(
        outbox
            .bevy_server_info_quest_observation_command()
            .is_none()
    );
    state.revision = Some(2);
    state.list.push(QuestListEntry {
        quest_index: 74,
        completed: false,
    });
    state.log.view = Some(QuestLogView::List(state.list.clone()));
    // A pushed revision, without quest_request or another user command.
    outbox.bevy_journal_quest_state(state);
    let command = outbox
        .bevy_server_info_quest_observation_command()
        .ok_or("pushed titles")?;
    assert_eq!(Query::decode(&command.payload), Some(Query::Quests));
    outbox.bevy_server_info_result(
        &Query::Quests,
        &result(
            Reply::Quests(vec![
                (73, "Rat Investigation".into()),
                (74, "A New Trail".into()),
            ])
            .encode()
            .ok_or("encode")?,
        ),
    );
    let projection = model::quests(&link.quest_panel(), 0, false);
    assert!(
        projection
            .rows
            .iter()
            .any(|r| r.id == model::RowId::Quest(74) && r.label == "A New Trail")
    );
    assert!(
        outbox
            .bevy_server_info_quest_observation_command()
            .is_none()
    );
    Ok(())
}
