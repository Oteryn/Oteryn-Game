//! PARITY-PROGRESSION-QUERIES-1: admitted command 84 bytes -> native charm port -> durable
//! receipts, assignment and gold fee -> reply bytes and reload (capability 67).
#![allow(clippy::expect_used)]
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use super::charm_progression_fixtures::{seed};

use super::*;
use crate::domain::bestiary::BestiaryRace;
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::{
    PRODUCTION_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::charm::{CharmAssignDisposition, CharmUnlockDisposition};
use oteryn_protocol_oteryn::charm_progression::{
    self as wire, CharmProgressionAction, CharmProgressionOutcome, CharmProgressionReply,
    CharmRemoveDisposition,
};
use std::num::NonZeroU32;

/// One command on a fresh connection that continues this GameSession's command stream.
async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    action: &CharmProgressionAction,
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let payload = wire::encode_charm_progression_request(action).map_err(debug)?;
    exchange_payload(owner, admitted, payload).await
}

async fn exchange_payload(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    payload: Vec<u8>,
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let command_id = admitted.continuity.next_command_id;
    let bytes = oteryn_protocol_oteryn::encode_client_command(
        admitted.continuity.connection_generation,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id,
            command_type: wire::COMMAND_TYPE_CHARM_PROGRESSION,
            payload: &payload,
        },
    )
    .map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client
        .write_all(&[u32::try_from(bytes.len())?.to_be_bytes().as_slice(), &bytes].concat())
        .await?;
    client.shutdown().await?;
    let end = serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS).await;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    if let crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(current)
    | crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenClosed(current, _) = &end
    {
        *admitted = *current;
    }
    let mut cursor = 0;
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        let envelope =
            decode_wire_envelope(&output[cursor + 4..cursor + 4 + length]).map_err(debug)?;
        cursor += length + 4;
        if envelope.message_type() == MessageType::CommandResult {
            let value =
                oteryn_protocol_oteryn::decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(value.command_id, command_id);
            return Ok((value.status, value.payload.to_vec()));
        }
    }
    Err(format!("no result for command {command_id}: {end:?}").into())
}

async fn reply(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    action: CharmProgressionAction,
) -> TestResult<CharmProgressionReply> {
    let (status, bytes) = exchange(owner, admitted, &action).await?;
    assert_eq!(status, CommandStatus::Accepted, "{action:?}");
    wire::decode_charm_progression_reply(&bytes).map_err(debug)
}

#[test]
fn charm_progression_real_dispatch_unlocks_assigns_and_removes_for_the_gold_fee() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false, false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "charm progression scenario panicked")?
        .map_err(Into::into)
}

#[test]
fn charm_expansion_real_dispatch_discounts_removal_and_lifts_slot_limit() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(true, false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "charm expansion scenario panicked")?
        .map_err(Into::into)
}

fn scenario(expansion: bool, premium_current: bool) -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "charm_progression", true, false).await?;
        eprintln!("POSTGRES_STARTED P01C charm_progression_real_dispatch expansion={expansion}");
        prepare_current_session(&harness).await?;
        // Keep the original three-entry case; expansion also funds three major charms.
        let races = crate::gameplay_transport::charm_progression::canonical_races()
            .ok_or("canonical Bestiary")?;
        let completed: Vec<&BestiaryRace> = races
            .iter()
            .filter(|race| race.final_kill_threshold() == 5000)
            .take(if expansion || premium_current { 8 } else { 3 })
            .collect();
        seed(&harness, &completed).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (generation, pin) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(world, "content-1")?;
        let room = pin.qualified_room().clone();
        let (channel_pin, _, _, _) = pin.into_channel_parts();
        let dependencies = Dependencies::with_content(
            channel_pin,
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let (consumer, clock, activation, _) =
            crate::gameplay_transport::spell_premium_coordinator::purchase_fixture::current(id(40));
        if premium_current {
            harness
                .root
                .latch_premium_activation(
                    &activation.activation_id,
                    activation.switch_over_us,
                    activation.switch_over_us + 60_000_000,
                )
                .await
                .map_err(debug)?;
        }
        let premium =
            crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
                consumer,
                Some(clock),
                Some(activation),
                Some(harness.root.clone()),
            );
        let mut owner = dependencies.owner(&harness, &authority);
        if premium_current {
            owner.premium_coordinator = Some(&premium);
        }
        owner.active_generation = generation.active();
        dependencies.initialize(&owner).await;
        if premium_current {
            owner.admit_premium(id(40), dependencies.admitted.game_session_id);
        }
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(dependencies.actor)
            .map_err(debug)?;
        let current = crate::gameplay_transport::character_gameplay_fence::current(
            &owner,
            dependencies.admitted.game_session_id,
            false,
        )
        .await
        .ok_or("current fence")?;
        let mut admitted = dependencies.admitted;
        admitted.item_fence = Some(current.item_fence());
        admitted.first_entry = FirstEntryOutcome::Positioned;
        owner
            .runtime
            .lock()
            .await
            .bind_attacker_lease(
                dependencies.actor,
                admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(current.character_id.as_bytes())
                        .map_err(debug)?,
                    current.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        assert!(
            crate::gameplay_transport::battle_instinct_status::actor_is_current(
                &*owner.runtime.lock().await,
                admitted
            )
        );
        assert!(owner.charm_progression_available());
        // Unnegotiated: the command never reaches the charm port.
        let (status, _) = exchange(&owner, &mut admitted, &CharmProgressionAction::Query).await?;
        assert_eq!(status, CommandStatus::Rejected);
        let offer = crate::gameplay_transport::charm_progression::offered(
            PRODUCTION_OFFERED_CAPABILITIES,
            owner.charm_progression_available(),
        );
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            &offer,
            &[1, wire::CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1],
        )
        .ok_or("selection")?;

        if !expansion && !premium_current {
            catalogue::verify(&owner, &mut admitted, races).await?;
        }
        let named_dodge = if !expansion && !premium_current {
            Some(metadata::verify(&owner, &mut admitted).await?)
        } else {
            None
        };
        let query = reply(&owner, &mut admitted, CharmProgressionAction::Query).await?;
        assert_eq!(query.outcome, CharmProgressionOutcome::Query);
        assert!(
            query.charms.charms.iter().all(|charm| charm.effect_active),
            "the qualified native command84 reply advertises connected runtime consumers"
        );
        let earned = if expansion || premium_current {
            800
        } else {
            300
        };
        assert_eq!(query.charms.charm_points_available, earned);
        assert_eq!(
            query.charms.assignment_slot_limit,
            NonZeroU32::new(if premium_current { 6 } else { 2 })
        );
        assert!(query.charms.charms.iter().all(|c| c.unlocked_stage == 0));
        // The wire index is the position in race-key order.
        let mut keys: Vec<&str> = races.iter().map(BestiaryRace::key).collect();
        keys.sort_unstable();
        let race = |key: &str| {
            keys.binary_search(&key)
                .ok()
                .and_then(|at| NonZeroU32::new(u32::try_from(at + 1).ok()?))
                .ok_or("race index")
        };
        let first = race(completed[0].key())?;
        assert!(query.bestiary.iter().any(|p| p.race == first));
        if let Some(id) = named_dodge {
            let selected = query
                .charms
                .charms
                .iter()
                .find(|c| c.charm == id)
                .ok_or("named charm in authoritative view")?;
            assert_eq!(selected.next_stage_cost, 240);
            assert_eq!(
                selected.kind,
                oteryn_protocol_oteryn::charm::CharmKind::Major
            );
        }
        // The 240-point major charm (dodge).
        let charm = query
            .charms
            .charms
            .iter()
            .find(|c| c.next_stage_cost == 240)
            .ok_or("dodge")?
            .charm;

        let removed = reply(
            &owner,
            &mut admitted,
            CharmProgressionAction::Remove { charm },
        )
        .await?;
        assert_eq!(
            (removed.outcome, removed.fee_gold),
            (
                CharmProgressionOutcome::Remove(CharmRemoveDisposition::NotAssigned),
                0
            )
        );
        let unlocked = reply(
            &owner,
            &mut admitted,
            CharmProgressionAction::Unlock {
                charm,
                expected_stage: 0,
            },
        )
        .await?;
        assert_eq!(
            unlocked.outcome,
            CharmProgressionOutcome::Unlock(CharmUnlockDisposition::Unlocked)
        );
        assert_eq!(unlocked.charms.charm_points_available, earned - 240);
        let assigned = reply(
            &owner,
            &mut admitted,
            CharmProgressionAction::Assign { charm, race: first },
        )
        .await?;
        assert_eq!(
            assigned.outcome,
            CharmProgressionOutcome::Assign(CharmAssignDisposition::Assigned)
        );
        let state = |view: &CharmProgressionReply| {
            view.charms
                .charms
                .iter()
                .find(|c| c.charm == charm)
                .map(|c| c.assigned_race)
        };
        assert_eq!(state(&assigned), Some(Some(first)));
        if expansion || premium_current {
            let others: Vec<_> = query
                .charms
                .charms
                .iter()
                .filter(|entry| entry.next_stage_cost == 240 && entry.charm != charm)
                .map(|entry| entry.charm)
                .take(2)
                .collect();
            assert_eq!(others.len(), 2);
            for (index, other) in others.iter().enumerate() {
                let unlocked = reply(
                    &owner,
                    &mut admitted,
                    CharmProgressionAction::Unlock {
                        charm: *other,
                        expected_stage: 0,
                    },
                )
                .await?;
                assert_eq!(
                    unlocked.outcome,
                    CharmProgressionOutcome::Unlock(CharmUnlockDisposition::Unlocked)
                );
                let assigned = reply(
                    &owner,
                    &mut admitted,
                    CharmProgressionAction::Assign {
                        charm: *other,
                        race: race(completed[index + 1].key())?,
                    },
                )
                .await?;
                assert_eq!(
                    assigned.outcome,
                    CharmProgressionOutcome::Assign(if index == 0 || premium_current {
                        CharmAssignDisposition::Assigned
                    } else {
                        CharmAssignDisposition::AssignmentSlotsFull
                    })
                );
            }
            if !premium_current {
                sqlx::query(
                    "INSERT INTO game_character_charm_expansions(character_id) \
             VALUES(encode($1,'hex')::uuid)",
                )
                .bind(id(41).as_slice())
                .execute(&harness.pool)
                .await?;
                let expanded = reply(&owner, &mut admitted, CharmProgressionAction::Query).await?;
                assert_eq!(expanded.charms.assignment_slot_limit, None);
                let assigned = reply(
                    &owner,
                    &mut admitted,
                    CharmProgressionAction::Assign {
                        charm: others[1],
                        race: race(completed[2].key())?,
                    },
                )
                .await?;
                assert_eq!(
                    assigned.outcome,
                    CharmProgressionOutcome::Assign(CharmAssignDisposition::Assigned)
                );
            }
        }
        {
            use oteryn_gameplay_progression::charm_effects::CharmStateRead;
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let snapshot = states
                .charm_state(&runtime, dependencies.actor, admitted.game_session_id)
                .ok_or("combat snapshot after assignment")?;
            let assignments = snapshot
                .assignments_for_race(completed[0].key())
                .map_err(debug)?;
            assert_eq!(assignments.len(), 1);
            assert_eq!(
                assignments[0].unlocked_stage, 1,
                "unlock and assignment are live without reconnect"
            );
        }

        let level: i64 = sqlx::query_scalar("SELECT level FROM game_character_progression_state")
            .fetch_one(&harness.pool)
            .await?;
        let removed = reply(
            &owner,
            &mut admitted,
            CharmProgressionAction::Remove { charm },
        )
        .await?;
        assert_eq!(
            (removed.outcome, removed.fee_gold),
            (
                CharmProgressionOutcome::Remove(CharmRemoveDisposition::Removed),
                u64::try_from(level)? * if expansion { 75 } else { 100 }
            )
        );
        assert_eq!(state(&removed), Some(None), "the race is free again");
        {
            use oteryn_gameplay_progression::charm_effects::CharmStateRead;
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            assert!(
                states
                    .charm_state(&runtime, dependencies.actor, admitted.game_session_id)
                    .ok_or("combat snapshot after remove")?
                    .assignments_for_race(completed[0].key())
                    .map_err(debug)?
                    .is_empty()
            );
        }

        let durable: (i64, i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM game_character_charm_assignments), \
                    (SELECT count(*) FROM game_character_charm_receipts WHERE command_kind = 3), \
                    (SELECT sum(fee_gold_units)::bigint FROM game_item_fee_burns \
                      WHERE cause_kind = 1)",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(
            durable,
            (
                if expansion || premium_current { 2 } else { 0 },
                1,
                level * if expansion { 75 } else { 100 }
            )
        );
        // A second remove has nothing to free and pays nothing.
        let again = reply(
            &owner,
            &mut admitted,
            CharmProgressionAction::Remove { charm },
        )
        .await?;
        assert_eq!(
            again.outcome,
            CharmProgressionOutcome::Remove(CharmRemoveDisposition::NotAssigned)
        );
        // The charm can be assigned again after a remove (0195 assignment cycles).
        let reassigned = reply(
            &owner,
            &mut admitted,
            CharmProgressionAction::Assign {
                charm,
                race: race(completed[if expansion || premium_current { 3 } else { 1 }].key())?,
            },
        )
        .await?;
        assert_eq!(
            reassigned.outcome,
            CharmProgressionOutcome::Assign(CharmAssignDisposition::Assigned)
        );
        let reset = reply(&owner, &mut admitted, CharmProgressionAction::ResetAll).await?;
        assert_eq!(
            reset.outcome,
            CharmProgressionOutcome::ResetAll(CharmRemoveDisposition::Removed)
        );
        let expected_fee = crate::domain::charm::reset_all_fee(
            u64::try_from(level)?,
            if expansion {
                crate::domain::charm::CharmSlotEntitlement::CharmExpansion
            } else {
                crate::domain::charm::CharmSlotEntitlement::Free
            },
        )
        .ok_or("reset fee")?;
        assert_eq!(reset.fee_gold, expected_fee);
        assert_eq!(reset.charms.charm_points_available, earned);
        assert!(
            reset
                .charms
                .charms
                .iter()
                .all(|c| c.unlocked_stage == 0 && c.assigned_race.is_none())
        );
        assert_eq!(reset.charms.minor_charm_echoes_available, 0);
        let reloaded = reply(&owner, &mut admitted, CharmProgressionAction::Query).await?;
        assert_eq!(reloaded.charms, reset.charms);
        let bought_again = reply(
            &owner,
            &mut admitted,
            CharmProgressionAction::Unlock {
                charm,
                expected_stage: 0,
            },
        )
        .await?;
        assert_eq!(
            bought_again.outcome,
            CharmProgressionOutcome::Unlock(CharmUnlockDisposition::Unlocked)
        );
        let receipt_count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_charm_receipts WHERE command_kind=4",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(receipt_count, 1);
        if !expansion && !premium_current {
            catalogue::verify_echo(&owner, &mut admitted, races).await?;
        }
        eprintln!(
            "WB2_P01_RESET_PREMIUM_POSTGRES_COMPLETED expansion={expansion} premium={premium_current}"
        );
        Ok(())
    })
}

#[test]
fn charm_premium_real_dispatch_has_six_slots_without_expansion() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false, true).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "premium charm scenario panicked")?
        .map_err(Into::into)
}

#[path = "charm_progression/catalogue.rs"]
mod catalogue;

#[path = "charm_progression/metadata.rs"]
mod metadata;
