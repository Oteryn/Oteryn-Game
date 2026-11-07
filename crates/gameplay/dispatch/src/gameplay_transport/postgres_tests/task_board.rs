//! TASKBOARD-1: expanded commands use real admitted bytes, fenced receipts, custody,
//! a real auto-attack kill, completion XP and the committed reset owner.
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
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, ControllerBinding, FirstEntryOutcome, IDLE_LIVENESS,
    SessionContinuity, serve_admitted,
};
use crate::gameplay_transport::kill_reward::DrainOutcome;
use crate::gameplay_transport::{ComposedFreshAdmission, character_gameplay_fence};
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;

#[path = "task_board/scenarios.rs"]
mod scenarios;
use scenarios::*;
#[path = "task_board/capacity_case.rs"]
mod capacity_case;
#[path = "task_board/cosmetic_cases.rs"]
mod cosmetic_cases;
#[path = "task_board/new_balance_case.rs"]
mod currency_case;
#[path = "task_board/depot_case.rs"]
mod depot_case;
#[path = "task_board/progression_case.rs"]
mod progression_case;
#[path = "task_board/token_case.rs"]
mod token_case;
#[path = "task_board/rollback_cases.rs"]
mod rollback_cases;
use rollback_cases::{RollbackFault, assert_unavailable};

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
// The admitted level is eight: the existing hunting XP owner applies
// floor(5 * (1200 + (1200 - 25 * (8 - 2))) / 1200) = 9.
const RAT_XP_AT_LEVEL_8: i64 = 9;

#[test]
fn task_board_real_dispatch_weekly_bounty_custody_reset_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("<TASK_BOARD> POSTGRES_CASE_SKIPPED: OTERYN_TEST_POSTGRES_ADMIN_URL is not set");
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("task-board-e2e", scenario)
}

fn request(action: u8, task: u8) -> Vec<u8> {
    wire::encode_task_board_request(&wire::TaskBoardRequest { action, task }).expect("request")
}

type CommandReplies = Vec<(u64, CommandStatus, Vec<u8>)>;

/// Normal exchanges must retain one terminal result for every supplied command.
async fn serve_from(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    first: u64,
    payloads: &[Vec<u8>],
) -> TestResult<CommandReplies> {
    let (end, values) = serve_attempt(owner, admitted, first, payloads).await?;
    assert_eq!(values.len(), payloads.len(), "{end:?}; {values:?}");
    Ok(values)
}

/// The same real connection exchange also exposes its end for isolated rollback cases.
async fn serve_attempt(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    first: u64,
    payloads: &[Vec<u8>],
) -> TestResult<(ConnectionEnd, CommandReplies)> {
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
        assert_eq!(current.game_session_id, admitted.game_session_id);
        assert_eq!(
            current.continuity.connection_generation,
            admitted.continuity.connection_generation
        );
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
    Ok((end, values))
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

fn scenario() -> TestResult {
    scenario_with_fault(None)
}

#[allow(clippy::too_many_lines)]
fn scenario_with_fault(fault: Option<RollbackFault>) -> TestResult {
    run(async move |admin| {
        let tag = fault.map_or("task_board_e2e", RollbackFault::tag);
        let harness =
            Harness::create_with_admission_seed(admin, tag, false, false).await?;
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
        let cosmetics = cosmetic_cases::cosmetics_packet()?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::cosmetics_fixture::activated_with_optional_cosmetics(
                world,
                Some(include_bytes!(
                    "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
                )),
                Some(&cosmetics),
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
            // This tests task credit, not combat odds. A trained fist makes even the
            // minimum real swing lethal after the production rat's defence and armor,
            // independent of the content digest that seeds the swing draws.
            let mut skills = [(10, 0); 7];
            skills[0] = (1_000, 0);
            let build = crate::durability::character_build::DurableBuildState::new(
                "monk",
                (
                    crate::gameplay_transport::actor_spell::tests::FACTS
                        .magic_level
                        .try_into()?,
                    0,
                ),
                skills,
            )
            .map_err(debug)?;
            assert!(
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .load_owned_build(&runtime, actor, session, &build,)
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

        inactive_death_without_calendar(&owner, &harness, &admitted).await?;
        // The existing reset owner supplies period identity. Initializer is owner-only;
        // this fixture calls it with the actual active native generation's digests.
        sqlx::query("SELECT game_world_reset_initialize(encode($1,'hex')::uuid,$2,$3,$2,$4,$5,'TIBIA_SERVER_SAVE_BERLIN_06',1)")
            .bind(world.as_bytes().as_slice())
            .bind(pin.identity().server_artifact_digest().as_slice()).bind("taskboard-test-map")
            .bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice())
            .execute(&harness.pool).await?;

        // Without the selected capability the command is no Task Board command.
        let first = admitted.continuity.next_command_id;
        let values = serve_from(&owner, &mut admitted, first, &[request(0, 0)]).await?;
        assert_ne!(values[0].1, CommandStatus::Accepted, "{values:?}");
        // Expanded Task Board is intentionally offered only in bundle-world rollout.
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[1, wire::CAPABILITY_TASK_BOARD_V1],
        )
        .ok_or("selection")?;

        let legacy = board(&owner, &mut admitted, 0, 0).await?;
        assert!(!legacy.refused);
        assert!(
            legacy.tasks.is_empty(),
            "the retired fixed board cannot remain a second implementation"
        );
        assert!(board(&owner, &mut admitted, 1, 1).await?.refused);
        assert!(board(&owner, &mut admitted, 2, 1).await?.refused);
        assert_eq!(
            full_board(&owner, &mut admitted, 5, 0, "").await?.refusal,
            3,
            "level one cannot start Beginner tasks"
        );
        // This Character entered the fixture at level one; accepted progression at
        // level eight is now fixture input. All task mutations below use real dispatch.
        progression_case::fixture_admitted_level(&owner, &harness, 8).await?;
        capacity_case::global_quest_capacity(&owner, &harness, &mut admitted).await?;
        let start_id = admitted.continuity.next_command_id;
        let started = full_board(&owner, &mut admitted, 5, 0, "").await?;
        assert_eq!(started.refusal, 0);
        let expected_hash = owner
            .quest_catalogue
            .as_ref()
            .and_then(|catalogue| catalogue.definition_hash(task_board::QUEST))
            .ok_or("boot Task Board definition hash")?;
        let stored_hash: Vec<u8> = sqlx::query_scalar(
            "SELECT definition_hash FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2")
            .bind(id(41).as_slice()).bind(task_board::QUEST)
            .fetch_one(&harness.pool).await?;
        assert_eq!(
            stored_hash.as_slice(),
            expected_hash.as_slice(),
            "the durable quest pin must identify the actual boot catalogue definition"
        );
        assert_eq!(
            (started.weekly.kills.len(), started.weekly.deliveries.len()),
            (6, 6)
        );
        assert!(
            started
                .weekly
                .kills
                .iter()
                .filter(|task| !task.race.is_empty())
                .all(|task| (50..=100).contains(&task.required) && task.current == 0)
        );
        let replay = serve_from(&owner, &mut admitted, start_id, &[full_request(5, 0, "")]).await?;
        assert_eq!(replay[0].1, CommandStatus::DuplicateReplay);
        assert_eq!(
            wire::full::decode_reply(&replay[0].2).map_err(debug)?,
            started
        );
        assert_eq!(
            full_board(&owner, &mut admitted, 5, 0, "").await?.refusal,
            2
        );
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence read")?
            .ok_or("current fence")?;
        assert!(
            owner
                .credit_task_board_kill(fence, "oteryn:creature.rat", death_occurrence(65_000))
                .await
        );
        let after_old_death = full_board(&owner, &mut admitted, 4, 0, "").await?;
        assert!(
            after_old_death
                .weekly
                .kills
                .iter()
                .all(|task| task.current == 0),
            "a death durably ignored before board creation cannot count after creation"
        );
        println!(
            "<TASK_BOARD> POSTGRES_CASE_COMPLETED: inactive_death_without_calendar_replays_noop"
        );
        println!("<TASK_BOARD> POSTGRES_CASE_COMPLETED: weekly_start_and_exact_replay");
        let before_real_kill = experience(&harness).await?;
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
        assert_eq!(
            experience(&harness).await?,
            before_real_kill + RAT_XP_AT_LEVEL_8
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_xp_receipts")
                .fetch_one(&harness.pool)
                .await?,
            1,
            "the real kill's XP receipt is committed"
        );

        let credited = full_board(&owner, &mut admitted, 4, 0, "").await?;
        assert_eq!(
            credited
                .weekly
                .kills
                .iter()
                .find(|task| task.race.is_empty())
                .expect("any kill")
                .current,
            1
        );
        let task = credited
            .weekly
            .kills
            .iter()
            .find(|task| !task.race.is_empty())
            .expect("generated kill")
            .clone();
        let before_completion = experience(&harness).await?;
        for count in task.current..task.required {
            let fence = owner
                .current_quest_fence(session)
                .await
                .map_err(|()| "fence read")?
                .ok_or("current fence")?;
            let occurrence = death_occurrence(u16::try_from(100 + count)?);
            assert!(
                owner
                    .credit_task_board_kill(fence, &task.race, occurrence)
                    .await
            );
            assert!(
                owner
                    .credit_task_board_kill(fence, &task.race, occurrence)
                    .await
            );
        }
        let completed = full_board(&owner, &mut admitted, 4, 0, "").await?;
        assert_eq!(
            completed
                .weekly
                .kills
                .iter()
                .find(|entry| entry.race == task.race)
                .expect("same kill")
                .current,
            task.required
        );
        assert_eq!(
            experience(&harness).await?,
            before_completion + i64::try_from(credited.weekly.kill_experience)?
        );
        assert_eq!(
            completed.points, 0,
            "weekly points settle at the owner period boundary"
        );
        println!("<TASK_BOARD> POSTGRES_CASE_COMPLETED: real_kill_credit_and_completion_xp");

        progression_case::fixture_vocation(
            &owner,
            &admitted,
            crate::durability::character_build::NO_VOCATION,
        )
        .await?;
        assert_eq!(
            full_board(&owner, &mut admitted, 7, 0, "").await?.refusal,
            2,
            "a durable no-vocation build cannot open bounty offers"
        );
        progression_case::fixture_vocation(&owner, &admitted, "monk").await?;
        let opened = full_board(&owner, &mut admitted, 7, 0, "").await?;
        assert_eq!(opened.refusal, 0);
        assert_eq!((opened.bounty.state, opened.bounty.offers.len()), (1, 3));
        let selected = full_board(&owner, &mut admitted, 8, 0, "").await?;
        assert_eq!(selected.refusal, 0);
        let bounty = selected.bounty.active.clone().expect("active bounty");
        assert_eq!(
            full_board(&owner, &mut admitted, 9, 0, "").await?.refusal,
            2
        );
        assert_eq!(
            full_board(&owner, &mut admitted, 8, 1, "").await?.refusal,
            2
        );
        for count in 0..bounty.required {
            let fence = owner
                .current_quest_fence(session)
                .await
                .map_err(|()| "fence read")?
                .ok_or("current fence")?;
            let occurrence = death_occurrence(u16::try_from(500 + count)?);
            assert!(
                owner
                    .credit_task_board_kill(fence, &bounty.race, occurrence)
                    .await
            );
            assert!(
                owner
                    .credit_task_board_kill(fence, &bounty.race, occurrence)
                    .await
            );
        }
        assert_eq!(
            full_board(&owner, &mut admitted, 4, 0, "")
                .await?
                .bounty
                .state,
            3
        );
        let before_claim = experience(&harness).await?;
        let claim_id = admitted.continuity.next_command_id;
        let claimed = full_board(&owner, &mut admitted, 9, 0, "").await?;
        assert_eq!(claimed.refusal, 0);
        assert_eq!(
            (
                claimed.bounty.state,
                claimed.bounty_points,
                claimed.bounty.rerolls
            ),
            (1, u64::from(bounty.points), 1)
        );
        assert_eq!(
            experience(&harness).await?,
            before_claim + i64::try_from(bounty.experience)?
        );
        let replay = serve_from(&owner, &mut admitted, claim_id, &[full_request(9, 0, "")]).await?;
        assert_eq!(replay[0].1, CommandStatus::DuplicateReplay);
        assert_eq!(
            wire::full::decode_reply(&replay[0].2).map_err(debug)?,
            claimed
        );
        assert_eq!(
            full_board(&owner, &mut admitted, 9, 0, "").await?.refusal,
            2
        );
        println!("<TASK_BOARD> POSTGRES_CASE_COMPLETED: bounty_select_complete_claim_once");

        if fault == Some(RollbackFault::TokenLedger) {
            token_case::missing_participant(&owner, &harness, &mut admitted).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            harness.cleanup().await?;
            return Ok(());
        }
        let daily = full_board(&owner, &mut admitted, 12, 0, "").await?;
        assert_eq!((daily.refusal, daily.bounty.rerolls), (0, 2));
        assert_eq!(
            full_board(&owner, &mut admitted, 12, 0, "").await?.refusal,
            5
        );
        let reroll_id = admitted.continuity.next_command_id;
        let rerolled = full_board(&owner, &mut admitted, 11, 0, "").await?;
        assert_eq!((rerolled.refusal, rerolled.bounty.rerolls), (0, 1));
        let replay =
            serve_from(&owner, &mut admitted, reroll_id, &[full_request(11, 0, "")]).await?;
        assert_eq!(replay[0].1, CommandStatus::DuplicateReplay);
        assert_eq!(
            wire::full::decode_reply(&replay[0].2).map_err(debug)?,
            rerolled
        );
        println!("<TASK_BOARD> POSTGRES_CASE_COMPLETED: daily_token_and_paid_reroll_ledger");

        let preferred = full_board(&owner, &mut admitted, 13, 0, &bounty.race).await?;
        assert_eq!(preferred.refusal, 0);
        assert_eq!(preferred.bounty.lists[0].preferred, bounty.race);
        let unwanted = full_board(&owner, &mut admitted, 14, 0, &task.race).await?;
        assert_eq!(unwanted.refusal, 0);
        assert_eq!(unwanted.bounty.lists[0].unwanted, task.race);
        assert_eq!(
            full_board(&owner, &mut admitted, 15, 0, "").await?.refusal,
            4
        );
        assert_eq!(
            full_board(&owner, &mut admitted, 10, 3, "").await?.refusal,
            3
        );
        println!("<TASK_BOARD> POSTGRES_CASE_COMPLETED: preferred_unwanted_and_level_refusal");

        expansion_case(&owner, &harness, &mut admitted).await?;
        let stash_fault = fault.filter(|case| case.is_stash());
        delivery_cases(&owner, &harness, &mut admitted, stash_fault).await?;
        if stash_fault.is_some() {
            drop(owner);
            drop(authority);
            drop(seal);
            harness.cleanup().await?;
            return Ok(());
        }
        let accrued = full_board(&owner, &mut admitted, 4, 0, "").await?;
        assert!(accrued.weekly.pending_points > 0 && accrued.weekly.pending_soulseals > 0);
        let refused_start_id = admitted.continuity.next_command_id;
        let refused_start = full_board(&owner, &mut admitted, 5, 0, "").await?;
        assert_eq!(refused_start.refusal, 2);
        assert_eq!(refused_start.weekly, accrued.weekly);
        assert_eq!(
            (
                refused_start.points,
                refused_start.soulseals,
                refused_start.bounty_points
            ),
            (accrued.points, accrued.soulseals, accrued.bounty_points)
        );
        let refused_replay = serve_from(
            &owner,
            &mut admitted,
            refused_start_id,
            &[full_request(5, 0, "")],
        )
        .await?;
        assert_eq!(refused_replay[0].1, CommandStatus::DuplicateReplay);
        assert_eq!(
            wire::full::decode_reply(&refused_replay[0].2).map_err(debug)?,
            refused_start
        );
        println!(
            "<TASK_BOARD> POSTGRES_CASE_COMPLETED: second_start_preserves_progress_and_accrued_rewards"
        );
        let mint_fault = fault == Some(RollbackFault::TalismanMint);
        item_shop_and_talisman_cases(&owner, &harness, &mut admitted, mint_fault).await?;
        if mint_fault {
            drop(owner);
            drop(authority);
            drop(seal);
            harness.cleanup().await?;
            return Ok(());
        }
        let cosmetic_fault = fault == Some(RollbackFault::CosmeticGrant);
        cosmetic_cases::cosmetics_case(&owner, &harness, &mut admitted, cosmetic_fault).await?;
        if cosmetic_fault {
            drop(owner);
            drop(authority);
            drop(seal);
            harness.cleanup().await?;
            return Ok(());
        }
        let before_reset = full_board(&owner, &mut admitted, 4, 0, "").await?;
        fixture_next_reset(&harness).await?;
        let reset = full_board(&owner, &mut admitted, 4, 0, "").await?;
        assert!(!reset.weekly.started);
        assert_eq!(reset.period, before_reset.period + 7);
        assert_eq!(
            reset.points,
            before_reset.points + before_reset.weekly.pending_points
        );
        assert_eq!(
            reset.soulseals,
            before_reset.soulseals + before_reset.weekly.pending_soulseals
        );
        let settled_points = reset.points;
        let again = full_board(&owner, &mut admitted, 4, 0, "").await?;
        assert_eq!(
            (again.points, again.soulseals),
            (reset.points, reset.soulseals)
        );
        println!("<TASK_BOARD> POSTGRES_CASE_COMPLETED: committed_reset_settles_once");

        // Restore the existing refusal coverage before fixture funding upgrades.
        fixture_track(&harness, task_board::BOUNTY_POINTS, 0).await?;
        // The talisman upgrade is reached through opcode 89's real command stream. With no
        // Bounty Points it refuses atomically and exposes all four durable paths and sourced
        // next costs (rather than manufacturing points from Weekly Task Points).
        let talisman = board(&owner, &mut admitted, 3, 1).await?;
        assert!(talisman.refused);
        assert_eq!(talisman.bounty_points, 0);
        assert_eq!(talisman.talisman_levels, [0; 4]);
        assert_eq!(talisman.talisman_bonuses, [0; 4]);
        assert_eq!(talisman.talisman_next_costs, [5; 4]);
        assert_eq!(talisman.points, settled_points as i64);

        // Fixture funding represents previously earned Bounty Points. All upgrades below
        // still use the production catalogue, writer, fences and real command stream.
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,37,2) ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value")
            .bind(id(41).as_slice()).bind(task_board::BOUNTY_POINTS).bind(task_board::QUEST)
            .execute(&mut *tx).await?;
        tx.commit().await?;
        owner.refresh_quest_session(session, false).await;
        for path in 1..=4 {
            let upgrade_id = admitted.continuity.next_command_id;
            let upgraded = board(&owner, &mut admitted, 3, path).await?;
            assert!(!upgraded.refused);
            assert_eq!(upgraded.bounty_points, 37 - i64::from(path) * 5);
            assert_eq!(upgraded.talisman_levels[usize::from(path - 1)], 1);
            assert_eq!(
                upgraded.talisman_bonuses[usize::from(path - 1)],
                if path == 4 { 600 } else { 300 }
            );
            assert_eq!(upgraded.talisman_next_costs[usize::from(path - 1)], 17);
            let replay = serve_from(&owner, &mut admitted, upgrade_id, &[request(3, path)]).await?;
            assert_eq!(replay[0].1, CommandStatus::DuplicateReplay);
        }
        let upgraded = board(&owner, &mut admitted, 3, 1).await?;
        assert!(!upgraded.refused);
        assert_eq!(upgraded.bounty_points, 0);
        assert_eq!(upgraded.talisman_levels, [2, 1, 1, 1]);
        assert_eq!(upgraded.talisman_next_costs, [29, 17, 17, 17]);
        assert!(board(&owner, &mut admitted, 3, 1).await?.refused);
        owner.refresh_quest_session(session, false).await;
        let reloaded = board(&owner, &mut admitted, 0, 0).await?;
        assert_eq!(reloaded.talisman_levels, upgraded.talisman_levels);
        assert_eq!(reloaded.bounty_points, 0);
        assert_eq!(reloaded.points, settled_points as i64);

        // An already capped path refuses rather than silently succeeding as a query.
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_character_quest_tracks SET value=$1 WHERE character_id=encode($2,'hex')::uuid AND track_key=$3")
            .bind(task_board::TALISMAN_MAX_LEVEL).bind(id(41).as_slice()).bind(task_board::TALISMAN_DAMAGE)
            .execute(&mut *tx).await?;
        tx.commit().await?;
        owner.refresh_quest_session(session, false).await;
        let receipts = quest_receipts(&harness).await?;
        let capped = board(&owner, &mut admitted, 3, 1).await?;
        assert!(capped.refused);
        assert_eq!(capped.talisman_levels[0], task_board::TALISMAN_MAX_LEVEL);
        assert_eq!(capped.talisman_next_costs[0], 0);
        assert_eq!(quest_receipts(&harness).await?, receipts);

        println!("<TASK_BOARD> POSTGRES_CASE_COMPLETED: talisman_upgrade_and_cap");
        currency_case::currency_boundaries(&owner, &harness, &mut admitted).await?;
        token_case::bigint_boundary(&owner, &harness, &mut admitted).await?;
        progression_case::pending_experience_case(&owner, &harness, &admitted).await?;
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await?;
        Ok(())
    })
}

/// A distinct UUIDv7-shaped death occurrence, as the kill settle memoizes one.
fn death_occurrence(tag: u16) -> [u8; 16] {
    let mut bytes = [0xA7; 16];
    bytes[..2].copy_from_slice(&tag.to_be_bytes());
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}
