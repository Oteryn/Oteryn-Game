//! A real admitted lease and live owned summon, driven by the ordinary spawn owner.
#![cfg(feature = "qualification-spawn")]

use crate::bestiary_postgres_harness::{Harness, TestResult, debug, id, runtime, scope};
use crate::foundation::{ChannelRuntimeV1, ControlLossMark, ExactActorRef, MovementLocalPosition};
use crate::node::serve::qualification_forge_population::draw;
use crate::node::{operator_files::hex_bytes, uuid_bytes, uuid_text};
use crate::party_shared_xp_postgres_tests::committed_fresh_player;
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
use oteryn_gameplay_domain::domain::forge_creature::{
    FIENDISH_RESPAWN_MICROS, FIENDISH_SIMULTANEOUS_MAX, ForgeCreature, ForgeSpawnCandidate,
    INFLUENCED_SIMULTANEOUS_MAX, forge_population_roll, influenced_available_slots,
    plan_fiendish_tick, plan_influenced_tick,
};
use oteryn_gameplay_entities::carrier::runtime_actor_companion::{
    CompanionSnapshot, CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags,
};
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;
use tokio::sync::Mutex;

const RAT: &str = "oteryn:creature.rat";
const FIRST_CYCLE: u64 = 10_000_000;

#[test]
fn forge_population_draw_matches_production_selection_and_excludes_live_owned_summon_postgres()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            runtime()
                .map_err(debug)?
                .block_on(async {
                    let harness = Harness::create(admin, "forge_population", true).await?;
                    let result = exercise_draw(&harness).await;
                    harness.cleanup().await?;
                    result
                })
                .map_err(debug)
        })?
        .join()
        .map_err(|_| "Forge population PostgreSQL thread failed")?
        .map_err(Into::into)
}

async fn exercise_draw(harness: &Harness) -> TestResult {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&harness.pool)
            .await?;
    let admission = committed_fresh_player(harness, 60, 62, 70, now).await?;
    let session = admission.current_game_session_id();
    let lease = admission.current_character_lease();
    let (mut channel, seed) = channel(harness)?;
    let reservation = channel.reserve_fresh_session(session)?;
    let player = channel.commit_fresh_session(reservation)?;
    channel.initialize_first_entry_position(player)?;
    let owned = channel.create_companion(
        player,
        session,
        RAT,
        MovementLocalPosition {
            x: 20,
            y: 0,
            floor: 0,
        },
        None,
        None,
        0,
    )?;
    assert_eq!(
        channel
            .companion_snapshot(owned)?
            .state
            .master
            .ok_or("unowned summon")?
            .actor,
        player
    );
    let request = format!(
        "forge-population-draw {} {seed}\n",
        uuid_text(session.as_bytes())
    );
    let missing = format!("forge-population-draw {} {seed}\n", uuid_text(&id(71)));
    let channel = Mutex::new(channel);
    let root = &harness.root;
    assert_eq!(
        draw(root, false, Some(&channel), request.as_bytes()).await,
        "rejected reason=not_qualification"
    );
    assert_eq!(
        draw(root, true, Some(&channel), missing.as_bytes()).await,
        "rejected reason=lease_required"
    );
    assert_eq!(
        draw(root, true, Some(&channel), request.as_bytes()).await,
        "rejected reason=lease_required",
        "the durable lease did not bind the committed player slot"
    );
    channel
        .lock()
        .await
        .bind_attacker_lease(player, session, lease)?;

    let drive = when_draw_is_armed(&channel, |owner| {
        owner.drive_spawn_respawns(FIRST_CYCLE - 1, |_, _| false)?;
        assert!(
            owner.qualification_forge_draw_pending(),
            "early cycle consumed the draw"
        );
        owner.drive_spawn_respawns(FIRST_CYCLE, |_, _| false)?;
        assert!(
            !owner.qualification_forge_draw_pending(),
            "draw was not one-shot"
        );
        Ok(())
    });
    let (reply, driven) = tokio::join!(draw(root, true, Some(&channel), request.as_bytes()), drive);
    driven?;
    let census: Value = serde_json::from_str(&reply).map_err(|err| format!("{err}: {reply}"))?;
    assert_eq!(census["now_micros"], FIRST_CYCLE);
    assert_eq!(census["seed"], seed);
    assert_eq!(census["due_micros"], FIRST_CYCLE);
    assert_replayed_selection(&census)?;
    let mut owner = channel.lock().await;
    let mut candidates = Vec::new();
    let mut eligible = Vec::new();
    let mut selected = Vec::new();
    let mut fiends = Vec::new();
    let mut held_fiend = None;
    for (point, _, actor) in owner.spawn_point_creatures() {
        let actor = actor.ok_or("expected a live native spawn")?;
        let spawn_identity = (u64::from(point.source) << 8) | u64::from(point.ordinal);
        candidates.push(ForgeSpawnCandidate {
            spawn_identity,
            eligible: true,
            alive: true,
        });
        eligible.push(serde_json::json!({
            "id": actor_id(actor),
            "generation": actor.actor_local_generation(),
            "spawn_identity": spawn_identity,
        }));
        let snapshot = owner.companion_snapshot(actor)?;
        if let Some(kind) = snapshot.forge {
            if kind == ForgeCreature::Fiendish {
                fiends.push(spawn_identity);
                held_fiend = Some(actor);
            }
            selected.push(selected_entry(&snapshot, kind));
        }
    }
    let mut expected = plan_fiendish_tick(FIRST_CYCLE, seed, &[], &candidates).selected;
    expected.sort_unstable();
    fiends.sort_unstable();
    assert_eq!(
        fiends, expected,
        "the production selector must consume the requested seed"
    );
    assert_eq!(census["eligible"], Value::Array(eligible));
    assert_eq!(census["retained"], serde_json::json!([]));
    assert_eq!(census["fiendish_vacancy"], FIENDISH_SIMULTANEOUS_MAX);
    assert_eq!(
        census["fiendish_available_slots"],
        FIENDISH_SIMULTANEOUS_MAX
    );
    let reported = census["selected"]
        .as_array()
        .ok_or("missing selected census")?;
    assert_eq!(reported.len(), expected.len() + 1);
    assert_eq!(reported.len(), selected.len());
    assert!(selected.iter().all(|entry| reported.contains(entry)));
    assert_eq!(owner.companion_snapshot(owned)?.forge, None);
    assert_eq!(
        census["excluded_owned_summons"],
        serde_json::json!([{"id": actor_id(owned), "reason": "owned_summon"}])
    );
    // One retained fiend dies before the next scan. The planner sees a vacancy
    // while the population owner still reserves that actor's slot.
    let held_fiend = held_fiend.ok_or("missing fiend to hold a replacement slot")?;
    owner.set_test_creature_health(held_fiend, 0)?;
    let retained: Vec<_> = selected
        .iter()
        .map(|entry| {
            serde_json::json!({
                "id": entry["id"], "generation": entry["generation"], "stacks": entry["stacks"]
            })
        })
        .collect();
    drop(owner);
    let drive = when_draw_is_armed(&channel, |owner| {
        owner.drive_spawn_respawns(2 * FIRST_CYCLE, |_, _| false)?;
        Ok(())
    });
    let (reply, driven) = tokio::join!(draw(root, true, Some(&channel), request.as_bytes()), drive);
    driven?;
    let census: Value = serde_json::from_str(&reply).map_err(|err| format!("{err}: {reply}"))?;
    assert_eq!(census["now_micros"], 2 * FIRST_CYCLE);
    assert_eq!(census["seed"], seed);
    assert_eq!(census["due_micros"], 2 * FIRST_CYCLE);
    assert_replayed_selection(&census)?;
    assert_eq!(census["fiendish_vacancy"], 1);
    assert_eq!(census["fiendish_available_slots"], 0);
    let reported = census["retained"]
        .as_array()
        .ok_or("missing retained roster")?;
    assert_eq!(reported.len(), retained.len());
    assert!(retained.iter().all(|entry| reported.contains(entry)));
    let owner = channel.lock().await;
    let mut newly_classified = Vec::new();
    for actor in owner
        .spawn_point_creatures()
        .into_iter()
        .filter_map(|(_, _, actor)| actor)
    {
        let snapshot = owner.companion_snapshot(actor)?;
        if let Some(kind) = snapshot.forge {
            let entry = selected_entry(&snapshot, kind);
            if !selected.contains(&entry) {
                assert!(matches!(kind, ForgeCreature::Influenced(_)));
                newly_classified.push(entry);
            }
        }
    }
    assert_eq!(newly_classified.len(), 1);
    assert_eq!(census["selected"], Value::Array(newly_classified));
    drop(owner);

    let advance = when_draw_is_armed(&channel, |owner| {
        owner.record_control_loss(
            player,
            session,
            ControlLossMark {
                epoch: 1,
                grace_deadline: now + 10,
            },
        )?;
        owner.restore_control(player, session, 1)?;
        owner.drive_spawn_respawns(3 * FIRST_CYCLE, |_, _| false)?;
        Ok(())
    });
    let (stale, advanced) = tokio::join!(
        draw(root, true, Some(&channel), request.as_bytes()),
        advance
    );
    advanced?;
    assert_eq!(
        stale, "rejected reason=lease_required",
        "a draw cannot cross a control revision"
    );
    eprintln!("Forge population lease, production draw, and owned-summon assertions reached");
    Ok(())
}

#[derive(Deserialize)]
struct DrawEvidence {
    now_micros: u64,
    due_micros: u64,
    seed: u64,
    artifact_digest: String,
    fiendish_slots: Vec<FiendishSlotEvidence>,
    candidates: Vec<CandidateEvidence>,
}

#[derive(Deserialize)]
struct FiendishSlotEvidence {
    replacement_due_micros: Option<u64>,
}

#[derive(Deserialize)]
struct CandidateEvidence {
    id: String,
    generation: u64,
    spawn_identity: u64,
    stacks: Option<u8>,
    owned: bool,
    forge_difficulty: Option<u8>,
    creature_key: String,
    position: Value,
}

impl CandidateEvidence {
    fn selection(&self, kind: ForgeCreature) -> Value {
        serde_json::json!({
            "id": self.id,
            "generation": self.generation,
            "stacks": kind.reward_stacks(),
            "creature_key": self.creature_key,
            "position": self.position,
        })
    }
}

// Replay uses the ordered pre-draw snapshots. The reported selection, vacancy
// and allowances are assertions, never inputs to the plan.
fn assert_replayed_selection(census: &Value) -> TestResult {
    let evidence = DrawEvidence::deserialize(census)?;
    assert!(evidence.now_micros >= evidence.due_micros);
    assert_eq!(census["fiendish_cap"], FIENDISH_SIMULTANEOUS_MAX);
    assert_eq!(census["influenced_cap"], INFLUENCED_SIMULTANEOUS_MAX);
    assert_eq!(
        census["influenced_per_tick_cap"],
        influenced_available_slots(0)
    );
    assert_eq!(census["fiendish_respawn_micros"], FIENDISH_RESPAWN_MICROS);
    assert_eq!(census["population_tick_micros"], FIRST_CYCLE);

    let artifact = hex_bytes::<32>(&evidence.artifact_digest).map_err(debug)?;
    let held = evidence
        .fiendish_slots
        .iter()
        .filter(|slot| {
            slot.replacement_due_micros
                .is_none_or(|due| evidence.now_micros < due)
        })
        .count();
    let available = usize::from(FIENDISH_SIMULTANEOUS_MAX).saturating_sub(held);
    assert_eq!(census["fiendish_available_slots"], available);

    let fiendish = ForgeCreature::Fiendish.reward_stacks();
    let mut existing = Vec::new();
    let mut candidates = Vec::new();
    let mut eligible = Vec::new();
    let mut influenced_count = 0;
    for candidate in &evidence.candidates {
        let is_fiendish = candidate.stacks == Some(fiendish);
        let is_influenced = candidate.stacks.is_some() && !is_fiendish;
        let unowned_forgeable = !candidate.owned && candidate.forge_difficulty.is_some();
        if is_fiendish {
            existing.push(candidate.spawn_identity);
        }
        influenced_count += usize::from(is_influenced);
        let is_eligible = unowned_forgeable && !is_influenced;
        if is_eligible {
            eligible.push(serde_json::json!({
                "id": candidate.id,
                "generation": candidate.generation,
                "spawn_identity": candidate.spawn_identity,
            }));
        }
        candidates.push(ForgeSpawnCandidate {
            spawn_identity: candidate.spawn_identity,
            eligible: is_eligible,
            alive: true,
        });
    }

    assert_eq!(census["eligible"], Value::Array(eligible));
    let plan = plan_fiendish_tick(evidence.now_micros, evidence.seed, &existing, &candidates);
    assert_eq!(census["fiendish_vacancy"], plan.vacancy);
    let additions: Vec<_> = plan
        .selected
        .into_iter()
        .filter(|identity| !existing.contains(identity))
        .take(available)
        .collect();
    let mut selected = Vec::new();
    for identity in &additions {
        let candidate = evidence
            .candidates
            .iter()
            .find(|candidate| candidate.spawn_identity == *identity)
            .ok_or("missing planned fiendish spawn")?;
        selected.push(candidate.selection(ForgeCreature::Fiendish));
    }

    let influenced_available = influenced_available_slots(influenced_count);
    assert_eq!(census["influenced_count"], influenced_count);
    assert_eq!(census["influenced_available_slots"], influenced_available);
    let mut rolls = Vec::new();
    for candidate in &evidence.candidates {
        if candidate.owned
            || candidate.forge_difficulty.is_none()
            || candidate.stacks.is_some()
            || additions.contains(&candidate.spawn_identity)
        {
            continue;
        }
        let placement = uuid_bytes(&candidate.id).ok_or("invalid candidate placement identity")?;
        let roll = forge_population_roll(
            artifact,
            placement,
            candidate.generation,
            Some(evidence.seed),
        );
        rolls.push((roll, candidate));
    }
    if let Some((candidate, kind)) = plan_influenced_tick(influenced_available, rolls.into_iter()) {
        selected.push(candidate.selection(kind));
    }
    assert_eq!(census["selected"], Value::Array(selected));
    Ok(())
}

#[test]
fn forge_population_draw_replays_fixed_json() -> TestResult {
    // Fixed orders and stacks computed from the two documented hash inputs.
    let census: Value = serde_json::from_str(
        r#"{
    "now_micros": 20000000,
    "due_micros": 20000000,
    "seed": 18446744073709551613,
    "artifact_digest": "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
    "population_tick_micros": 10000000,
    "fiendish_respawn_micros": 270000000,
    "fiendish_cap": 4,
    "influenced_cap": 300,
    "influenced_per_tick_cap": 1,
    "influenced_count": 1,
    "influenced_available_slots": 1,
    "fiendish_slots": [
        {"id":"00000000-0000-0000-0000-000000000001","generation":7,"replacement_due_micros":null},
        {"id":"00000000-0000-0000-0000-000000000008","generation":14,"replacement_due_micros":280000000},
        {"id":"00000000-0000-0000-0000-000000000009","generation":15,"replacement_due_micros":290000000},
        {"id":"00000000-0000-0000-0000-00000000000a","generation":16,"replacement_due_micros":20000000}
    ],
    "fiendish_vacancy": 3,
    "fiendish_available_slots": 1,
    "candidates": [
        {"id":"00000000-0000-0000-0000-000000000001","generation":7,"spawn_identity":256,"stacks":15,"owned":false,"forge_difficulty":4,"creature_key":"oteryn:creature.rat","position":{"x":21,"y":3,"floor":-1}},
        {"id":"00000000-0000-0000-0000-000000000002","generation":8,"spawn_identity":257,"stacks":null,"owned":false,"forge_difficulty":4,"creature_key":"oteryn:creature.rat","position":{"x":22,"y":3,"floor":-1}},
        {"id":"00000000-0000-0000-0000-000000000003","generation":9,"spawn_identity":258,"stacks":null,"owned":false,"forge_difficulty":4,"creature_key":"oteryn:creature.rat","position":{"x":23,"y":3,"floor":-1}},
        {"id":"00000000-0000-0000-0000-000000000004","generation":10,"spawn_identity":259,"stacks":null,"owned":false,"forge_difficulty":4,"creature_key":"oteryn:creature.rat","position":{"x":24,"y":3,"floor":-1}},
        {"id":"00000000-0000-0000-0000-000000000005","generation":11,"spawn_identity":260,"stacks":2,"owned":false,"forge_difficulty":4,"creature_key":"oteryn:creature.rat","position":{"x":25,"y":3,"floor":-1}},
        {"id":"00000000-0000-0000-0000-000000000006","generation":12,"spawn_identity":261,"stacks":null,"owned":true,"forge_difficulty":4,"creature_key":"oteryn:creature.rat","position":{"x":26,"y":3,"floor":-1}},
        {"id":"00000000-0000-0000-0000-000000000007","generation":13,"spawn_identity":262,"stacks":null,"owned":false,"forge_difficulty":null,"creature_key":"oteryn:creature.rat","position":{"x":27,"y":3,"floor":-1}}
    ],
    "eligible": [
        {"id":"00000000-0000-0000-0000-000000000001","generation":7,"spawn_identity":256},
        {"id":"00000000-0000-0000-0000-000000000002","generation":8,"spawn_identity":257},
        {"id":"00000000-0000-0000-0000-000000000003","generation":9,"spawn_identity":258},
        {"id":"00000000-0000-0000-0000-000000000004","generation":10,"spawn_identity":259}
    ],
    "retained": [
        {"id":"00000000-0000-0000-0000-000000000001","generation":7,"stacks":15},
        {"id":"00000000-0000-0000-0000-000000000005","generation":11,"stacks":2},
        {"id":"00000000-0000-0000-0000-000000000008","generation":14,"stacks":15},
        {"id":"00000000-0000-0000-0000-000000000009","generation":15,"stacks":15}
    ],
    "selected": [
        {"id":"00000000-0000-0000-0000-000000000004","generation":10,"creature_key":"oteryn:creature.rat","position":{"x":24,"y":3,"floor":-1},"stacks":15},
        {"id":"00000000-0000-0000-0000-000000000002","generation":8,"creature_key":"oteryn:creature.rat","position":{"x":22,"y":3,"floor":-1},"stacks":2}
    ],
    "excluded_owned_summons": [
        {"id":"00000000-0000-0000-0000-000000000006","reason":"owned_summon"}
    ]
}"#,
    )?;
    assert_replayed_selection(&census)
}

async fn when_draw_is_armed(
    channel: &Mutex<ChannelRuntimeV1>,
    drive: impl FnOnce(&mut ChannelRuntimeV1) -> TestResult,
) -> TestResult {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let mut owner = channel.lock().await;
            if owner.qualification_forge_draw_pending() {
                return drive(&mut owner);
            }
            drop(owner);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await?
}

fn selected_entry(snapshot: &CompanionSnapshot, kind: ForgeCreature) -> Value {
    let MovementLocalPosition { x, y, floor } = snapshot.position;
    serde_json::json!({
        "id": actor_id(snapshot.actor),
        "generation": snapshot.actor.actor_local_generation(),
        "stacks": kind.reward_stacks(),
        "creature_key": snapshot.state.policy.definition_key,
        "position": {"x": x, "y": y, "floor": floor},
    })
}

fn actor_id(actor: ExactActorRef) -> String {
    uuid_text(&actor.placement_identity())
}

fn channel(harness: &Harness) -> TestResult<(ChannelRuntimeV1, u64)> {
    let scope = scope()?;
    let pin = ChannelContentPin::from_activation(
        scope.world_id(),
        1,
        [1; 32],
        [2; 32],
        [3; 32],
        [4; 32],
        (0, 0, 0),
    );
    let node = harness.node.fact();
    let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
        scope,
        node.node_id(),
        node.registration_revision(),
        1,
        1,
        "runtime-scope-assignment:1",
        32,
        pin.clone(),
    )?;
    runtime.install_companion_policies(CompiledCreaturePolicies::from_active_artifact(
        pin.server_artifact_digest(),
        vec![CompiledCreaturePolicy {
            definition_key: RAT.into(),
            definition_revision: "fixture:1".into(),
            display_name: "Rat".into(),
            maximum_health: 100,
            forge_difficulty: Some(4),
            base_speed: 110,
            outfit_look_type: 21,
            object_look_type: None,
            summonable: true,
            convinceable: false,
            mana_cost: Some(100),
            is_familiar: false,
            condition_immunities: vec![],
            defense: Some(1),
            armor: Some(1),
            mitigation: None,
            resistances: vec![],
            damage_immunities: vec![],
            healing_from_damage: vec![],
            preferred_distance: Some(1),
            reward_boss: Some(false),
            blocked_by_nearby_players: false,
            flags: CreatureFlags {
                attackable: true,
                illusionable: false,
                health_hidden: false,
            },
        }],
    )?)?;
    let points = (2..11)
        .map(|x| ((x, 3, -1), 60_000, RAT.to_owned(), 100, false))
        .collect();
    runtime.begin_activation_spawn_sources(vec![(points, 5_000)])?;
    assert_eq!(runtime.realize_activation_spawn_window(0)?.remaining, 0);
    // Pick a seed whose set differs from the artifact default, regardless of fixture source IDs.
    let candidates: Vec<_> = runtime
        .spawn_point_creatures()
        .into_iter()
        .map(|(point, _, _)| ForgeSpawnCandidate {
            spawn_identity: (u64::from(point.source) << 8) | u64::from(point.ordinal),
            eligible: true,
            alive: true,
        })
        .collect();
    let default_seed = u64::from_be_bytes(pin.server_artifact_digest()[..8].try_into()?);
    let default = plan_fiendish_tick(FIRST_CYCLE, default_seed, &[], &candidates).selected;
    let seed = (0..64)
        .find(|seed| {
            plan_fiendish_tick(FIRST_CYCLE, *seed, &[], &candidates)
                .selected
                .iter()
                .any(|id| !default.contains(id))
        })
        .ok_or("no discriminating Forge seed")?;
    Ok((runtime, seed))
}
