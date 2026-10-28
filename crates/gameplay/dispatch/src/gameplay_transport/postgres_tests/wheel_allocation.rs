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

fn reply(refused: bool, revision: u64, points: [u16; 36]) -> TestResult<(CommandStatus, Vec<u8>)> {
    Ok((
        CommandStatus::Accepted,
        wire::encode_wheel_reply(&wire::WheelAllocationReply {
            refused,
            revision,
            points,
        })
        .map_err(debug)?,
    ))
}

#[test]
fn wheel_allocation_postgres_real_dispatch_refusal_reset_and_relogin_allocation() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Wheel live PostgreSQL thread panic")??;
    Ok(())
}

fn scenario() -> TestResult {
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
        let (_controller, pin) =
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
        atelier_fixture::higher_level(&h).await?;
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
        let request = wire::WheelAllocationRequest {
            action: 1,
            occurrence: id(160),
            expected_revision: 0,
            points,
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
        let mut reset = wire::WheelAllocationRequest {
            action: 2,
            occurrence: id(162),
            expected_revision: 1,
            points: [0; 36],
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
            "W-2 PostgreSQL dispatch allocate/refuse/replay/reset/reload allocation verified"
        );
        drop(restarted);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
