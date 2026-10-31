//! W-2: framed commands -> composed owner -> fenced PostgreSQL -> durable allocation/reload.
use super::atelier::fixture as atelier_fixture;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
    serve_admitted,
};
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use crate::gameplay_transport::{ComposedFreshAdmission, SeamRooms, item_fence_of};
use oteryn_gameplay_progression::wheel_gem_data::WheelGemData;
use oteryn_protocol_oteryn::wheel as wire;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[path = "wheel_allocation/capacity.rs"]
mod capacity;

async fn send(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    body: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let command = admitted.continuity.next_command_id;
    let frame = encode_client_command(
        admitted.continuity.connection_generation,
        &ClientCommandValue {
            command_id: command,
            command_type: wire::COMMAND_TYPE_WHEEL_ALLOCATION,
            payload: body,
        },
    )
    .map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client
        .write_all(&(frame.len() as u32).to_be_bytes())
        .await?;
    client.write_all(&frame).await?;
    client.shutdown().await?;
    let end = Box::pin(serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS)).await;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _) = &end
    {
        *admitted = *current;
    }
    let mut cursor = 0;
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        let envelope =
            decode_wire_envelope(&output[cursor + 4..cursor + 4 + length]).map_err(debug)?;
        cursor += 4 + length;
        if envelope.message_type() == MessageType::CommandResult {
            let reply = decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(reply.command_id, command);
            return Ok((reply.status, reply.payload.to_vec()));
        }
    }
    Err(format!("Wheel result missing: {end:?}").into())
}

fn next_fresh_wheel_action(owner: &mut ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    // Functional refusals below must reach their own rule, not the shared UI limit.
    owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_millis(490))
        .ok_or("next fresh Wheel action")?;
    Ok(())
}

fn reply(refused: bool, revision: u64, points: [u16; 36]) -> TestResult<(CommandStatus, Vec<u8>)> {
    Ok((
        CommandStatus::Accepted,
        wire::encode_wheel_reply(&wire::WheelAllocationReply {
            refused,
            revision,
            points,
            vessels: None,
        })
        .map_err(debug)?,
    ))
}

#[test]
fn wheel_allocation_postgres_real_dispatch_refusal_reset_and_relogin_allocation() -> TestResult {
    run_wheel_case(None)
}

#[test]
fn wheel_allocation_postgres_socket_refusals_replay_and_relogin() -> TestResult {
    run_wheel_case(Some(WheelExtensionCase::Socket))
}

#[test]
fn wheel_allocation_postgres_unsocket_refusals_replay_and_relogin() -> TestResult {
    run_wheel_case(Some(WheelExtensionCase::Unsocket))
}

#[test]
fn wheel_allocation_postgres_gift_trigger_snapshot_receipts_and_relogin() -> TestResult {
    run_wheel_case(Some(WheelExtensionCase::Gift))
}

#[test]
fn wheel_allocation_postgres_capacity_uses_current_allocation_and_live_premium_at_commit()
-> TestResult {
    run_wheel_case(Some(WheelExtensionCase::Capacity))
}

#[derive(Clone, Copy)]
enum WheelExtensionCase {
    Socket,
    Unsocket,
    Gift,
    Capacity,
}

fn run_wheel_case(transition: Option<WheelExtensionCase>) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(transition).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Wheel live PostgreSQL thread panic")??;
    Ok(())
}

fn scenario(transition: Option<WheelExtensionCase>) -> TestResult {
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "wheel_live_dispatch", true, false).await?;
        let admission_request = prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                world,
                Some(include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
                ))),
            );
        let room = pin.qualified_room().clone();
        let start = room.entry_start();
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            room.map_revision_digest(),
            (start.x, start.y, start.floor),
        );
        let deps = Dependencies::with_content(
            channel_pin,
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
        h.root
            .latch_premium_activation(
                &activation.activation_id,
                activation.switch_over_us,
                activation.switch_over_us + 60_000_000,
            )
            .await
            .map_err(debug)?;
        let free = SpellPremiumCoordinator::new(
            Default::default(),
            Some(clock.clone()),
            Some(activation.clone()),
            Some(h.root.clone()),
        );
        let gate = SpellPremiumCoordinator::new(
            consumer,
            Some(clock),
            Some(activation),
            Some(h.root.clone()),
        );
        let data = WheelGemData::embedded().map_err(debug)?;
        let ruleset = Arc::new(
            crate::durability::character_wheel::WheelRuleset::from_catalogue(
                data.revision(),
                data.wheel(),
            )
            .map_err(debug)?,
        );
        let mut owner = deps.owner(&h, &authority);
        if matches!(transition, Some(WheelExtensionCase::Capacity)) {
            owner.active_generation = controller.active();
        }
        owner.wheel_ruleset = Some(Arc::clone(&ruleset));
        owner.premium_coordinator = Some(&gate);
        owner.rooms = SeamRooms::new(Some(&deps.room), Some(&deps.room));
        let session = deps.admitted.game_session_id;
        owner.admit_premium(id(40), session);
        atelier_fixture::eligible(&h.pool).await?;
        let purchase = atelier_fixture::prepare(&h).await?;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?;
        let outcome = owner
            .revision_sequencer
            .acquire(fence.character_id)
            .await
            .commit_npc_purchase(&h.root, &authority, &h.node, fence, purchase, &gate)
            .await
            .map_err(debug)?;
        assert!(matches!(
            outcome,
            crate::durability::npc_purchase::PurchaseOutcome::Committed(_)
        ));
        if !matches!(transition, Some(WheelExtensionCase::Capacity)) {
            atelier_fixture::higher_level(&h).await?;
        }
        // Initialize the actual actor from Character-owned cast facts, as admission does.
        let crate::gameplay_transport::CastFactsLoad::Ready { facts, .. } =
            crate::gameplay_transport::load_character_cast_facts(
                &h.root,
                &authority,
                fence.character_id,
            )
            .await
        else {
            return Err("cast facts".into());
        };
        assert_eq!(
            facts.vocation,
            crate::spell::Vocation::Sorcerer,
            "permanent promotion does not rewrite the Character's base vocation"
        );
        {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    session,
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                            .map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            assert!(
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .initialize(
                        &runtime,
                        deps.actor,
                        session,
                        facts,
                        (0, 0),
                        owner.owner_now()
                    )
                    .is_some()
            );
        }
        // A session without an allocation must not compete with death reconciliation for
        // the root's single holder. No durable read may even request a replacement holder.
        h.root.maintain_ready_once().await?;
        let pass = h.root.try_issue_semantic_pass()?;
        assert!(!h.root.has_ready_demand());
        assert!(owner.refresh_wheel_vitals(deps.actor, session).await);
        assert!(
            !h.root.has_ready_demand(),
            "unallocated Wheel vitals refresh must not attempt a durable read"
        );
        pass.run(|_, _| Box::pin(async { Ok(()) })).await?;
        owner.admit_wheel_session(session).await;
        let (current, _) =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(session)
                .await?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = item_fence_of(current);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[wire::CAPABILITY_WHEEL_ALLOCATION_V1],
        )
        .ok_or("Wheel capability")?;
        if matches!(transition, Some(WheelExtensionCase::Capacity)) {
            capacity::incoming_transfer(&h, &owner, &mut admitted, &gate, &free).await?;
            eprintln!("WHEEL_CAPACITY POSTGRES_CASE_COMPLETED");
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        let mut points = [0; 36];
        for (index, slot) in data.wheel()["topology"]
            .as_array()
            .ok_or("topology")?
            .iter()
            .enumerate()
        {
            if slot["domain"] == "green" {
                points[index] = u16::try_from(slot["capacity"].as_u64().ok_or("capacity")?)?;
            }
        }
        points[20] = 50;
        points[21] = 50;
        if let Some(transition) = transition {
            let initial = h.root.read_character_atelier(&authority, fence.character_id)
                .await.map_err(debug)?.ok_or("initial Atelier")?;
            let gem = initial.gems.values().find(|gem| {
                gem.domain == oteryn_gameplay_progression::wheel::WheelDomain::Green
                    && gem.quality == oteryn_gameplay_progression::atelier::GemQuality::Regular
            }).ok_or("green initial gem")?;
            let wrong_domain = initial.gems.values().find(|gem| {
                gem.domain == oteryn_gameplay_progression::wheel::WheelDomain::Red
            }).ok_or("red initial gem")?;
            let vessels = [Some(gem.id.to_be_bytes()), None, None, None];
            let save = wire::WheelAllocationRequest {
                action: 1,
                occurrence: id(170),
                expected_revision: 0,
                points,
                vessels: Some(vessels),
            };
            let payload = wire::encode_wheel_request(&save).map_err(debug)?;
            let expected = |refused, revision, current_vessels| -> TestResult<_> {
                Ok((CommandStatus::Accepted, wire::encode_wheel_reply(&wire::WheelAllocationReply {
                    refused,
                    revision,
                    points,
                    vessels: Some(current_vessels),
                }).map_err(debug)?))
            };
            let original_fence = admitted.item_fence.ok_or("item fence")?;
            let mut stale = original_fence;
            stale.character_lease_generation += 1;
            admitted.item_fence = Some(stale);
            assert_eq!(send(&owner, &mut admitted, &payload).await?.0, CommandStatus::Rejected);
            admitted.item_fence = Some(original_fence);
            // Crystal permits vessel replacement anywhere. No retained temple evidence is
            // needed when points are only added or kept (player_wheel.cpp:1763-1772).
            let rooms = owner.rooms;
            owner.rooms = SeamRooms::new(None, None);
            assert_eq!(send(&owner, &mut admitted, &payload).await?, expected(false, 1, vessels)?);
            let revision: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(send(&owner, &mut admitted, &payload).await?, expected(false, 1, vessels)?);
            assert_eq!(sqlx::query_scalar::<_, String>("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?, revision,
                "replaying the socket does not create another Character change");
            let stored = h.root.read_character_atelier(&authority, fence.character_id)
                .await.map_err(debug)?.ok_or("socketed Atelier")?;
            assert!(stored.gems[&gem.id].vessel);
            assert_eq!(stored.gems.values().filter(|gem| gem.vessel).count(), 1);
            for (seed, gem_id) in [(171, wrong_domain.id.to_be_bytes()), (172, id(199))] {
                next_fresh_wheel_action(&mut owner)?;
                let bad = wire::WheelAllocationRequest {
                    occurrence: id(seed),
                    expected_revision: 1,
                    vessels: Some([Some(gem_id), None, None, None]),
                    ..save.clone()
                };
                assert_eq!(send(&owner, &mut admitted, &wire::encode_wheel_request(&bad).map_err(debug)?).await?,
                    expected(true, 1, vessels)?, "wrong-domain or unowned gem refuses atomically");
            }
            next_fresh_wheel_action(&mut owner)?;
            let stale_revision = wire::WheelAllocationRequest { occurrence: id(173), ..save.clone() };
            assert_eq!(send(&owner, &mut admitted, &wire::encode_wheel_request(&stale_revision).map_err(debug)?).await?,
                expected(true, 1, vessels)?);
            owner.premium_coordinator = Some(&free);
            next_fresh_wheel_action(&mut owner)?;
            let unpaid = wire::WheelAllocationRequest {
                occurrence: id(174), expected_revision: 1, vessels: Some([None; 4]), ..save.clone()
            };
            assert_eq!(send(&owner, &mut admitted, &wire::encode_wheel_request(&unpaid).map_err(debug)?).await?,
                expected(true, 1, vessels)?, "free accounts cannot change vessels");
            owner.premium_coordinator = Some(&gate);
            let gift_remaining = if matches!(transition, WheelExtensionCase::Gift) {
                assert!(owner.load_wheel_gift(deps.actor, session).await);
                assert!(owner.refresh_wheel_vitals(deps.actor, session).await);
                let now = owner.owner_now().get();
                {
                    let runtime = deps.runtime.lock().await;
                    let mut states = owner.states.spell_states.lock().await;
                    let state = states.get_mut(&runtime, deps.actor, session).ok_or("Gift actor")?;
                    let facts = state.character_facts();
                    state.restore_saved_vitals((facts.max_health / 10).max(1), 0);
                    let damage = state.health + facts.max_health / 10;
                    let (next, hit) = crate::spell::actor_conditions::stage_creature_hit(state, damage, now)
                        .map_err(debug)?;
                    assert!(hit.health_after > 0, "the production Wheel projection activates Gift before lethal damage");
                    assert_eq!(next.vitals().mana, (u64::from(facts.max_mana) * 30 / 100) as u32);
                    assert_eq!(next.wheel_gift_save(now, false), Some((1, 10 * 60 * 60 * 1_000_000)));
                    *state = next;
                }
                assert!(owner.save_wheel_gift(deps.actor, session, false).await);
                let (snapshot_revision, remaining, occurrence): (i64, i64, String) = sqlx::query_as(
                    "SELECT revision,remaining_micros,occurrence::text FROM game_character_wheel_gift_state WHERE character_id=encode($1,'hex')::uuid")
                    .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(snapshot_revision, 1);
                let remaining = u64::try_from(remaining)?;
                assert!(remaining > 0 && remaining <= 10 * 60 * 60 * 1_000_000);
                assert_eq!(h.root.read_character_wheel_gift(&authority, fence.character_id).await.map_err(debug)?, remaining);
                assert!(owner.save_wheel_gift(deps.actor, session, false).await);
                let occurrence = crate::durability::character_wheel::WheelOccurrence::from_bytes(
                    u128::from_str_radix(&occurrence.replace('-', ""), 16)?.to_be_bytes()
                ).map_err(debug)?;
                let mut stale = original_fence;
                stale.character_lease_generation += 1;
                h.root.save_character_wheel_gift(&authority, &h.node, stale, occurrence, remaining)
                    .await.map_err(debug)?;
                assert!(matches!(
                    h.root.save_character_wheel_gift(&authority, &h.node, original_fence, occurrence, remaining - 1).await,
                    Err(crate::durability::character_progression::CharacterProgressionError::ConflictingOccurrence)
                ));
                let fresh_occurrence = crate::durability::character_wheel::WheelOccurrence::from_bytes(id(176)).map_err(debug)?;
                assert!(matches!(
                    h.root.save_character_wheel_gift(&authority, &h.node, stale, fresh_occurrence, remaining).await,
                    Err(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected)
                ));
                assert!(matches!(
                    h.root.save_character_wheel_gift(&authority, &h.node, original_fence, fresh_occurrence, 30 * 60 * 60 * 1_000_000 + 1).await,
                    Err(crate::durability::character_progression::CharacterProgressionError::InvalidInput)
                ));
                assert_eq!(h.root.read_character_wheel_gift(&authority, fence.character_id).await.map_err(debug)?, remaining);
                let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_wheel_gift_receipts WHERE character_id=encode($1,'hex')::uuid")
                    .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(receipts, 1, "replay and refusals leave exactly one cooldown receipt");
                assert_eq!(sqlx::query_scalar::<_, String>("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                    .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?, revision,
                    "cooldown snapshots do not mint a CharacterRevision");
                let mut tx = h.pool.begin().await?;
                assert!(sqlx::query("UPDATE game_character_wheel_gift_state SET revision=revision+1,remaining_micros=0 WHERE character_id=encode($1,'hex')::uuid")
                    .bind(fence.character_id.as_bytes().as_slice()).execute(&mut *tx).await.is_err(),
                    "the state guard refuses an unreceipted cooldown replacement");
                tx.rollback().await?;
                assert_gift_runtime_guards(&h).await?;
                Some(assert_gift_snapshot_rollback(&h, &authority, original_fence, remaining).await?)
            } else {
                None
            };
            let mut restarted = deps.owner(&h, &authority);
            restarted.clock_origin = owner.clock_origin;
            restarted.wheel_ruleset = Some(Arc::clone(&ruleset));
            restarted.premium_coordinator = Some(&gate);
            restarted.rooms = rooms;
            restarted.admit_premium(id(40), session);
            super::tutorial_vitals_fixture::admit(&restarted, &deps, &admission_request).await?;
            restarted.admit_wheel_session(session).await;
            admitted.continuity = crate::gameplay_transport::connection::SessionContinuity::FRESH;
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                restarted.offered_capabilities(), &[wire::CAPABILITY_WHEEL_ALLOCATION_V1],
            ).ok_or("Wheel selection")?;
            let reloaded = restarted.states.atelier_sessions.lock().map_err(|_| "Atelier cache")?
                .get(&session).cloned().flatten().ok_or("reloaded Atelier")?;
            assert!(reloaded.gems[&gem.id].vessel);
            assert_eq!(send(&restarted, &mut admitted, &[8, 0]).await?, reply(false, 1, points)?,
                "legacy query shape remains unchanged with socketed gems");
            if let Some(remaining) = gift_remaining {
                assert!(restarted.load_wheel_gift(deps.actor, session).await);
                let now = restarted.owner_now().get();
                let runtime = deps.runtime.lock().await;
                let mut states = restarted.states.spell_states.lock().await;
                let state = states.get_mut(&runtime, deps.actor, session).ok_or("reloaded Gift actor")?;
                let (_, reloaded) = state.wheel_gift_save(now, true).ok_or("loaded Gift remainder")?;
                assert!(reloaded > 0 && reloaded <= remaining);
                state.health = (state.character_facts().max_health / 10).max(1);
                let (_, hit) = crate::spell::actor_conditions::stage_creature_hit(state, state.health, now).map_err(debug)?;
                assert_eq!(hit.health_after, 0, "relogin cannot refresh the Gift cooldown");
            }
            let (final_vessels, final_revision) = match transition {
                WheelExtensionCase::Socket | WheelExtensionCase::Gift => {
                    (vessels, 1)
                }
                WheelExtensionCase::Unsocket => {
                    next_fresh_wheel_action(&mut restarted)?;
                    restarted.rooms = SeamRooms::new(None, None);
                    let clear = wire::WheelAllocationRequest {
                        occurrence: id(175), expected_revision: 1, vessels: Some([None; 4]), ..save.clone()
                    };
                    let clear = wire::encode_wheel_request(&clear).map_err(debug)?;
                    assert_eq!(send(&restarted, &mut admitted, &clear).await?, expected(false, 2, [None; 4])?);
                    assert_eq!(send(&restarted, &mut admitted, &clear).await?, expected(false, 2, [None; 4])?);
                    restarted.admit_wheel_session(session).await;
                    let reloaded = restarted.states.atelier_sessions.lock().map_err(|_| "Atelier cache")?
                        .get(&session).cloned().flatten().ok_or("reloaded cleared Atelier")?;
                    assert!(reloaded.gems.values().all(|gem| !gem.vessel));
                    assert_eq!(send(&restarted, &mut admitted, &[8, 0]).await?, reply(false, 2, points)?);
                    ([None; 4], 2)
                }
                WheelExtensionCase::Capacity => return Err("capacity case already handled".into()),
            };
            let conflicting = wire::WheelAllocationRequest {
                vessels: Some([None; 4]), ..save
            };
            assert_eq!(send(&restarted, &mut admitted, &wire::encode_wheel_request(&conflicting).map_err(debug)?).await?.0,
                CommandStatus::Rejected, "one receipt identity cannot be reused for another vessel vector");
            let committed = h.root.read_character_wheel(&authority, fence.character_id, ruleset)
                .await.map_err(debug)?;
            assert_eq!(committed.wheel_revision, final_revision);
            let stored = h.root.read_character_atelier(&authority, fence.character_id)
                .await.map_err(debug)?.ok_or("final Atelier")?;
            assert_eq!(stored.gems[&gem.id].vessel, final_vessels[0].is_some());
            let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_wheel_receipts WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(receipts, i64::try_from(final_revision)?);
            match transition {
                WheelExtensionCase::Socket => eprintln!("WHEEL_SOCKET POSTGRES_CASE_COMPLETED"),
                WheelExtensionCase::Unsocket => eprintln!("WHEEL_UNSOCKET POSTGRES_CASE_COMPLETED"),
                WheelExtensionCase::Gift => eprintln!("WHEEL_GIFT POSTGRES_CASE_COMPLETED"),
                WheelExtensionCase::Capacity => return Err("capacity case already handled".into()),
            }
            drop(restarted);
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        let request = wire::WheelAllocationRequest {
            action: 1,
            occurrence: id(160),
            expected_revision: 0,
            points,
            vessels: None,
        };
        let bytes = wire::encode_wheel_request(&request).map_err(debug)?;
        let original_fence = admitted.item_fence;
        let mut stale = original_fence.ok_or("item fence")?;
        stale.character_lease_generation += 1;
        admitted.item_fence = Some(stale);
        assert_eq!(
            send(&owner, &mut admitted, &bytes).await?.0,
            CommandStatus::Rejected
        );
        admitted.item_fence = original_fence;
        assert_eq!(
            send(&owner, &mut admitted, &bytes).await?,
            reply(false, 1, points)?
        );
        let (allocated_revision, allocated_vitals) = owner
            .observe_actor_vitals(deps.actor, session)
            .await
            .ok_or("allocated vitals")?;
        assert!(
            allocated_vitals.max_health > facts.max_health
                || allocated_vitals.max_mana > facts.max_mana,
            "the production allocation route installs dedication in the stats owner"
        );
        assert_eq!(
            send(&owner, &mut admitted, &bytes).await?,
            reply(false, 1, points)?,
            "receipt replay retains the committed allocation"
        );
        assert_eq!(
            owner.observe_actor_vitals(deps.actor, session).await,
            Some((allocated_revision, allocated_vitals)),
            "receipt replay does not stack dedication or change vitals"
        );
        owner.premium_coordinator = Some(&free);
        assert_eq!(
            send(&owner, &mut admitted, &[8, 0]).await?,
            reply(false, 1, points)?
        );
        let unpaid_vitals = owner
            .observe_actor_vitals(deps.actor, session)
            .await
            .ok_or("unpaid vitals")?
            .1;
        assert_eq!(
            (unpaid_vitals.max_health, unpaid_vitals.max_mana),
            (facts.max_health, facts.max_mana),
            "Premium loss removes dedication without changing the allocation"
        );
        next_fresh_wheel_action(&mut owner)?;
        let mut unpaid = request.clone();
        unpaid.occurrence = id(165);
        unpaid.expected_revision = 1;
        assert_eq!(
            send(
                &owner,
                &mut admitted,
                &wire::encode_wheel_request(&unpaid).map_err(debug)?
            )
            .await?,
            reply(true, 1, points)?
        );
        owner.premium_coordinator = Some(&gate);
        assert_eq!(
            send(&owner, &mut admitted, &[8, 0]).await?,
            reply(false, 1, points)?
        );
        let restored_vitals = owner
            .observe_actor_vitals(deps.actor, session)
            .await
            .ok_or("restored vitals")?
            .1;
        assert_eq!(
            (restored_vitals.max_health, restored_vitals.max_mana),
            (allocated_vitals.max_health, allocated_vitals.max_mana),
            "Premium restoration reapplies dedication without relogin"
        );
        next_fresh_wheel_action(&mut owner)?;
        let mut invalid = request.clone();
        invalid.occurrence = id(161);
        invalid.expected_revision = 1;
        invalid.points[35] = 1; // Outer slice without a full predecessor.
        assert_eq!(
            send(
                &owner,
                &mut admitted,
                &wire::encode_wheel_request(&invalid).map_err(debug)?
            )
            .await?,
            reply(true, 1, points)?
        );
        next_fresh_wheel_action(&mut owner)?;
        invalid.occurrence = id(164);
        for (index, slot) in data.wheel()["topology"]
            .as_array()
            .ok_or("topology")?
            .iter()
            .enumerate()
        {
            invalid.points[index] = u16::try_from(slot["capacity"].as_u64().ok_or("capacity")?)?;
        }
        assert_eq!(
            send(
                &owner,
                &mut admitted,
                &wire::encode_wheel_request(&invalid).map_err(debug)?
            )
            .await?,
            reply(true, 1, points)?,
            "4000 points exceed level 2000's budget"
        );
        next_fresh_wheel_action(&mut owner)?;
        let mut reset = wire::WheelAllocationRequest {
            action: 2,
            occurrence: id(162),
            expected_revision: 1,
            points: [0; 36],
            vessels: None,
        };
        let rooms = owner.rooms;
        owner.rooms = SeamRooms::new(None, None);
        assert_eq!(
            send(
                &owner,
                &mut admitted,
                &wire::encode_wheel_request(&reset).map_err(debug)?
            )
            .await?,
            reply(true, 1, points)?,
            "reset action cannot manufacture temple authority"
        );
        owner.rooms = rooms;
        // A fresh composed owner reloads the committed allocation through admission.
        let mut restarted = deps.owner(&h, &authority);
        restarted.clock_origin = owner.clock_origin;
        restarted.wheel_ruleset = Some(ruleset);
        restarted.premium_coordinator = Some(&gate);
        restarted.rooms = rooms;
        restarted.admit_premium(id(40), session);
        super::tutorial_vitals_fixture::admit(&restarted, &deps, &admission_request).await?;
        restarted.admit_wheel_session(session).await;
        admitted.continuity = crate::gameplay_transport::connection::SessionContinuity::FRESH;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            restarted.offered_capabilities(),
            &[wire::CAPABILITY_WHEEL_ALLOCATION_V1],
        )
        .ok_or("Wheel selection")?;
        assert_eq!(
            send(&restarted, &mut admitted, &[8, 0]).await?,
            reply(false, 1, points)?,
            "admission reloads the durable allocation"
        );
        let reloaded_vitals = restarted
            .observe_actor_vitals(deps.actor, session)
            .await
            .ok_or("reloaded vitals")?
            .1;
        assert_eq!(
            (reloaded_vitals.max_health, reloaded_vitals.max_mana),
            (allocated_vitals.max_health, allocated_vitals.max_mana),
            "reloaded allocation restores dedication in the stats owner"
        );
        next_fresh_wheel_action(&mut restarted)?;
        reset.occurrence = id(163);
        assert_eq!(
            send(
                &restarted,
                &mut admitted,
                &wire::encode_wheel_request(&reset).map_err(debug)?
            )
            .await?,
            reply(false, 2, [0; 36])?
        );
        let reset_vitals = restarted
            .observe_actor_vitals(deps.actor, session)
            .await
            .ok_or("reset vitals")?
            .1;
        assert_eq!(
            (reset_vitals.max_health, reset_vitals.max_mana),
            (facts.max_health, facts.max_mana),
            "replacement removes dedication from the existing stats wire"
        );
        assert_eq!(
            send(&restarted, &mut admitted, &[8, 0, 24, 1]).await?.0,
            CommandStatus::Rejected
        );
        admitted.continuity.selected_capabilities = SelectedCapabilities::NONE;
        assert_eq!(
            send(&restarted, &mut admitted, &[8, 0]).await?.0,
            CommandStatus::Rejected
        );
        eprintln!(
            "WHEEL_ALLOCATION POSTGRES_CASE_COMPLETED"
        );
        drop(restarted);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn assert_gift_runtime_guards(h: &Harness) -> TestResult {
    for (table, can_update) in [
        ("public.game_character_wheel_gift_receipts", false),
        ("public.game_character_wheel_gift_state", true),
    ] {
        for (privilege, runtime_allowed) in [
            ("SELECT", true), ("INSERT", true), ("UPDATE", can_update),
            ("DELETE", false), ("TRUNCATE", false), ("REFERENCES", false), ("TRIGGER", false),
        ] {
            for (role, allowed) in [("oteryn_game_runtime", runtime_allowed), ("oteryn_game_control", false)] {
                let granted: bool = sqlx::query_scalar("SELECT has_table_privilege($1,$2,$3)")
                    .bind(role).bind(table).bind(privilege).fetch_one(&h.pool).await?;
                assert_eq!(granted, allowed, "{role} {privilege} on {table}");
            }
        }
        let public_grants: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_class c CROSS JOIN LATERAL aclexplode(coalesce(c.relacl,acldefault('r',c.relowner))) a WHERE c.oid=$1::regclass AND a.grantee=0")
            .bind(table).fetch_one(&h.pool).await?;
        assert_eq!(public_grants, 0, "PUBLIC has no Gift table access");
    }
    for (role, allowed) in [("oteryn_game_runtime", true), ("oteryn_game_control", false)] {
        let granted: bool = sqlx::query_scalar("SELECT has_function_privilege($1,'public.game_guard_wheel_gift_state()','EXECUTE')")
            .bind(role).fetch_one(&h.pool).await?;
        assert_eq!(granted, allowed, "{role} Gift guard execution");
    }
    let (security_definer, settings): (bool, Option<Vec<String>>) = sqlx::query_as(
        "SELECT prosecdef,proconfig FROM pg_proc WHERE oid='public.game_guard_wheel_gift_state()'::regprocedure")
        .fetch_one(&h.pool).await?;
    assert!(security_definer);
    assert_eq!(settings, Some(vec!["search_path=pg_catalog, public, pg_temp".into()]));
    let public_grants: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_proc p CROSS JOIN LATERAL aclexplode(coalesce(p.proacl,acldefault('f',p.proowner))) a WHERE p.oid='public.game_guard_wheel_gift_state()'::regprocedure AND a.grantee=0")
        .fetch_one(&h.pool).await?;
    assert_eq!(public_grants, 0, "PUBLIC cannot execute the security-definer guard");

    // Each real runtime statement must fail at the privilege boundary, rather than
    // incidentally hitting a constraint or the state-to-receipt foreign key.
    for statement in [
        "UPDATE game_character_wheel_gift_receipts SET remaining_micros=0",
        "DELETE FROM game_character_wheel_gift_receipts",
        "TRUNCATE game_character_wheel_gift_receipts",
    ] {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
        let result = sqlx::query(statement).execute(&mut *tx).await;
        tx.rollback().await?;
        let error = result.err().ok_or("runtime changed immutable Gift receipts")?;
        assert_eq!(error.as_database_error().and_then(|e| e.code()).as_deref(), Some("42501"), "{statement}");
    }
    Ok(())
}

async fn assert_gift_snapshot_rollback(
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    remaining: u64,
) -> TestResult<u64> {
    use crate::durability::character_progression::CharacterProgressionError;
    use crate::durability::character_wheel::WheelOccurrence;
    const STATE: &str = "SELECT revision,remaining_micros,occurrence::text FROM game_character_wheel_gift_state WHERE character_id=encode($1,'hex')::uuid";
    const RECEIPTS: &str = "SELECT count(*) FROM game_character_wheel_gift_receipts WHERE character_id=encode($1,'hex')::uuid";
    let character = fence.character_id.as_bytes().as_slice();
    let revision: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(character).fetch_one(&h.pool).await?;
    let second = WheelOccurrence::from_bytes(id(177)).map_err(debug)?;
    h.root.save_character_wheel_gift(authority, &h.node, fence, second, remaining).await.map_err(debug)?;
    let before: (i64, i64, String) = sqlx::query_as(STATE).bind(character).fetch_one(&h.pool).await?;
    assert_eq!((before.0, before.1), (2, i64::try_from(remaining)?), "a fresh snapshot must update the existing timer row");
    assert_eq!(sqlx::query_scalar::<_, i64>(RECEIPTS).bind(character).fetch_one(&h.pool).await?, 2);

    // Only this isolated test database receives the CHECK. The real writer inserts
    // its receipt first, so a rejected state successor must roll back that receipt.
    // Revision two remains valid; the next physical state write is refused.
    let third = WheelOccurrence::from_bytes(id(178)).map_err(debug)?;
    let next_remaining = remaining.checked_sub(1).filter(|value| *value > 0).ok_or("remaining online cooldown")?;
    sqlx::query("ALTER TABLE game_character_wheel_gift_state ADD CONSTRAINT wheel_gift_test_refuse_revision_three CHECK (revision <> 3)")
        .execute(&h.pool).await?;
    let refused = h.root.save_character_wheel_gift(authority, &h.node, fence, third, next_remaining).await;
    // Remove the fixture restriction before inspecting the result, even if the
    // writer unexpectedly accepted the operation. No production seam is changed.
    sqlx::query("ALTER TABLE game_character_wheel_gift_state DROP CONSTRAINT wheel_gift_test_refuse_revision_three")
        .execute(&h.pool).await?;
    assert!(matches!(&refused, Err(CharacterProgressionError::Unavailable(crate::durability::DurabilityError::Database(_)))), "the physical state write must fail: {refused:?}");
    let after: (i64, i64, String) = sqlx::query_as(STATE).bind(character).fetch_one(&h.pool).await?;
    assert_eq!(after, before, "a rejected state write preserves the previous snapshot");
    assert_eq!(sqlx::query_scalar::<_, i64>(RECEIPTS).bind(character).fetch_one(&h.pool).await?, 2);
    let retained: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_wheel_gift_receipts WHERE character_id=encode($1,'hex')::uuid AND occurrence=encode($2,'hex')::uuid)")
        .bind(character).bind(third.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert!(!retained, "the rejected state write cannot leave a success receipt");
    assert_eq!(sqlx::query_scalar::<_, String>("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(character).fetch_one(&h.pool).await?, revision);

    // Unregistered database errors are intentionally classified at DurabilityRoot.
    // Repeating the identical request after removing only the CHECK proves the
    // failure was not stale authority, receipt rebinding, or the INSERT guard.
    h.root.save_character_wheel_gift(authority, &h.node, fence, third, next_remaining).await.map_err(debug)?;
    let after: (i64, i64, bool) = sqlx::query_as("SELECT revision,remaining_micros,occurrence=encode($2,'hex')::uuid FROM game_character_wheel_gift_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(character).bind(third.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(after, (3, i64::try_from(next_remaining)?, true));
    assert_eq!(sqlx::query_scalar::<_, i64>(RECEIPTS).bind(character).fetch_one(&h.pool).await?, 3);
    assert_eq!(sqlx::query_scalar::<_, String>("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(character).fetch_one(&h.pool).await?, revision);
    Ok(next_remaining)
}
