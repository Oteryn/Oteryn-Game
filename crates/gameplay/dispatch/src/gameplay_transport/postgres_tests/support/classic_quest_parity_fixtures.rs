use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, decode_use_result,
    encode_item_use_intent, encode_use_intent,
};
use std::num::NonZeroU64;

pub(super) const NPC_CASE: &str = "qengine-rewards-classic-exchanges";
pub(super) const ACCESS_PREFIX: &str = "classic-quest-access/";
const MARID_LAMP: &str = "marid-lamp";

#[derive(Clone, Copy)]
pub(super) struct Access {
    pub name: &'static str,
    pub source: Position,
    pub start: Position,
    pub tool: Option<(u16, Option<i64>)>,
    pub ordinal: u8,
    pub destination: Position,
}

impl Access {
    const fn dig(
        name: &'static str,
        x: u16,
        y: u16,
        floor: i8,
        start_x: u16,
        tool: (u16, i64),
    ) -> Self {
        Self {
            name,
            source: Position { x, y, floor },
            start: Position {
                x: start_x,
                y,
                floor,
            },
            tool: Some((tool.0, Some(tool.1))),
            ordinal: 0,
            destination: Position {
                x,
                y,
                floor: floor - 1,
            },
        }
    }
}

pub(super) const ACCESS: [Access; 10] = [
    Access::dig("present-pick", 32149, 32110, -11, 32148, (3456, 394)),
    Access::dig("silver-brooch-pick", 32775, 32012, -10, 32774, (3456, 394)),
    Access::dig(
        "spike-sword-first-pick",
        32595,
        32068,
        -9,
        32594,
        (3456, 394),
    ),
    Access::dig(
        "spike-sword-second-pick",
        32580,
        32103,
        -11,
        32579,
        (3456, 394),
    ),
    Access::dig("scale-armor-shovel", 32335, 32174, -7, 32334, (3457, 594)),
    Access::dig("steel-helmet-shovel", 32489, 31965, -7, 32490, (3457, 594)),
    Access {
        name: "steel-helmet-rope",
        source: Position {
            x: 32466,
            y: 31966,
            floor: -8,
        },
        start: Position {
            x: 32465,
            y: 31966,
            floor: -8,
        },
        tool: Some((3003, None)),
        ordinal: 0,
        destination: Position {
            x: 32466,
            y: 31967,
            floor: -7,
        },
    },
    Access {
        name: "scale-armor-well-32353",
        source: Position {
            x: 32353,
            y: 32131,
            floor: -8,
        },
        start: Position {
            x: 32353,
            y: 32130,
            floor: -8,
        },
        tool: None,
        ordinal: 1,
        destination: Position {
            x: 32354,
            y: 32131,
            floor: -9,
        },
    },
    Access {
        name: "scale-armor-well-32354",
        source: Position {
            x: 32354,
            y: 32131,
            floor: -8,
        },
        start: Position {
            x: 32355,
            y: 32131,
            floor: -8,
        },
        tool: None,
        ordinal: 1,
        destination: Position {
            x: 32355,
            y: 32131,
            floor: -9,
        },
    },
    Access {
        name: MARID_LAMP,
        source: Position {
            x: 33048,
            y: 32630,
            floor: -1,
        },
        start: Position {
            x: 33048,
            y: 32631,
            floor: -1,
        },
        tool: None,
        ordinal: 2,
        destination: Position {
            x: 33048,
            y: 32631,
            floor: -1,
        },
    },
];

pub(super) fn access(name: &str) -> Option<Access> {
    let name = name.strip_prefix(ACCESS_PREFIX)?;
    ACCESS.into_iter().find(|case| case.name == name)
}

pub(super) async fn access_exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    case: Access,
) -> TestResult {
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("terrain tool capabilities")?;
    let source = case.source;
    let map = owner.bundle_map().ok_or("map")?;
    let key =
        oteryn_world_bundle::bundle::placement_key(source.floor, source.x, source.y, case.ordinal)
            .ok_or("source placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    };
    let command = admitted.continuity.next_command_id;
    if case.name == MARID_LAMP {
        let mission = "oteryn:quest-progress/quest/u7_4/djinn_war/marid_faction/mission03";
        super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
            h,
            owner,
            admitted,
            &[(mission.to_owned(), 1)],
            123,
        )
        .await?;
        supply(
            owner,
            admitted,
            "oteryn:item.tibia.i3231",
            case.start,
            19023,
            1,
        )
        .await?;
    }
    if let Some((tool, appearance)) = case.tool {
        let item = format!("oteryn:item.tibia.i{tool}");
        supply(owner, admitted, &item, source, 19023, 1).await?;
        let definition = owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .and_then(|n| n.item_index())
            .and_then(|i| i.definition_ref("Item", &item, "definition-r1"))
            .ok_or("tool definition")?;
        let payload = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(NonZeroU64::MIN),
            use_with: None,
            use_with_position: None,
            use_with_map_item: Some(MapUseTarget::Object(target)),
        })
        .map_err(debug)?;
        let (next, result, _) = super::rust_lottery_fixtures::exchange(
            owner,
            *admitted,
            command,
            &payload,
            Some(definition),
            None,
        )
        .await?;
        *admitted = next;
        assert_eq!(result, UseDisposition::Committed, "{}", case.name);
        if let Some(appearance) = appearance {
            let state: i64 =
                sqlx::query_scalar("SELECT state_item_id FROM game_terrain_tool_transforms")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(state, appearance, "{}", case.name);
            let direction = if case.start.x < source.x {
                oteryn_protocol_oteryn::world_spatial::StepDirection::East
            } else {
                oteryn_protocol_oteryn::world_spatial::StepDirection::West
            };
            super::questpc7_ferumbras_fixtures::step(owner, admitted, direction).await?;
        }
    } else {
        let reply = super::quest_world_callbacks_fixtures::wire(
            owner,
            admitted,
            command,
            2,
            encode_use_intent(&target).map_err(debug)?,
        )
        .await?;
        assert_eq!(
            decode_use_result(&reply).map_err(debug)?,
            UseDisposition::Committed,
            "{}",
            case.name
        );
    }
    let actor = admitted.runtime_actor.ok_or("actor")?;
    if case.name == MARID_LAMP {
        assert_eq!(quantity(h, 3231).await?, 0);
        assert_eq!(quantity(h, 3243).await?, 1);
    }
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        super::quest_world_callbacks_fixtures::local(case.destination),
        "{}",
        case.name
    );
    Ok(())
}

pub(super) async fn supply(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    item: &str,
    position: Position,
    command: u64,
    count: u16,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks_fixtures::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position,
            item: item.into(),
            count,
        },
        command,
        true,
    )
    .await
}

pub(super) async fn npc_exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use super::qengine_rewards_fixtures::{talk, wire_conversation};
    let mut selected = admitted.continuity.selected_capabilities.as_slice().to_vec();
    selected.extend([3, 6, 25]);
    selected.sort_unstable();
    selected.dedup();
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &selected,
        )
        .ok_or("NPC conversation capabilities")?;
    let [amber, seymour, willie] = super::classic_quests_fixtures::install(
        owner,
        admitted,
        [
            "oteryn:npc.amber",
            "oteryn:npc.seymour",
            "oteryn:npc.willie",
        ],
    )
    .await?;
    let source = Position {
        x: 33293,
        y: 31677,
        floor: -15,
    };
    let actor = admitted.runtime_actor.ok_or("actor")?;
    assert_eq!(
        talk(owner, admitted, seymour, 18000, "hi").await?,
        NpcIntentDisposition::Ok
    );
    for (level, xp, phrase) in [(3_i64, 200_i64, "killing rats"), (4, 400, "little present")] {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        let updated = sqlx::query(
            "UPDATE game_character_progression_state SET level=$1,total_experience=$2 \
             WHERE character_id=encode($3,'hex')::uuid",
        )
        .bind(level)
        .bind(xp)
        .bind(super::fixture::id(41).as_slice())
        .execute(&mut *tx)
        .await?;
        assert_eq!(updated.rows_affected(), 1);
        tx.commit().await?;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let reply = owner
            .npc_talk(
                actor,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: 18000 + u64::try_from(level)?,
                    item_fence: Some(fence.item_fence()),
                },
                NpcTalkIntent {
                    npc_actor: seymour,
                    text: "quest".into(),
                },
            )
            .await;
        let lines = reply.conversation.ok_or("Seymour mission reply")?.lines;
        assert!(lines.iter().any(|line| line.contains(phrase)), "{lines:?}");
    }
    talk(owner, admitted, seymour, 18005, "bye").await?;
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    for (npc, key, keyword, input, output, command) in [
        (
            amber,
            "classic-amber-notebook",
            "notebook",
            2821,
            3294,
            19020,
        ),
        (willie, "classic-willie-banana", "banana", 3587, 3426, 19021),
    ] {
        let callback = callbacks
            .iter()
            .find(|c| c.key.ends_with(key))
            .ok_or("exchange")?;
        let words = || vec!["hi", keyword, "yes"];
        assert_eq!(
            wire_conversation(owner, admitted, npc, callback, false, words())
                .await?
                .0,
            NpcIntentDisposition::Rejected
        );
        supply(
            owner,
            admitted,
            &format!("oteryn:item.tibia.i{input}"),
            source,
            command,
            1,
        )
        .await?;
        assert_eq!(
            wire_conversation(owner, admitted, npc, callback, false, words())
                .await?
                .0,
            NpcIntentDisposition::Ok
        );
        assert_eq!(quantity(h, input).await?, 0);
        assert_eq!(quantity(h, output).await?, 1);
        assert_eq!(
            wire_conversation(owner, admitted, npc, callback, false, words())
                .await?
                .0,
            NpcIntentDisposition::Rejected
        );
        assert_eq!(quantity(h, output).await?, 1);
    }
    Ok(())
}

async fn quantity(h: &Harness, item: u32) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
        .bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}
