//! TASKBOARD-1: admitted connection bytes select a kill task, a real auto-attack rat kill settled
//! by `drain_kill_rewards` credits it, and a claim grants the task's experience and points once.
//! The setup is the kill reward end-to-end fixture's (production generation, Thalom open cells).
#![allow(clippy::expect_used)]

use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::task_board;
use oteryn_protocol_oteryn::task_board as wire;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

use super::fixture::{Harness, debug, id, run};
use super::{TestResult, prepare_current_session};
use crate::foundation::{
    ChannelId, ChannelRuntimeV1, CommandStatus, GameSessionId, MessageType, MovementLocalPosition,
    NodeId, WorldId, decode_wire_envelope,
};
use crate::gameplay_transport::capabilities::{
    PRODUCTION_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, ControllerBinding, FirstEntryOutcome, IDLE_LIVENESS,
    SessionContinuity, serve_admitted,
};
use crate::gameplay_transport::kill_reward::DrainOutcome;
use crate::gameplay_transport::{ComposedFreshAdmission, character_gameplay_fence};
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;

const OPEN_PLAYER: MovementLocalPosition = MovementLocalPosition {
    x: 2,
    y: 11,
    floor: -7,
};
const OPEN_RAT: MovementLocalPosition = MovementLocalPosition {
    x: 1,
    y: 11,
    floor: -7,
};
const PASS: Duration = Duration::from_secs(3);
const RAT_XP: i64 = 5;

#[test]
fn task_board_real_dispatch_selects_credits_a_real_kill_and_claims_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("task-board-e2e", scenario)
}

fn request(action: u8, task: u8) -> Vec<u8> {
    wire::encode_task_board_request(&wire::TaskBoardRequest { action, task }).expect("request")
}

/// One connection continuing this session's command stream from `first`.
async fn serve_from(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    first: u64,
    payloads: &[Vec<u8>],
) -> TestResult<Vec<(u64, CommandStatus, Vec<u8>)>> {
    let mut input = Vec::new();
    for (command_id, payload) in (first..).zip(payloads) {
        let bytes = oteryn_protocol_oteryn::encode_client_command(
            1,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id,
                command_type: wire::COMMAND_TYPE_TASK_BOARD,
                payload,
            },
        )
        .map_err(debug)?;
        input.extend(u32::try_from(bytes.len())?.to_be_bytes());
        input.extend(bytes);
    }
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client.write_all(&input).await?;
    client.shutdown().await?;
    let end = serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS).await;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _) = &end
    {
        *admitted = *current;
    }
    let mut cursor = 0;
    let mut values = Vec::new();
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        let envelope =
            decode_wire_envelope(&output[cursor + 4..cursor + 4 + length]).map_err(debug)?;
        cursor += length + 4;
        if envelope.message_type() == MessageType::CommandResult {
            let value =
                oteryn_protocol_oteryn::decode_command_result(envelope.payload()).map_err(debug)?;
            values.push((value.command_id, value.status, value.payload.to_vec()));
        }
    }
    assert_eq!(values.len(), payloads.len(), "{end:?}; {values:?}");
    Ok(values)
}

/// One accepted board command; returns its decoded board.
async fn board(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    action: u8,
    task: u8,
) -> TestResult<wire::TaskBoardReply> {
    let first = admitted.continuity.next_command_id;
    let values = serve_from(owner, admitted, first, &[request(action, task)]).await?;
    assert_eq!(values[0].1, CommandStatus::Accepted, "{values:?}");
    wire::decode_task_board_reply(&values[0].2).map_err(debug)
}

async fn experience(harness: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state")
            .fetch_one(&harness.pool)
            .await?,
    )
}

async fn quest_receipts(harness: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&harness.pool)
            .await?,
    )
}

#[allow(clippy::too_many_lines)]
fn scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "task_board_e2e", false, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let channel = ChannelId::decode(&id(43)).map_err(debug)?;
        let session = GameSessionId::decode(&id(50)).map_err(debug)?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                world,
                Some(include_bytes!(
                    "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
                )),
            );
        let active = controller.active().expect("activated generation");
        let native = active.native_gameplay().expect("native gameplay state");
        let room = pin.qualified_room().clone();
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            room.map_revision_digest(),
            (OPEN_PLAYER.x, OPEN_PLAYER.y, OPEN_PLAYER.floor),
        );
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3::text::numeric(20,0), NULL, $4, $5, $6)")
            .bind(id(42).as_slice())
            .bind(id(43).as_slice())
            .bind(pin.activation_sequence().to_string())
            .bind(pin.identity().server_artifact_digest().as_slice())
            .bind(pin.identity().client_artifact_digest().as_slice())
            .bind(pin.frame_binding().digest().as_slice())
            .execute(&harness.pool)
            .await?;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            channel_pin.clone(),
        )
        .map_err(debug)?;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                session,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                    2,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        runtime
            .realize_native_qualification_spawn(channel_pin, "oteryn:creature.rat", OPEN_RAT)
            .map_err(debug)?;
        let rat = {
            let creature = runtime
                .visible_entities()
                .creatures
                .first()
                .copied()
                .expect("the rat is visible");
            EntityRef {
                identity: creature.actor.placement_identity(),
                generation: creature.generation,
            }
        };
        let door_content =
            crate::world_runtime::ReferenceContentGeneration::from_content(room.door())
                .map_err(debug)?;
        let fence = super::fixture::fence(1)?;
        let door_fence = crate::world_runtime::ScopeContentGenerationFence::for_test(
            fence.runtime_scope,
            fence.scope_ownership_generation,
            door_content,
        );
        let door = crate::world_runtime::bind_native_entry_door(
            room.door(),
            &door_fence,
            fence.runtime_scope,
            fence.scope_ownership_generation,
        )
        .map_err(debug)?;
        let descriptor = crate::native_admission_source::descriptor::ProducerDescriptor::new(
            "platform-test".into(),
            ("127.0.0.1".into(), 1),
            "source.test".into(),
            "source.test".into(),
            vec![rustls::pki_types::CertificateDer::from(
                include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/ca.der").to_vec(),
            )],
            vec![rustls::pki_types::CertificateDer::from(
                include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client.der").to_vec(),
            )],
            rustls::pki_types::PrivateKeyDer::try_from(
                include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client-key.der")
                    .to_vec(),
            )?,
        )
        .map_err(debug)?;
        let evidence =
            crate::gameplay_transport::fresh_evidence::FreshEvidenceSource::new(descriptor);
        let spells = crate::spell::cast::v1_spell_book().map_err(debug)?;
        let achievements =
            crate::achievement_catalogue::AchievementCatalogue::embedded().map_err(debug)?;
        let charms =
            crate::content::charm_source::CanonicalCharmCatalogue::embedded().map_err(debug)?;
        let runtime = Mutex::new(runtime);
        let door = Mutex::new(door);
        let mut admitted = AdmittedSession {
            game_session_id: session,
            world_id: world,
            channel_id: channel,
            runtime_actor: Some(actor),
            first_entry: FirstEntryOutcome::NotApplicable,
            controller: Some(ControllerBinding {
                transport: crate::foundation::AuthenticatedTransportRefV1::decode(&[9; 16])
                    .map_err(debug)?,
                account_id: id(40),
            }),
            continuity: SessionContinuity::FRESH,
            item_fence: None,
        };
        let mut owner = crate::gameplay_transport::composed_test_owner::ComposedTestOwner {
            root: &harness.root,
            character: &authority,
            holder: &harness.node,
            evidence: &evidence,
            world_id: world,
            channel_id: channel,
            required_capabilities: crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
                &[],
                crate::foundation::RuntimeScopeRefV1::channel(world, channel),
                1,
                "policy-1",
                crate::gameplay_transport::world_required_capabilities::offered_for_scope(false, None),
            )
            .expect("an empty declared set binds"),
            runtime: &runtime,
            door: &door,
            spells: &spells,
            achievements: &achievements,
            imported_charms: &charms,
            room: &room,
        }.build();
        owner.active_generation = Some(active);
        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
        owner.clock_origin = std::time::Instant::now()
            .checked_sub(Duration::from_secs(60))
            .expect("a monotonic clock older than a minute");
        {
            let runtime = owner.runtime.lock().await;
            assert!(
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .initialize(
                        &runtime,
                        actor,
                        session,
                        crate::spell::cast::CharacterCastFacts {
                            vocation: crate::spell::Vocation::Monk,
                            ..crate::gameplay_transport::actor_spell::tests::FACTS
                        },
                        (3, 7_000_000),
                        owner.owner_now(),
                    )
                    .is_some()
            );
        }
        owner.admit_progression_session(&mut admitted).await;
        assert!(owner.progression_binding(session).is_some());

        // The boot catalogue as `load_quest_catalogue` builds it: embedded quests plus the board.
        let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?;
        owner.quest_catalogue = Some(std::sync::Arc::new(
            task_board::extend(lowered.catalogue()).map_err(debug)?,
        ));
        owner.refresh_quest_session(session, true).await;
        let current = character_gameplay_fence::current(&owner, session, false)
            .await
            .ok_or("current fence")?;
        admitted.item_fence = Some(current.item_fence());
        admitted.first_entry = FirstEntryOutcome::Positioned;

        // Without the selected capability the command is no Task Board command.
        let first = admitted.continuity.next_command_id;
        let values = serve_from(&owner, &mut admitted, first, &[request(0, 0)]).await?;
        assert_ne!(values[0].1, CommandStatus::Accepted, "{values:?}");
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            PRODUCTION_OFFERED_CAPABILITIES,
            &[1, wire::CAPABILITY_TASK_BOARD_V1],
        )
        .ok_or("selection")?;

        // Query: the empty board of five tasks.
        let empty = board(&owner, &mut admitted, 0, 0).await?;
        assert_eq!((empty.refused, empty.selected, empty.kills, empty.points), (false, 0, 0, 0));
        assert_eq!(empty.tasks.len(), 5);
        assert_eq!(empty.tasks[0].race, "oteryn:creature.rat");

        // Select the rat task; a claim before its kills is refused and writes nothing.
        let selected = board(&owner, &mut admitted, 1, 1).await?;
        assert_eq!((selected.refused, selected.selected, selected.kills), (false, 1, 0));
        let receipts = quest_receipts(&harness).await?;
        let early = board(&owner, &mut admitted, 2, 1).await?;
        assert!(early.refused);
        assert_eq!((early.selected, early.points), (1, 0));
        assert_eq!(quest_receipts(&harness).await?, receipts);
        // Another task cannot be selected while one is active.
        assert!(board(&owner, &mut admitted, 1, 2).await?.refused);

        // A real rat kill: auto-attack swings, then the durable kill settle credits the task.
        {
            let runtime = owner.runtime.lock().await;
            let now = owner.owner_now();
            assert_eq!(
                owner.states.attack.lock().await.set_target(
                    &runtime,
                    Some(&room),
                    actor,
                    session,
                    now,
                    Some(rat),
                    1
                ),
                oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
            );
        }
        let mut swings = 0;
        loop {
            owner.refresh_attack_weapon(actor, session).await;
            owner.drain_auto_attacks().await;
            if owner
                .states
                .attack
                .lock()
                .await
                .kills()
                .has_session(session)
            {
                break;
            }
            swings += 1;
            assert!(swings < 64, "the held rat dies to auto-attack swings");
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(PASS)
                .expect("a monotonic clock older than the passes");
        }
        let mut drains = 0;
        while owner.drain_kill_rewards(session).await != DrainOutcome::Drained {
            drains += 1;
            assert!(drains < 8, "the queued kill settles");
        }
        assert_eq!(experience(&harness).await?, RAT_XP);
        let credited = board(&owner, &mut admitted, 0, 0).await?;
        assert_eq!((credited.selected, credited.kills), (1, 1), "the real kill counts");

        // The remaining kills through the same settle hook, each its own death occurrence. A
        // repeated occurrence (a settle retry) never counts twice.
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence read")?
            .ok_or("live session")?;
        let rat_task = task_board::TASKS[0];
        for kill in 2..=rat_task.required {
            let occurrence = death_occurrence(u8::try_from(kill)?);
            owner
                .credit_task_board_kill(fence, rat_task.race, occurrence)
                .await;
            owner
                .credit_task_board_kill(fence, rat_task.race, occurrence)
                .await;
        }
        // A kill of another race credits nothing.
        owner
            .credit_task_board_kill(fence, task_board::TASKS[1].race, death_occurrence(0xEE))
            .await;
        let done = board(&owner, &mut admitted, 0, 0).await?;
        assert_eq!((done.selected, done.kills), (1, rat_task.required));

        // Claim: experience and task points once; the task is locked for the week.
        let claim_id = admitted.continuity.next_command_id;
        let claimed = board(&owner, &mut admitted, 2, 1).await?;
        assert!(!claimed.refused);
        assert_eq!((claimed.selected, claimed.kills), (0, 0));
        assert_eq!(claimed.points, task_board::TASK_POINTS);
        assert!(claimed.tasks[0].claimed_at_unix > 0);
        assert_eq!(
            experience(&harness).await?,
            RAT_XP + task_board::EXPERIENCE
        );
        // A lost acknowledgement: the same CommandId replays the retained result.
        let replay = serve_from(&owner, &mut admitted, claim_id, &[request(2, 1)]).await?;
        assert_eq!(replay[0].1, CommandStatus::DuplicateReplay, "{replay:?}");
        let receipts = quest_receipts(&harness).await?;
        // A second claim and a reselect inside the week are refused and write nothing.
        let again = board(&owner, &mut admitted, 2, 1).await?;
        assert!(again.refused);
        assert_eq!(again.points, task_board::TASK_POINTS);
        let relock = board(&owner, &mut admitted, 1, 1).await?;
        assert!(relock.refused);
        assert_eq!(relock.selected, 0);
        assert_eq!(quest_receipts(&harness).await?, receipts);
        assert_eq!(
            experience(&harness).await?,
            RAT_XP + task_board::EXPERIENCE
        );

        // The production router also owns bounty selection and claiming. Real death settlement
        // credits its active offer through the same fenced quest authority.
        let bounty = task_board::BOUNTY_OFFERS[0];
        assert!(!board(&owner, &mut admitted, 3, 1).await?.refused);
        let bounty_fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "bounty fence read")?
            .ok_or("live bounty session")?;
        for kill in 0..bounty.required {
            owner
                .credit_task_board_kill(
                    bounty_fence,
                    bounty.race,
                    death_occurrence(u8::try_from(kill + 80)?),
                )
                .await;
        }
        assert!(!board(&owner, &mut admitted, 4, 1).await?.refused);
        assert_eq!(
            experience(&harness).await?,
            RAT_XP + task_board::EXPERIENCE + bounty.experience
        );

        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await?;
        Ok(())
    })
}

/// A distinct UUIDv7-shaped death occurrence, as the kill settle memoizes one.
fn death_occurrence(tag: u8) -> [u8; 16] {
    let mut bytes = [tag; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}
