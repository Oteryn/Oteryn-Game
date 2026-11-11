//! Four source claims, real filled-container MINT, exact replay and source exit.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::debug};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{kill_objectives as rules, world_callbacks::Position};
use oteryn_protocol_oteryn::world_object::*;

const PREFIX: &str = "oteryn:reward-claim.quest.u8_1.to_blind_the_enemy.";

#[test]
fn elvenbane_source_treasure_exit_and_journal_via_wire() -> TestResult {
    super::annihilator_route::elvenbane_source_complete()
}

fn target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    x: u16,
    y: u16,
) -> TestResult<WorldObjectTarget> {
    target_at(owner, x, y, -3)
}

fn target_at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    x: u16,
    y: u16,
    floor: i8,
) -> TestResult<WorldObjectTarget> {
    let map = owner.bundle_map().ok_or("source map")?;
    let key = crate::map::view::placement_key(crate::map::overlay::TilePos { x, y, floor }, 1)
        .ok_or("source chest")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
}

pub(super) async fn complete(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    member: &mut AdmittedSession,
    pool: &sqlx::PgPool,
) -> TestResult {
    let started = std::time::Instant::now();
    let version: i32 = sqlx::query_scalar("SELECT current_setting('server_version_num')::int")
        .fetch_one(pool)
        .await?;
    eprintln!("ELVENBANE_TREASURE POSTGRES_CASE_OBSERVED server_version_num={version}");
    let fence = owner
        .current_quest_fence(member.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("session")?;
    // An earned return without source claims must not finish or close the cohort.
    exit(owner, member).await?;
    assert_incomplete(owner, member, pool).await?;
    return_to_chests(owner, member).await?;
    let mut keys = Vec::new();
    for (suffix, x, y, approach_x, approach_y) in [
        ("bp1", 32588, 31645, 32589, 31645),
        ("bp2", 32588, 31644, 32589, 31644),
        ("dwarven_shield", 32591, 31647, 32591, 31646),
        ("morning_star", 32590, 31647, 32590, 31646),
    ] {
        super::desert_earned_route::walk_to(
            owner,
            member,
            Position {
                x: approach_x,
                y: approach_y,
                floor: -3,
            },
        )
        .await?;
        let chest = target(owner, x, y)?;
        let command = member.continuity.next_command_id;
        assert_eq!(
            super::annihilator_route::use_target(owner, member, &chest).await?,
            UseDisposition::Committed,
            "actual source chest {suffix} must atomically mint its reward"
        );
        let replay = super::quest_world_callbacks::wire(
            owner,
            member,
            command,
            2,
            encode_use_intent(&chest).map_err(debug)?,
        )
        .await?;
        assert_eq!(
            decode_use_result(&replay).map_err(debug)?,
            UseDisposition::Committed,
            "literal source claim command replay {suffix}"
        );
        assert_eq!(
            super::annihilator_route::use_target(owner, member, &chest).await?,
            UseDisposition::NothingToUse,
            "fresh command cannot repeat source once claim {suffix}"
        );
        keys.push(format!("{PREFIX}{suffix}"));
        if keys.len() == 3 {
            exit(owner, member).await?;
            assert_incomplete(owner, member, pool).await?;
            return_to_chests(owner, member).await?;
        }
    }
    let roots: Vec<(String,String,i64,i64,bool)> = sqlx::query_as("SELECT m.claim_production_key,i.definition_production_key,i.quantity,jsonb_array_length(r.contents_plan)::bigint,i.xmin::text=m.xmin::text FROM game_reward_claim_mint_receipts m JOIN game_reward_claim_mint_reservations r USING(game_session_id,command_id) JOIN game_item_instances i ON i.item_instance_id=m.item_instance_id WHERE m.character_id=encode($1,'hex')::uuid AND m.claim_production_key=ANY($2::text[]) ORDER BY m.claim_production_key")
        .bind(fence.character_id.as_bytes().as_slice()).bind(&keys).fetch_all(pool).await?;
    let expected = [
        ("bp1", 2853, 3),
        ("bp2", 2853, 2),
        ("dwarven_shield", 3425, 0),
        ("morning_star", 3282, 0),
    ]
    .into_iter()
    .map(|(suffix, item, children)| {
        (
            format!("{PREFIX}{suffix}"),
            format!("oteryn:item.tibia.i{item}"),
            1,
            children,
            true,
        )
    })
    .collect::<Vec<_>>();
    assert_eq!(
        roots, expected,
        "four physical source claim roots and atomic receipts"
    );
    let children: Vec<(String,String,i64,bool)> = sqlx::query_as("SELECT m.claim_production_key,i.definition_production_key,i.quantity,i.xmin::text=m.xmin::text AND e.xmin::text=m.xmin::text FROM game_reward_claim_mint_receipts m JOIN game_item_nested_entries e ON e.parent_item_instance_id=m.item_instance_id JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id WHERE m.character_id=encode($1,'hex')::uuid AND m.claim_production_key=ANY($2::text[]) ORDER BY m.claim_production_key,e.placement_ordinal")
        .bind(fence.character_id.as_bytes().as_slice()).bind(&keys).fetch_all(pool).await?;
    let expected = [
        ("bp1", 237, 1),
        ("bp1", 3147, 1),
        ("bp1", 3059, 1),
        ("bp2", 3028, 2),
        ("bp2", 3031, 100),
    ]
    .into_iter()
    .map(|(suffix, item, quantity)| {
        (
            format!("{PREFIX}{suffix}"),
            format!("oteryn:item.tibia.i{item}"),
            quantity,
            true,
        )
    })
    .collect::<Vec<_>>();
    assert_eq!(
        children, expected,
        "both filled source bags own all five source children in each root's physical transaction"
    );
    exit(owner, member).await?;
    let phase: Option<i64> = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(fence.character_id.as_bytes().as_slice()).bind(rules::PHASE).fetch_optional(pool).await?;
    assert_eq!(
        phase,
        Some(4),
        "earned treasure and actual source exit must complete the recipe"
    );
    let observation = owner
        .quest_log_observation(
            member.runtime_actor.ok_or("actor")?,
            member.game_session_id,
            None,
        )
        .await;
    let crate::gameplay_transport::quest_log::QuestLogObservation::Changed(source) = observation
    else {
        return Err("journal unavailable".into());
    };
    assert!(
        source
            .copy
            .quest_state(rules::QUEST)
            .is_some_and(|state| state.completed),
        "actual admitted journal copy must show Elvenbane completed"
    );
    journal_via_wire(owner, member).await?;
    eprintln!(
        "ELVENBANE_TREASURE POSTGRES_CASE_COMPLETED roots=4 containers=2 nested_children=5 once_replay=proven exit=earned journal=completed elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

async fn journal_via_wire(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    member: &AdmittedSession,
) -> TestResult {
    use oteryn_protocol_oteryn::quest_log::{
        QuestLogQuery, QuestLogView, decode_quest_log, encode_quest_log_query,
    };
    use oteryn_protocol_oteryn::{MessageType, decode_state_delta, decode_wire_envelope};
    let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let loaded = oteryn_gameplay_quests::quest::log::loader::load_embedded_quest_log(&lowered)?;
    let quest_index = u32::try_from(
        loaded.content.active().quests().iter()
            .position(|quest| quest.key == rules::QUEST).ok_or("Elvenbane journal entry")?,
    )?;
    let mut framed = Vec::new();
    for (offset, query) in [QuestLogQuery::List, QuestLogQuery::Quest { quest_index }]
        .into_iter().enumerate()
    {
        let payload = encode_quest_log_query(&query).map_err(debug)?;
        let command = oteryn_protocol_oteryn::encode_client_command(
            1,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id: member.continuity.next_command_id
                    .checked_add(u64::try_from(offset)?).ok_or("journal command overflow")?,
                command_type: 22,
                payload: &payload,
            },
        ).map_err(debug)?;
        framed.extend(u32::try_from(command.len())?.to_be_bytes());
        framed.extend(command);
    }
    let (_, output) = super::weather_wire::fixture_access::serve_input(owner, *member, &framed).await?;
    let mut reader = output.as_slice();
    let mut views = Vec::new();
    let mut accepted = 0;
    while !reader.is_empty() {
        let frame = crate::gameplay_transport::tcp_tls::read_frame(&mut reader).await?;
        let envelope = decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == MessageType::CommandResult {
            let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(result.status, oteryn_protocol_oteryn::CommandStatus::Accepted);
            accepted += 1;
        } else if envelope.message_type() == MessageType::StateDelta {
            let delta = decode_state_delta(envelope.payload()).map_err(debug)?;
            if delta.domain_id == 16 {
                views.push(decode_quest_log(delta.payload).map_err(debug)?.view);
            }
        }
    }
    assert_eq!(accepted, 2, "actual list and quest-line wire queries must be accepted");
    assert!(views.iter().any(|view| matches!(view, Some(QuestLogView::List(quests))
        if quests.iter().any(|quest| quest.quest_index == quest_index && quest.completed))));
    assert!(views.iter().any(|view| matches!(view, Some(QuestLogView::Line(line))
        if line.quest_index == quest_index && line.completed && line.missions.len() == 5
            && line.missions.iter().all(|mission| mission.done)
            && line.missions.iter().any(|mission| mission.text.contains("returned from the fortress")))));
    Ok(())
}

async fn exit(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    member: &mut AdmittedSession,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::StepDirection::{North, South};
    for (floor, x, y, direction) in [
        (3, 32588, 31642, South),
        (4, 32593, 31644, North),
        (5, 32588, 31642, South),
        (6, 32593, 31642, South),
    ] {
        super::desert_earned_route::walk_to(
            owner,
            member,
            Position {
                x,
                y,
                floor: -floor,
            },
        )
        .await?;
        super::desert_earned_route::step(owner, member, direction).await?;
        let at = owner
            .runtime
            .lock()
            .await
            .read_actor_position(member.runtime_actor.ok_or("actor")?)
            .map_err(debug)?
            .position();
        assert_eq!(
            at.floor,
            i16::from(floor) + 1,
            "native source descent {floor}"
        );
    }
    super::desert_earned_route::walk_to(
        owner,
        member,
        Position {
            x: 32597,
            y: 31642,
            floor: -7,
        },
    )
    .await?;
    Ok(())
}

async fn assert_incomplete(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    member: &AdmittedSession,
    pool: &sqlx::PgPool,
) -> TestResult {
    let fence = owner
        .current_quest_fence(member.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("session")?;
    let progress:Vec<(String,i64)>=sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=ANY($2::text[]) ORDER BY track_key")
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(vec![rules::PHASE,"oteryn:quest-progress/elvenbane/native-defenders/phase"])
        .fetch_all(pool).await?;
    assert_eq!(
        progress,
        vec![
            (
                "oteryn:quest-progress/elvenbane/native-defenders/phase".into(),
                0
            ),
            (rules::PHASE.into(), 3)
        ],
        "zero or partial source claims cannot complete either phase"
    );
    Ok(())
}

async fn return_to_chests(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    member: &mut AdmittedSession,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::StepDirection::{East, North};
    for (floor, x, y) in [(7, 32593, 31644), (6, 32588, 31644)] {
        super::desert_earned_route::walk_to(
            owner,
            member,
            Position {
                x,
                y,
                floor: -floor,
            },
        )
        .await?;
        super::desert_earned_route::step(owner, member, North).await?;
    }
    super::desert_earned_route::walk_to(
        owner,
        member,
        Position {
            x: 32593,
            y: 31644,
            floor: -5,
        },
    )
    .await?;
    let ladder = target_at(owner, 32593, 31643, -5)?;
    assert_eq!(
        super::annihilator_route::use_target(owner, member, &ladder).await?,
        UseDisposition::Committed
    );
    super::desert_earned_route::walk_to(
        owner,
        member,
        Position {
            x: 32588,
            y: 31644,
            floor: -4,
        },
    )
    .await?;
    super::desert_earned_route::step(owner, member, North).await?;
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(member.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    assert_eq!(
        at.floor, 3,
        "actual source re-ascent after refused completion"
    );
    // Move off the landing before the flat path search can choose the down stair.
    super::desert_earned_route::step(owner, member, East).await?;
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(member.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    assert_eq!(
        (at.x, at.y, at.floor),
        (32589, 31642, 3),
        "actual safe source approach to the treasure room"
    );
    Ok(())
}
