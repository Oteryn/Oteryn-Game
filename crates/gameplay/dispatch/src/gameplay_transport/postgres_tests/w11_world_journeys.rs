//! Both endgame chains keep one Character through every pinned entry and return.
use super::fixture::{Harness, debug};
use super::m01b_explore_fixtures::koshei::relocate;
use super::soul_war_fixtures::current;
use super::{ComposedFreshAdmission, Dependencies, TestResult, reset};
use crate::content::bosses::activity_template::{
    PinnedActivitySourceRoom as Room, captured_boss_levers,
};
use crate::foundation::{
    CommandId, CommandRef, ExactActorRef, MovementLocalPosition, OwnerDamageCommand,
    RuntimeScopeRefV1,
};
use crate::gameplay_transport::connection::AdmittedSession;
use oteryn_gameplay_quests::quest::{rotten_blood as rotten, world_callbacks::Position};
use oteryn_protocol_oteryn::{CommandStatus, boss_encounter as wire};
use std::time::Duration;

const SOUL_WAR: &[(Room, &str)] = &[
    (Room::GoshnarsMalice, "oteryn:creature.goshnar_s_malice"),
    (Room::GoshnarsGreed, "oteryn:creature.goshnar_s_greed"),
    (Room::GoshnarsSpite, "oteryn:creature.goshnar_s_spite"),
    (Room::GoshnarsCruelty, "oteryn:creature.goshnar_s_cruelty"),
    (Room::GoshnarsHatred, "oteryn:creature.goshnar_s_hatred"),
    (
        Room::GoshnarsMegalomania,
        "oteryn:creature.goshnar_s_megalomania_purple",
    ),
];
const ROTTEN_BLOOD: &[(Room, &str)] = &[
    (Room::Murcion, "oteryn:creature.murcion"),
    (Room::Vemiath, "oteryn:creature.vemiath"),
    (Room::Chagorz, "oteryn:creature.chagorz"),
    (Room::Ichgahal, "oteryn:creature.ichgahal"),
    (Room::Bakragore, "oteryn:creature.bakragore"),
];
const ASPECT: &str = "oteryn:creature.aspect_of_power";
const GREEN: &str = "oteryn:creature.goshnar_s_megalomania_green";

#[test]
fn w11_world_soul_war_levers_victories_and_one_chest_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |_| {
        journey(admin, SOUL_WAR, true).await
    })
}

#[test]
fn w11_world_rotten_blood_paid_access_levers_and_one_outfit_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |_| {
        journey(admin, ROTTEN_BLOOD, false).await
    })
}

fn lever(key: &str) -> TestResult<Position> {
    let candidates = captured_boss_levers().map_err(debug)?;
    let &[x, y, z] = candidates
        .iter()
        .find(|row| row.creature_key.as_deref() == Some(key))
        .and_then(|row| row.lever_positions.first())
        .ok_or_else(|| format!("first captured pad: {key}"))?;
    Ok(Position {
        x: u16::try_from(x)?,
        y: u16::try_from(y)?,
        floor: -i8::try_from(z)?,
    })
}

async fn journey(admin: String, chain: &[(Room, &str)], soul_war: bool) -> TestResult {
    reset();
    let &(_, final_key) = chain.last().ok_or("endgame final")?;
    let start = lever(final_key)?;
    let (scene, world, entry_pin) =
        super::srx3_soul_war_world_fixture_fixtures::source_world_with_bundle(
            MovementLocalPosition {
                x: i32::from(start.x),
                y: i32::from(start.y),
                floor: -i16::from(start.floor),
            },
        )?;
    let active = scene.controller.active().ok_or("active World generation")?;
    let deps = Dependencies::with_content_and_capacity(
        scene.pin.clone(),
        scene.room.clone(),
        crate::spell::cast::v1_spell_book().map_err(debug)?,
        16,
    )?;
    let mut rooms = Vec::new();
    for &(kind, _) in chain {
        let raw = active
            .qualify_activity_source_room(&scene.room, kind)
            .map_err(debug)?;
        let room = crate::gameplay_transport::activity_runtime::source_bundle::qualify(
            &raw,
            kind,
            active,
            scene.pin.world_id(),
            deps.admitted.channel_id,
        )
        .map_err(|_| format!("pinned destination {kind:?}"))?;
        rooms.push((kind, room));
    }

    let name = if soul_war {
        "w11_world_soul"
    } else {
        "w11_world_rotten"
    };
    let h = Harness::create_with_admission_seed(admin, name, false, false).await?;
    let evidence = super::house_transfer_fixtures::RecoverySource::start(&h).await?;
    super::house_transfer_fixtures::prepare_owned_session(&h).await?;
    let funds = [("oteryn:item.tibia.i3043", 100); 5];
    super::item_look_fixtures::seed_main_backpack(&h, if soul_war { &[] } else { &funds }).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut owner = deps.owner(&h, &authority);
    owner.evidence = &evidence.evidence;
    owner.active_generation = Some(active);
    owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&scene.room));
    let catalogue = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    owner.quest_catalogue = Some(std::sync::Arc::new(catalogue.catalogue().clone()));
    for (kind, room) in &rooms {
        owner
            .activity_runtimes
            .register_source_room(*kind, room)
            .map_err(|_| "register pinned destination")?;
    }
    super::boss_activity_fixtures::issue_content_and_reset_with_runtime_pin(
        &h,
        &entry_pin,
        &scene.pin,
        world.map_revision(),
    )
    .await?;
    let mut admitted = deps.admitted;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::connection::fresh_capability_selection(
            &owner,
            &[
                4,
                6,
                7,
                10,
                11,
                12,
                15,
                17,
                18,
                23,
                30,
                92,
                wire::CAPABILITY_BOSS_ENCOUNTER_V1,
                385,
            ],
        )
        .await
        .ok_or("native endgame capabilities")?;
    super::srx3_activity_source_fixture::prepare_player(&h, &owner, &mut admitted, 1999).await?;
    activate_quest(&h, &mut owner, &mut admitted, soul_war).await?;
    let origin = admitted.item_fence.ok_or("origin fence")?.runtime_scope;

    // One assignment per fight. The unused final assignment crosses the
    // counterfactual refusal, so missing capacity cannot mask a quest refusal.
    for index in 0..chain.len() {
        super::activity_runtime_fixtures::allocate_ready_instance(&h, 218 + u8::try_from(index)?)
            .await?;
    }
    let (_, refused, barrier) =
        super::boss_activity_fixtures::exchange(&owner, &mut admitted, final_key, false).await?;
    assert_eq!(
        refused,
        wire::BossActivityReply::Refused(wire::BossActivityRefusal::AuthorityRejected)
    );
    assert!(barrier.is_none());

    for (index, &(kind, key)) in chain.iter().enumerate() {
        let player = admitted.runtime_actor.ok_or("current World player")?;
        relocate(&owner, player, admitted.game_session_id, lever(key)?).await?;
        let (_, reply, barrier) =
            super::boss_activity_fixtures::exchange(&owner, &mut admitted, key, true).await?;
        assert!(matches!(reply, wire::BossActivityReply::Accepted { .. }));
        assert!(barrier.is_some(), "{key}: in-instance snapshot");
        let fence = admitted.item_fence.ok_or("current Instance fence")?;
        let RuntimeScopeRefV1::Instance { instance_id, .. } = fence.runtime_scope else {
            return Err("lever must transfer into its pinned Instance".into());
        };
        let instance = owner
            .activity_runtimes
            .get(fence.runtime_scope, fence.scope_ownership_generation.get())
            .ok_or("current Instance owner")?;
        let view = owner.runtime_view(&instance);
        view.refresh_quest_session(admitted.game_session_id, false)
            .await;
        let player = admitted.runtime_actor.ok_or("current Instance player")?;
        assert!(
            view.refresh_source_party_presence(player, admitted.game_session_id)
                .await
        );
        if !soul_war {
            // Run the original first-Think work before publishing damage,
            // including Bakragore's fenced essence snapshot.
            super::srx3_activity_source_fixture::drive_native_cycle(&view).await?;
        }
        let lethal_key = if kind == Room::GoshnarsMegalomania {
            open_megalomania(&view).await?;
            GREEN
        } else {
            key
        };
        defeat(&view, &admitted, lethal_key).await?;
        drop(view);
        wait_for_return(&h, instance_id).await?;
        let (_, reply, barrier) = super::boss_activity_fixtures::exchange_command(
            &owner,
            &mut admitted,
            key,
            true,
            wire::COMMAND_TYPE_BOSS_DEFEATED,
        )
        .await?;
        assert!(matches!(reply, wire::BossActivityReply::Accepted { .. }));
        assert!(barrier.is_some(), "{key}: return snapshot");
        assert_eq!(
            admitted
                .item_fence
                .ok_or("returned World fence")?
                .runtime_scope,
            origin
        );
        check_taints(
            &owner,
            &mut admitted,
            soul_war,
            index + 1 == chain.len(),
            u8::try_from(index + 1)?,
        )
        .await?;
    }

    if soul_war {
        claim_soul_reward(&h, &mut owner, &mut admitted).await?;
    } else {
        let outfit = "oteryn:appearance-grant/quest/rotten-blood/decaying-defender-base";
        let rows: (i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM game_account_appearance_unlocks WHERE unlock_key=$1),
                    (SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE unlock_key=$1)",
        ).bind(outfit).fetch_one(&h.pool).await?;
        assert_eq!(rows, (1, 1));
    }
    eprintln!("W11_WORLD_JOURNEY POSTGRES_CASE_COMPLETED family={name}");
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

async fn activate_quest(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    soul_war: bool,
) -> TestResult {
    let player = admitted.runtime_actor.ok_or("current World Player")?;
    let session = admitted.game_session_id;
    owner.admit_quest_session(admitted).await;
    let (npc_key, dialogue): (_, &[&str]) = if soul_war {
        ("oteryn:npc.flickering_soul", &["hi", "task", "yes"])
    } else {
        sqlx::query("INSERT INTO game_type2_audit_activation VALUES(1,now())")
            .execute(&h.pool)
            .await?;
        (
            "oteryn:npc.bloodshade_sacrifice",
            &["hi", "quest", "detour", "yes", "yes"],
        )
    };
    let npc = super::rotten_blood_progression_fixtures::place_npc(owner, player, session, npc_key)
        .await?;
    for (number, text) in (40..).zip(dialogue) {
        advance_npc_or_use(owner)?;
        super::rotten_blood_progression_fixtures::talk(owner, player, session, npc, number, text)
            .await?;
    }
    let fence = current(owner, session).await?;
    if soul_war {
        assert!(
            h.root
                .read_soul_war_state(owner.character, owner.holder, fence)
                .await?
                .access()
        );
    } else {
        assert!(
            h.root
                .read_rotten_blood_state(owner.character, owner.holder, fence)
                .await?
                .access()
        );
        let fee: i64 = sqlx::query_scalar(
            "SELECT fee_gold_units FROM game_item_fee_burns WHERE cause_kind=15",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(u64::try_from(fee)?, rotten::ACCESS_PRICE);
    }
    admitted.item_fence = Some(fence.item_fence());
    owner
        .activity_runtimes
        .remember(*admitted)
        .map_err(|_| "current World connection")?;
    Ok(())
}

fn advance_npc_or_use(owner: &mut ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(2))
        .ok_or("action pace")?;
    Ok(())
}

async fn await_creature(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    key: &str,
) -> TestResult<ExactActorRef> {
    tokio::time::timeout(Duration::from_secs(16), async {
        loop {
            super::srx3_native_source_cast_fixtures::settle_monster_melee_at(
                owner,
                owner.owner_now().get(),
            )
            .await?;
            let runtime = owner.runtime.lock().await;
            if let Some(actor) = runtime
                .current_live_creature_census()
                .map_err(debug)?
                .into_iter()
                .find(|actor| runtime.matches_live_creature_identity(*actor, key.as_bytes()))
            {
                return Ok(actor);
            }
            drop(runtime);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .map_err(|_| format!("native actor did not appear: {key}"))?
}

async fn open_megalomania(view: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    // Global archive, Soul War Quest/Spoiler: "After a round of 4 Aspects
    // are killed, Goshnar's Megalomania becomes vulnerable for 70 seconds".
    for ordinal in 0_u8..4 {
        let actor = await_creature(view, ASPECT).await?;
        let now = view.owner_now();
        {
            let mut runtime = view.runtime.lock().await;
            let damage = runtime.companion_snapshot(actor).map_err(debug)?.health;
            let hit = runtime
                .borrow_exact_actor_commit_at_us(now.get())
                .commit_damage(
                    actor,
                    OwnerDamageCommand {
                        target: ASPECT.as_bytes(),
                        occurrence: &[ordinal],
                        binding: b"w11-aspect",
                        damage,
                    },
                )
                .map_err(debug)?;
            assert!(hit.applied && hit.health_after == 0);
            let receipt = runtime
                .borrow_combat_death()
                .committed_lethal_receipt(actor)
                .map_err(debug)?;
            runtime
                .borrow_combat_death()
                .project_committed_lethal(receipt)
                .map_err(debug)?;
        }
        super::srx3_native_source_cast_fixtures::drain_source_death(view, actor, now).await?;
    }
    Ok(())
}

async fn defeat(
    view: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    key: &str,
) -> TestResult {
    use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, record_creature_kill};
    let boss = await_creature(view, key).await?;
    let native = view
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native combat content")?;
    let session = admitted.game_session_id;
    let player = admitted.runtime_actor.ok_or("current fight player")?;
    let now = view.owner_now();
    {
        let mut runtime = view.runtime.lock().await;
        let states = view.states.spell_states.try_lock().map_err(debug)?;
        let attack = view.states.attack.try_lock().map_err(debug)?;
        let lease = runtime
            .bound_attacker_lease(player, session)
            .map_err(debug)?;
        let damage = runtime.companion_snapshot(boss).map_err(debug)?.health;
        let hit = runtime
            .borrow_exact_actor_commit_at_us(now.get())
            .commit_damage_for_bound_attacker(
                boss,
                player,
                CommandRef::new(session, CommandId::new(32101).map_err(debug)?),
                0,
                OwnerDamageCommand {
                    target: key.as_bytes(),
                    occurrence: key.as_bytes(),
                    binding: b"w11-world-fight",
                    damage,
                },
            )
            .map_err(debug)?;
        assert!(hit.applied && hit.health_before > 0 && hit.health_after == 0);
        let activity = attack.shared_experience_cells();
        record_creature_kill(
            &mut runtime,
            KillSink {
                prey: Some(&states),
                activity: &activity,
                queue: attack.kills(),
                rewards: native.reward_table(),
                charms: Some(view.imported_charms),
            },
            boss,
            crate::combat::CapturedRewardPrincipal {
                character: lease.character_id(),
                lease_generation: lease.generation(),
                session,
                actor: player,
            },
            now.get() / 1000,
            Some(now),
        );
        assert!(
            attack.kills().has_session(session),
            "{key}: actual lethal reward entry"
        );
    }
    super::srx3_native_source_cast_fixtures::drain_source_death(view, boss, now).await?;
    view.root.await_ready().await?;
    assert_eq!(
        view.drain_kill_rewards(session).await,
        DrainOutcome::Drained,
        "{key}"
    );
    Ok(())
}

async fn wait_for_return(h: &Harness, instance_id: [u8; 16]) -> TestResult {
    let remaining: i64 = sqlx::query_scalar(
        "SELECT greatest(0, exit_deadline - ceil(extract(epoch FROM clock_timestamp()) * 1000)::bigint)
         FROM game_boss_room_activities WHERE instance_id=encode($1,'hex')::uuid AND state=2",
    ).bind(instance_id.as_slice()).fetch_one(&h.pool).await?;
    // Follow the published deadline. Advancing the shared quest clock would
    // put the next admission ahead of its real native damage timestamp.
    tokio::time::sleep(Duration::from_millis(u64::try_from(remaining)? + 1)).await;
    Ok(())
}

async fn check_taints(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    soul_war: bool,
    final_boss: bool,
    wins: u8,
) -> TestResult {
    let fence = current(owner, admitted.game_session_id).await?;
    let mut expected = wire::BossTaints {
        soul_war: 0,
        rotten_blood: 0,
        rotten_blood_flags: 0,
        rotten_blood_final: false,
    };
    if soul_war {
        let state = owner
            .root
            .read_soul_war_state(owner.character, owner.holder, fence)
            .await?;
        assert_eq!(state.completed(), final_boss);
        let count = if final_boss { 0 } else { wins };
        assert_eq!(state.defeated_bosses.count_ones(), u32::from(count));
        assert_eq!(state.first_taint_at.is_none(), final_boss);
        expected.soul_war = count;
    } else {
        let state = owner
            .root
            .read_rotten_blood_state(owner.character, owner.holder, fence)
            .await?;
        assert_eq!(
            (state.completed(), state.final_taint()),
            (final_boss, final_boss)
        );
        let count = if final_boss { 0 } else { wins };
        assert_eq!(state.regular_taints(), count);
        assert_eq!(state.child_flags().count_ones(), u32::from(count));
        expected.rotten_blood = count;
        expected.rotten_blood_flags = state.child_flags();
        expected.rotten_blood_final = final_boss;
    }
    let mut taint_field = Vec::new();
    wire::append_boss_taints(&mut taint_field, expected).map_err(debug)?;
    let id = admitted.continuity.next_command_id;
    let (status, payload) = super::boss_encounter_fixtures::exchange_kind(
        owner,
        admitted,
        id,
        wire::COMMAND_TYPE_BOSS_COOLDOWNS,
        &[16, 1],
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert!(
        payload.ends_with(&taint_field),
        "fenced wire projection {expected:?}"
    );
    Ok(())
}

async fn claim_soul_reward(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::{
        UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
    };
    // Crystal (Global silent), data-global/scripts/quests/soul_war/action-reward_soul_war.lua:4–28.
    let chest = Position {
        x: 33620,
        y: 31400,
        floor: -10,
    };
    let player = admitted.runtime_actor.ok_or("current reward player")?;
    relocate(
        owner,
        player,
        admitted.game_session_id,
        Position {
            x: chest.x - 1,
            ..chest
        },
    )
    .await?;
    let map = owner.bundle_map().ok_or("published chest map")?;
    let tile = crate::map::overlay::TilePos {
        x: chest.x,
        y: chest.y,
        floor: chest.floor,
    };
    use crate::map::view::{EntrySource, MapFacts, compose_tile};
    // Tibia Fandom archive, Brass-Shod Chest: itemid 23740.
    let tile = compose_tile(&map.overlay, &map.facts, tile)
        .map_err(debug)?
        .ok_or("published Soul War chest tile")?;
    let key = tile
        .entries
        .iter()
        .find_map(|entry| match entry.source {
            EntrySource::Base { placement_key, .. } if entry.facts.appearance_id == 23740 => {
                Some(placement_key)
            }
            _ => None,
        })
        .ok_or("published Brass-Shod Chest placement")?;
    let expected_revision = map.facts.object_revision(key);
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let request = encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision,
    })
    .map_err(debug)?;
    for expected in [UseDisposition::Committed, UseDisposition::Rejected] {
        advance_npc_or_use(owner)?;
        let number = admitted.continuity.next_command_id;
        let reply = super::quest_world_callbacks_fixtures::wire(
            owner,
            admitted,
            number,
            2,
            request.clone(),
        )
        .await?;
        assert_eq!(decode_use_result(&reply).map_err(debug)?, expected);
    }
    let fence = current(owner, admitted.game_session_id).await?;
    let state = h
        .root
        .read_soul_war_state(owner.character, owner.holder, fence)
        .await?;
    assert!(state.completed() && state.reward_claimed());
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_reward_claim_mint_receipts WHERE claim_production_key=$1",
    )
    .bind(crate::durability::quest_state::soul_war::SOUL_WAR_REWARD_CLAIM)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(receipts, 1);
    Ok(())
}
