//! A real admitted lease and live owned summon, driven by the ordinary spawn owner.
#![cfg(feature = "qualification-spawn")]

use crate::bestiary_postgres_harness::{Harness, TestResult, debug, id, runtime, scope};
use crate::foundation::{ChannelRuntimeV1, ControlLossMark, ExactActorRef, MovementLocalPosition};
use crate::node::serve::qualification_forge_population::draw;
use crate::node::uuid_text;
use crate::party_shared_xp_postgres_tests::committed_fresh_player;
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
use oteryn_gameplay_domain::domain::forge_creature::{
    ForgeCreature, ForgeSpawnCandidate, plan_fiendish_tick,
};
use oteryn_gameplay_entities::carrier::runtime_actor_companion::{
    CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags,
};
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
    let owner = channel.lock().await;
    let mut candidates = Vec::new();
    let mut eligible = Vec::new();
    let mut selected = Vec::new();
    let mut fiends = Vec::new();
    for (point, _, actor) in owner.spawn_point_creatures() {
        let actor = actor.ok_or("expected a live native spawn")?;
        let spawn_identity = (u64::from(point.source) << 8) | u64::from(point.ordinal);
        candidates.push(ForgeSpawnCandidate {
            spawn_identity,
            eligible: true,
            alive: true,
        });
        eligible.push(serde_json::json!({"id": actor_id(actor), "spawn_identity": spawn_identity}));
        match owner.companion_snapshot(actor)?.forge {
            Some(ForgeCreature::Fiendish) => {
                fiends.push(spawn_identity);
                selected.push(serde_json::json!({"id": actor_id(actor), "stacks": ForgeCreature::Fiendish.reward_stacks()}));
            }
            Some(ForgeCreature::Influenced(stacks)) => {
                selected.push(serde_json::json!({"id": actor_id(actor), "stacks": stacks}));
            }
            None => {}
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
        owner.drive_spawn_respawns(2 * FIRST_CYCLE, |_, _| false)?;
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
        .map(|x| ((x, 0, 0), 60_000, RAT.to_owned(), 100, false))
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
