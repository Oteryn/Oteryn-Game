use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{PlayView, play_channel},
};
use oteryn_client_runtime::{
    quest_log::{
        QuestLine, QuestListEntry, QuestLog, QuestLogQuery, QuestLogView, QuestMission, QuestState,
        decode_quest_log_query,
    },
    spell_catalog::spellbook_details::{DetailedSpell, Page, Query as SpellQuery},
};
use oteryn_renderer::TileCoord;
use oteryn_session::{CommandStatus, GatedCommandResult};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
fn quest_state() -> QuestState {
    QuestState {
        selected: true,
        revision: Some(1),
        log: QuestLog {
            view: Some(QuestLogView::Line(QuestLine {
                quest_index: 73,
                completed: false,
                missions: vec![QuestMission {
                    mission_index: 1,
                    name: "Find the guide".into(),
                    text: "Return to the island guide.".into(),
                    done: false,
                }],
            })),
            tracked: vec![],
        },
        list: vec![QuestListEntry {
            quest_index: 73,
            completed: false,
        }],
        refusal: None,
    }
}
fn result(payload: Vec<u8>) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload,
    }
}
pub(super) fn host() -> std::result::Result<
    (
        HostOwner,
        tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
        crate::play::Outbox,
    ),
    Box<dyn std::error::Error>,
> {
    let mut host = HostOwner::new(None)?;
    let (link, commands, outbox) = play_channel();
    host.play = Some(super::super::super::owner::ActivePlay {
        view: PlayView::new(TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    host.phase = super::super::super::owner::Phase::Playing;
    Ok((host, commands, outbox))
}
#[test]
fn journal_quest_launcher_selection_and_tracking_reach_existing_owner() -> Result {
    let (mut host, mut commands, outbox) = host()?;
    outbox.bevy_journal_quest_state(quest_state());
    open_quests(&mut host);
    assert!(
        host.play.as_ref().ok_or("play")?.link.quest_panel().open,
        "quest log owner must be opened"
    );
    let PlayCommand::QuestLog(payload) = commands.try_recv()? else {
        return Err("quest request missing".into());
    };
    assert_eq!(
        decode_quest_log_query(&payload).map_err(|_| "quest query")?,
        QuestLogQuery::List
    );
    outbox.bevy_journal_quest_result(&result(vec![]));
    let key = Key {
        generation: host.generation(),
        tab: Tab::Quests,
    };
    let mut state = State {
        key: Some(key),
        ..default()
    };
    let values = actions::Values::default();
    assert!(!actions::perform(
        &mut host,
        &mut state,
        Action {
            key,
            kind: Kind::Mission(9, 1)
        },
        &values
    ));
    assert!(actions::perform(
        &mut host,
        &mut state,
        Action {
            key,
            kind: Kind::Mission(73, 1)
        },
        &values
    ));
    let link = &host.play.as_ref().ok_or("play")?.link;
    assert_eq!(link.quest_panel().mission, Some(1));
    assert!(
        model::project(
            Tab::Quests,
            &link.quest_panel(),
            &link.spellbook_state(),
            &link.spell_catalog_panel(),
            None,
            0
        )
        .detail
        .contains("Return to the island guide.")
    );
    assert!(actions::perform(
        &mut host,
        &mut state,
        Action {
            key,
            kind: Kind::Track
        },
        &values
    ));
    let PlayCommand::QuestLog(payload) = commands.try_recv()? else {
        return Err("tracking request missing".into());
    };
    assert_eq!(
        decode_quest_log_query(&payload).map_err(|_| "quest query")?,
        QuestLogQuery::Track {
            quest_indexes: vec![73]
        }
    );
    assert!(
        host.play
            .as_ref()
            .ok_or("play")?
            .link
            .quest_panel()
            .state
            .log
            .tracked
            .is_empty()
    );
    assert!(!actions::perform(
        &mut host,
        &mut state,
        Action {
            key: Key {
                generation: key.generation + 1,
                ..key
            },
            kind: Kind::Track
        },
        &values
    ));
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn journal_spellbook_launcher_and_filters_use_typed_query_and_confirmed_page() -> Result {
    let (mut host, mut commands, outbox) = host()?;
    outbox.bevy_journal_spellbook_select();
    open_spells(&mut host);
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("spellbook query missing".into());
    };
    assert_eq!(command.capability, 449);
    assert_eq!(command.kind, 733);
    assert_eq!(
        SpellQuery::decode(&command.payload),
        Some(SpellQuery::default())
    );
    let page = Page {
        page: 0,
        total: 1,
        entries: vec![DetailedSpell {
            spell: oteryn_session::spell_catalog::SpellCatalogEntry {
                cast_index: 19,
                name: "Mending".into(),
                words: "mend".into(),
                level: 20,
                mana: 15,
                mana_percent: false,
                soul: 0,
                learned: true,
                available: true,
                cooldown_groups: vec![2],
                rune_item: None,
            },
            vocations: 1 << 9,
            premium: true,
            magic_level: 0,
        }],
    };
    outbox.bevy_journal_spellbook_result(&result(page.encode().ok_or("page encoding")?));
    let key = Key {
        generation: host.generation(),
        tab: Tab::Spells,
    };
    let mut state = State {
        key: Some(key),
        ..default()
    };
    let values = actions::Values {
        search: "mend".into(),
        minimum: "20".into(),
        maximum: "200".into(),
    };
    assert!(actions::perform(
        &mut host,
        &mut state,
        Action {
            key,
            kind: Kind::Search
        },
        &values
    ));
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("filtered query missing".into());
    };
    let query = SpellQuery::decode(&command.payload).ok_or("query")?;
    assert_eq!(query.search, "mend");
    assert_eq!(query.min_level, 20);
    assert_eq!(query.max_level, 200);
    assert_eq!(
        host.play
            .as_ref()
            .ok_or("play")?
            .link
            .spellbook_state()
            .page,
        Some(page.clone())
    );
    let link = &host.play.as_ref().ok_or("play")?.link;
    let projection = model::project(
        Tab::Spells,
        &link.quest_panel(),
        &link.spellbook_state(),
        &link.spell_catalog_panel(),
        None,
        0,
    );
    assert!(projection.rows[0].label.contains("Mending"));
    assert!(projection.detail.contains("mend"));
    let modal = link.bevy_server_modal_fixture();
    assert!(!actions::perform(
        &mut host,
        &mut state,
        Action {
            key,
            kind: Kind::Close
        },
        &values
    ));
    assert!(
        host.play
            .as_ref()
            .ok_or("play")?
            .link
            .spellbook_state()
            .open
    );
    drop(modal);
    assert!(actions::perform(
        &mut host,
        &mut state,
        Action {
            key,
            kind: Kind::Close
        },
        &values
    ));
    assert!(
        !host
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .spellbook_state()
            .open
    );
    assert_eq!(
        host.play
            .as_ref()
            .ok_or("play")?
            .link
            .spellbook_state()
            .page,
        Some(page)
    );
    Ok(())
}
#[test]
fn journal_paging_bounds_large_confirmed_quest_lists_without_sending_commands() -> Result {
    let (mut host, mut commands, outbox) = host()?;
    let mut quests = quest_state();
    quests.list = (1..=1024)
        .map(|id| QuestListEntry {
            quest_index: id,
            completed: false,
        })
        .collect();
    outbox.bevy_journal_quest_state(quests);
    open_quests(&mut host);
    let _ = commands.try_recv()?;
    outbox.bevy_journal_quest_result(&result(vec![]));
    let key = Key {
        generation: host.generation(),
        tab: Tab::Quests,
    };
    let mut state = State {
        key: Some(key),
        quest_page: 31,
        ..default()
    };
    assert!(actions::perform(
        &mut host,
        &mut state,
        Action {
            key,
            kind: Kind::Next
        },
        &actions::Values::default()
    ));
    assert_eq!(state.quest_page, 31);
    let link = &host.play.as_ref().ok_or("play")?.link;
    let projection = model::project(
        Tab::Quests,
        &link.quest_panel(),
        &link.spellbook_state(),
        &link.spell_catalog_panel(),
        None,
        state.quest_page,
    );
    assert_eq!(projection.rows.len(), 32);
    assert_eq!(projection.rows[0].id, model::RowId::Quest(993));
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn journal_fallback_selection_rechecks_the_core_filtered_catalogue() -> Result {
    let (mut host, mut commands, outbox) = host()?;
    let first = oteryn_session::spell_catalog::SpellCatalogEntry {
        cast_index: 19,
        name: "Mending".into(),
        words: "mend".into(),
        level: 20,
        mana: 15,
        mana_percent: false,
        soul: 0,
        learned: true,
        available: true,
        cooldown_groups: vec![2],
        rune_item: None,
    };
    let second = oteryn_session::spell_catalog::SpellCatalogEntry {
        cast_index: 20,
        name: "Flame".into(),
        words: "flame".into(),
        ..first.clone()
    };
    outbox.bevy_journal_catalogue(
        oteryn_session::spell_catalog::encode_catalog(&[first, second])
            .map_err(|_| "catalogue encoding")?,
    )?;
    open_spells(&mut host);
    let key = Key {
        generation: host.generation(),
        tab: Tab::Spells,
    };
    let mut state = State {
        key: Some(key),
        ..default()
    };
    let values = actions::Values {
        search: "mend".into(),
        ..default()
    };
    assert!(actions::perform(
        &mut host,
        &mut state,
        Action {
            key,
            kind: Kind::Search
        },
        &values
    ));
    assert!(!actions::perform(
        &mut host,
        &mut state,
        Action {
            key,
            kind: Kind::Spell(20)
        },
        &values
    ));
    assert!(actions::perform(
        &mut host,
        &mut state,
        Action {
            key,
            kind: Kind::Spell(19)
        },
        &values
    ));
    let catalogue = host.play.as_ref().ok_or("play")?.link.spell_catalog_panel();
    assert_eq!(
        catalogue.selected_spell().map(|entry| entry.cast_index),
        Some(19)
    );
    assert_eq!(catalogue.visible_spells().count(), 1);
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn journal_refusal_keeps_the_error_footer_bounded() -> Result {
    let (mut host, mut commands, outbox) = host()?;
    outbox.bevy_journal_quest_state(quest_state());
    open_quests(&mut host);
    let _ = commands.try_recv()?;
    outbox.bevy_journal_quest_result(&GatedCommandResult {
        status: CommandStatus::Rejected,
        ..result(vec![])
    });
    let link = &host.play.as_ref().ok_or("play")?.link;
    let projection = model::project(
        Tab::Quests,
        &link.quest_panel(),
        &link.spellbook_state(),
        &link.spell_catalog_panel(),
        None,
        0,
    );
    assert_eq!(
        projection.status.lines().count(),
        1,
        "footer must contain one public refusal line"
    );
    assert!(projection.status.contains("refused"));
    assert!(!projection.rows.is_empty());
    Ok(())
}

#[test]
fn journal_builds_both_source_shaped_windows_with_real_text_controls() -> Result {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    app.update();
    let skin = app.world_mut().remove_resource::<Skins>().ok_or("skins")?;
    let quest = oteryn_client_ui::quest_log::QuestPanel {
        state: quest_state(),
        open: true,
        ..default()
    };
    let book = crate::play::feature_0287_actor_spell_catalog::view::SpellbookState::default();
    let catalogue = oteryn_client_ui::spell_catalog::Panel::default();
    for (tab, detailed) in [
        (Tab::Quests, false),
        (Tab::Spells, false),
        (Tab::Spells, true),
    ] {
        let book = crate::play::feature_0287_actor_spell_catalog::view::SpellbookState {
            selected: detailed,
            ..book.clone()
        };
        let mut world = World::new();
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let overlay = commands.spawn_empty().id();
        let built = widgets::build(
            &mut commands,
            &skin,
            overlay,
            Key { generation: 1, tab },
            &quest,
            &book,
            &catalogue,
        );
        queue.apply(&mut world);
        let font = world.get::<TextFont>(built.search).ok_or("search font")?;
        assert_eq!(font.font_size, bevy::text::FontSize::Px(11.));
        let field = world
            .get::<EditableText>(built.search)
            .ok_or("search editor")?;
        assert_eq!(
            field.max_characters,
            Some(if tab == Tab::Quests { 512 } else { 64 })
        );
        assert!(
            world
                .get::<bevy::input_focus::tab_navigation::TabGroup>(built.root)
                .ok_or("modal scope")?
                .modal
        );
    }
    Ok(())
}
