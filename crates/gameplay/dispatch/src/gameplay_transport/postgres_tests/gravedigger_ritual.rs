//! The plain ritual-scroll USE debits both custody inputs and grants the achievement.
use super::quest_world_callbacks::{self, tool_fixture};
use super::{TestResult, fixture::debug};
use oteryn_gameplay_quests::quest::{
    QuestTrack,
    world_callbacks::{Callback, Offering, Position},
};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};

const QUEST: &str = "oteryn:quest.the_gravedigger_of_drefia_quest";
const MISSION_74: &str = "oteryn:quest-progress/quest/u10_10/the_gravedigger_of_drefia/mission74";
const MISSION_75: &str = "oteryn:quest-progress/quest/u10_10/the_gravedigger_of_drefia/mission75";
const SCROLL: &str = "oteryn:item.tibia.i18934";
const SKULL: &str = "oteryn:item.tibia.i19160";
const CHICKEN: &str = "oteryn:creature.chicken";
const ACHIEVEMENT: &str = "oteryn:achievement/gravedigger";

#[test]
fn plain_incantation_scroll_use_consumes_the_altar_skull_and_summons_the_ritual_chicken()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("gravedigger-ritual", move || scenario(admin))
}

fn scenario(admin: String) -> TestResult {
    let altar = Position {
        x: 33015,
        y: 32418,
        floor: -11,
    };
    // Crystal 201189f8 data-global/scripts/quests/the_gravedigger_of_drefia/actions_sacrifice.lua:3-17.
    // Gravedigger ID 326: data/scripts/lib/register_achievements.lua:327; assignment: Fandom archive quest spoiler.
    let callback: Callback = serde_json::from_value(serde_json::json!({
        "key": "oteryn:quest-callback/fixture/right-rite",
        "quest": QUEST, "trigger": "use",
        "selector": { "appearance": 3134, "native": { "position": altar, "appearance": 3134, "ordinal": 4 } },
        "requires_world_admission": true,
        "reversible": false, "passages": [], "return_on_failure": false,
        "storage": [
            { "track": MISSION_74, "minimum": 1, "maximum": 1, "set": 1, "preserve": true },
            { "track": MISSION_75, "minimum": 0, "maximum": 0, "set": 1 }
        ],
        "item_on_target": { "item": SCROLL, "count": 1, "selected_instance": true },
        "offerings": [{ "item": SKULL, "count": 1, "position": altar }],
        "success_message": "The scroll burns to dust. The magic stutters. Omrabas' soul flees to his old hideaway.",
        "achievements": [ACHIEVEMENT],
        "effects": { "direct_item_use": true, "creature_spawn": { "creature": CHICKEN, "position": altar } },
        "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_gravedigger_of_drefia/actions_sacrifice.lua:3-17"]
    }))?;
    let world = quest_world_callbacks::world_with_cells(&callback, altar, &[])?;
    let tracks = [(MISSION_74, 1), (MISSION_75, 0)].map(|(key, initial)| QuestTrack {
        key: key.into(),
        quest: QUEST.into(),
        initial,
        min: 0,
        max: 1,
    });
    tool_fixture::run_world(
        admin,
        "gravedigger_ritual",
        world,
        callback,
        tracks.into(),
        async |owner, mut admitted, h, fence| {
            let item = owner
                .active_generation
                .and_then(|g| g.native_gameplay())
                .and_then(|native| native.item_index())
                .and_then(|index| index.definition_ref("Item", SCROLL, "definition-r1"))
                .ok_or("scroll definition")?;
            admitted.continuity.selected_capabilities =
                crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                    owner.offered_capabilities(),
                    &[4, 6, 11, 15, 18],
                )
                .ok_or("capabilities")?;
            let payload = encode_item_use_intent(&UseIntent {
                target: UseTarget::ItemByDefinition(item),
                use_with: None,
                use_with_map_item: None,
                use_with_position: None,
            })
            .map_err(debug)?;
            quest_world_callbacks::input_item(
                owner,
                fence.item_fence(),
                &Offering {
                    position: altar,
                    item: SCROLL.into(),
                    count: 1,
                },
                49,
                true,
            )
            .await?;
            let (admitted, refusal, _) = super::item_craft::exchange(
                owner,
                admitted,
                admitted.continuity.next_command_id,
                &payload,
                Some(item),
            )
            .await?;
            assert_ne!(refusal, UseDisposition::Committed);
            quest_world_callbacks::input_item(
                owner,
                fence.item_fence(),
                &Offering {
                    position: altar,
                    item: SKULL.into(),
                    count: 1,
                },
                50,
                false,
            )
            .await?;
            let (_, accepted, _) = super::item_craft::exchange(
                owner,
                admitted,
                admitted.continuity.next_command_id,
                &payload,
                Some(item),
            )
            .await?;
            assert_eq!(accepted, UseDisposition::Committed);
            let snapshot = owner
                .root
                .read_character_quest_state(owner.character, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(snapshot.tracks().get(MISSION_75), Some(&1));
            let unspent: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key IN ($1,$2)")
                .bind(SCROLL).bind(SKULL).fetch_one(&h.pool).await?;
            assert_eq!(unspent, 0);
            let achievements: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_account_achievements WHERE achievement_key=$1",
            )
            .bind(ACHIEVEMENT)
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(achievements, 1);
            let runtime = owner.runtime.lock().await;
            let count = runtime
                .current_live_creature_census()
                .map_err(debug)?
                .into_iter()
                .filter(|actor| runtime.matches_live_creature_identity(*actor, CHICKEN.as_bytes()))
                .count();
            assert_eq!(count, 1);
            Ok(())
        },
    )
}
