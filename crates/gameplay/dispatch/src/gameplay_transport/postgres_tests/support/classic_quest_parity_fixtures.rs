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
pub(super) const ICE_DELIVERY: [&str; 2] = [
    "explorer-society-ice-delivery-1",
    "explorer-society-ice-delivery-2",
];

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
        let start = Position {
            x: start_x,
            y,
            floor,
        };
        Self {
            name,
            source: Position { x, y, floor },
            start,
            tool: Some((tool.0, Some(tool.1))),
            ordinal: 0,
            destination: start,
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
    if let Some(&key) = ICE_DELIVERY.iter().find(|&&key| key == name) {
        let callback = super::quest_world_callbacks_fixtures::callback(key).ok()?;
        let selected = callback.selector.native?;
        let start = if key == ICE_DELIVERY[0] {
            Position { y: selected.position.y - 1, ..selected.position }
        } else {
            Position { x: selected.position.x - 1, ..selected.position }
        };
        let item = callback.item_on_target?.item;
        return Some(Access {
            name: key,
            source: selected.position,
            start,
            tool: Some((item.strip_prefix("oteryn:item.tibia.i")?.parse().ok()?, None)),
            ordinal: selected.ordinal,
            destination: start,
        });
    }
    let name = name.strip_prefix(ACCESS_PREFIX)?;
    ACCESS.into_iter().find(|case| case.name == name)
}

pub(super) fn ice_delivery_world(
    native: &crate::content::native_gameplay::NativeGameplayState,
    access: Access,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups_fixtures::{NativeCrop, native_crop};
    let bytes: &[u8] = if access.name == ICE_DELIVERY[0] {
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z09-x125-y123.b3"))
    } else {
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z10-x132-y124.b3"))
    };
    let source = access.source;
    native_crop(native, NativeCrop {
        bytes,
        region: (source.floor.unsigned_abs(), source.x / 256, source.y / 256),
        bounds: (source.x - 1, source.y - 1, source.x + 1, source.y + 1),
        start: (access.start.x, access.start.y),
        expected: Vec::new(),
    })
}

pub(super) async fn access_exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    case: Access,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::StepDirection;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("terrain tool capabilities")?;
    let source = case.source;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    if case.name == "present-pick" {
        super::questpc7_ferumbras_fixtures::step(owner, admitted, StepDirection::East).await?;
        assert_eq!(
            owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(),
            super::quest_world_callbacks_fixtures::local(source),
            "the closed floor does not descend"
        );
        super::questpc7_ferumbras_fixtures::step(owner, admitted, StepDirection::West).await?;
    }
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
    let ice_delivery = ICE_DELIVERY.contains(&case.name)
        .then(|| super::quest_world_callbacks_fixtures::callback(case.name))
        .transpose()?;
    if let Some(callback) = &ice_delivery {
        let stage = callback.storage.iter()
            .map(|track| (track.track.clone(), track.minimum)).collect::<Vec<_>>();
        super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
            h, owner, admitted, &stage, 123,
        ).await?;
        owner.refresh_quest_session(admitted.game_session_id, true).await;
    }
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
            let tile = crate::map::overlay::TilePos { x: source.x, y: source.y, floor: source.floor };
            let base = map.overlay.base().tile(source.x, source.y, source.floor).ok_or("dug tile")?;
            let ground = *base.ids().first().ok_or("dug ground")?;
            let entry = map.facts.base_entry(tile, case.ordinal, ground).ok_or("dug entry")?;
            assert_eq!(i64::from(entry.appearance_id), appearance, "{}", case.name);
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
    if let Some(callback) = &ice_delivery {
        let retained: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_quest_world_tool_reads r JOIN game_item_instances i
             USING(item_instance_id) WHERE r.transition_key=$1 AND i.lifecycle=1 AND i.quantity=1",
        ).bind(&callback.key).fetch_one(&h.pool).await?;
        assert_eq!(retained, 1);
        let reward = callback.item_reward().as_ref().ok_or("icicle reward")?;
        let sample: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1",
        ).bind(&reward.item).fetch_one(&h.pool).await?;
        assert_eq!(sample, 1);
        let (duration, from_mint, terminal): (i64, i64, bool) = sqlx::query_as(
            "SELECT p.duration_millis,p.expires_at_unix_ms-r.occurred_at,p.target_family IS NULL
             FROM game_spell_item_temporal_schedules p
             JOIN game_reward_claim_mint_receipts r ON r.transaction_id=p.source_transaction_id
             WHERE p.definition_production_key=$1",
        )
        .bind(&reward.item)
        .fetch_one(&h.pool)
        .await?;
        let native = owner
            .active_generation
            .and_then(|generation| generation.native_gameplay())
            .ok_or("native")?;
        let policy = native
            .item_policy(&reward.item, "definition-r1")
            .ok_or("sample policy")?;
        use oteryn_gameplay_content::content::{ReferenceItemField::Known, ReferenceMilliseconds};
        let Known(temporal) = &policy.record().semantics.temporal else {
            return Err("missing native sample timer".into());
        };
        let Known(ReferenceMilliseconds(expected)) = &temporal.duration else {
            return Err("missing native sample duration".into());
        };
        let expected = i64::try_from(*expected)?;
        assert_eq!((duration, from_mint, terminal), (expected, expected, true));
        for track in &callback.storage {
            let value: i64 = sqlx::query_scalar(
                "SELECT value FROM game_character_quest_tracks WHERE track_key=$1",
            ).bind(&track.track).fetch_one(&h.pool).await?;
            assert_eq!(value, track.set);
        }
        expire_deposited_sample(owner, admitted, h, &reward.item).await?;
    }
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

async fn expire_deposited_sample(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    sample_key: &str,
) -> TestResult {
    use crate::durability::item_loop_move::DepotMove;
    use crate::foundation::{CommandId, CommandRef};
    use crate::gameplay_transport::item_ref_admission::NativeItemWeights;

    let sample: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(item_instance_id) FROM game_spell_item_temporal_schedules
         WHERE definition_production_key=$1",
    )
    .bind(sample_key)
    .fetch_one(&h.pool)
    .await?;
    let item = sample.as_slice().try_into()?;
    let session = admitted.game_session_id;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("ice sample fence")?
        .item_fence();
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native item generation")?;
    let access = DepotMove {
        town: 1,
        box_number: 1,
        deposit: true,
        request_digest: [1; 32],
        premium: owner.session_premium_gate(session).await,
    };
    let command = CommandId::new(admitted.continuity.next_command_id).map_err(debug)?;
    let permit = owner.spell_lane_permit().await.ok_or("Item writer lane")?;
    owner
        .root
        .commit_depot_move_at(
            &permit,
            owner.character,
            owner.holder,
            &fence,
            CommandRef::new(session, command),
            &NativeItemWeights(native),
            access,
            item,
            None,
        )
        .await
        .map_err(debug)?
        .map_err(debug)?;
    drop(permit);
    let browse = || {
        owner.root.read_depot_page(
            owner.character,
            fence.character_id,
            access.town,
            access.box_number,
            0,
        )
    };
    let fresh = browse().await?;
    assert_eq!(fresh.total, 1);
    assert_eq!(
        fresh.entries.first().ok_or("fresh sample")?.item.item_instance_id,
        item
    );

    // Advance this fixture's deadline without waiting for its real lifetime.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE game_spell_item_temporal_schedules
         SET expires_at_unix_ms=floor(extract(epoch FROM statement_timestamp())*1000)::bigint-1
         WHERE uuid_send(item_instance_id)=$1",
    )
    .bind(&sample)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    owner
        .states
        .spell_states
        .lock()
        .await
        .next_item_deadline_pass_us = 0;
    owner.drain_source_item_deadlines().await;
    let expired = browse().await?;
    assert_eq!(expired.total, 0);
    assert!(expired.entries.is_empty());
    let retired: (i16, i64, i16) = sqlx::query_as(
        "SELECT i.lifecycle,i.quantity,r.custody_kind FROM game_item_instances i
         JOIN game_spell_item_temporal_receipts r USING(item_instance_id)
         WHERE uuid_send(i.item_instance_id)=$1",
    )
    .bind(sample)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(retired, (2, 0, 5));
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
    selected.extend([
        oteryn_protocol_oteryn::npc_service::CAPABILITY_NPC_SERVICE_V1,
        oteryn_protocol_oteryn::world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES,
    ]);
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
    let original: (i64, i64) = sqlx::query_as(
        "SELECT level,total_experience FROM game_character_progression_state \
         WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(super::fixture::id(41).as_slice())
    .fetch_one(&h.pool)
    .await?;
    for (level, xp, phrase) in [(3_i64, 200_i64, "killing rats"), (4, 400, "little present")] {
        set_test_level(h, level, xp).await?;
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
    // Restore the receipt-backed values before either exchange writes Character state.
    set_test_level(h, original.0, original.1).await?;
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

pub(super) async fn set_test_level(h: &Harness, level: i64, experience: i64) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let updated = sqlx::query(
        "UPDATE game_character_progression_state SET level=$1,total_experience=$2 \
         WHERE character_id=encode($3,'hex')::uuid",
    )
    .bind(level)
    .bind(experience)
    .bind(super::fixture::id(41).as_slice())
    .execute(&mut *tx)
    .await?;
    assert_eq!(updated.rows_affected(), 1);
    tx.commit().await?;
    Ok(())
}

async fn quantity(h: &Harness, item: u32) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
        .bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}
