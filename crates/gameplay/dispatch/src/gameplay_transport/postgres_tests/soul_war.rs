//! W11 Soul War: real command7 NPC dispatch, native XP receipt descendants and reload.
use super::soul_war_fixtures::{command, current, kill};

use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};
use crate::durability::quest_state::soul_war::{
    SoulWarCommand, apply_soul_war_kill_in_transaction, soul_war_experience_in_transaction,
};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::ComposedFreshAdmission;
use oteryn_gameplay_quests::quest::soul_war::{
    SoulWarBoss, SoulWarRefusal, TAINT_DURATION_SECONDS,
};

#[test]
fn soul_war_postgres_native_taints_xp_reset_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("soul-war-durable", scenario)
}

#[test]
fn soul_war_postgres_mixed_and_all_expired_terminal_replay() -> TestResult {
    super::party_xp_fixtures::run(super::party_xp_fixtures::Case::SoulWarTerminal)
}

async fn place_npc(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
) -> TestResult<oteryn_protocol_oteryn::world_spatial_entities::EntityRef> {
    use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
    let catalogue = crate::content::NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        crate::content::npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    owner.states.chat.install_npc_talk(std::sync::Arc::new(
        crate::gameplay_transport::npc_talk::NpcTalkBook::from_model(&model),
    ));
    owner
        .states
        .chat
        .admit(session, "Soul War Hero", owner.owner_now());
    owner.states.chat.set_npc_service(session);
    let mut runtime = owner.runtime.lock().await;
    let spot = runtime
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    runtime
        .place_npcs(
            &NpcPlacementTable {
                npcs: vec![NpcPlacements {
                    key: "oteryn:npc.flickering_soul".into(),
                    placements: vec![NpcPlacement {
                        floor: -i8::try_from(spot.floor)?,
                        x: u16::try_from(spot.x + 1)?,
                        y: u16::try_from(spot.y)?,
                        direction: MovementFacing::South,
                    }],
                }],
            },
            |_| true,
            true,
        )
        .map_err(debug)?;
    let placed = runtime
        .visible_entities()
        .npcs
        .first()
        .copied()
        .ok_or("Flickering Soul placement")?;
    Ok(oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
        identity: placed.actor.placement_identity(),
        generation: placed.generation,
    })
}

async fn talk(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    number: u64,
    text: &str,
) -> TestResult {
    use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
    use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
    let fence = current(owner, session).await?;
    let result = owner
        .npc_talk(
            actor,
            UseCommand {
                game_session_id: session,
                command_id: number,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            },
        )
        .await;
    assert!(
        !result.unknown,
        "command {number} ({text}): NPC outcome must be retained"
    );
    assert_eq!(
        result.disposition,
        NpcIntentDisposition::Ok,
        "command {number} ({text})"
    );
    Ok(())
}

fn scenario() -> TestResult {
    scenario_with_outfit(false)
}

#[test]
fn qengine_l13_revenant_parent_and_accessories() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("qengine-l13-revenant", || {
        scenario_with_outfit(true)
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED qengine-l13-revenant duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

fn scenario_with_outfit(qualify_outfit: bool) -> TestResult {
    run(async |admin| {
        reset();
        use crate::gameplay_transport::connection::FirstEntryOutcome;
        use oteryn_gameplay_quests::quest::world_callbacks::{
            Callback, Position, Selector, Trigger,
        };
        let trigger = Position {
            x: 33620,
            y: 31400,
            floor: -10,
        };
        let world = super::quest_world_callbacks_fixtures::world(
            &Callback {
                track_initials: Default::default(),
                character_requirements: None,
                required_completed_quests: Vec::new(),
                admission: None,
                requires_premium: false,
                state_ops: Vec::new(),
                use_range: 1,
                disguise: None,
                completes: false,
                premium_required: false,
                present: vec![],
                required_travel: vec![],
                state_operations: Vec::new(),
                world_change: None,
                actor_kind: Default::default(),
                opening_only: false,
                requires_world_admission: false,
                key: "oteryn:quest-callback/soul-war-final-reward".into(),
                quest: "oteryn:quest/soul-war".into(),
                trigger: Trigger::Use,
                selector: Selector {
                    target_definition: None,
                    ordinal: None,
                    native: None,
                    position: Some(trigger),
                    unique: None,
                    action: None,
                    appearance: None,
                    position_aliases: Vec::new(),
                },
                lever: None,
                reversible: false,
                passages: vec![],
                teleport: None,
                facing: None,
                effects: Default::default(),
                world_requirements: vec![],
                storage: vec![],
                minimum_level: None,
                experience: None,
                monk_reward: None,
                offerings: vec![],
                item_on_target: None,
                return_on_failure: false,
                failure_message: None,
                success_message: None,
                native_rule: None,
                sources: vec![],
                failure_teleport: None,
                achievements: Vec::new(),
                activation: None,
                item_reward: None,
                remote_teleport: None,
                requirements: None,
                reward_bundle: false,
            },
            trigger,
        )?;
        let h = if qualify_outfit {
            Harness::create_quest_reward(admin, "soul_war_outfit", true).await?
        } else {
            Harness::create_with_admission_seed(admin, "soul_war_durable", false, false).await?
        };
        prepare_current_session(&h).await?;
        let mut deps = Dependencies::new()?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let (controller, activated) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("production generation")?;
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let frame = world.frame_binding_digest(deps.admitted.world_id);
        let room = activated
            .qualified_room()
            .bundle_view(
                cells.clone(),
                frame,
                world.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: 33619,
                    y: 31400,
                    floor: 10,
                },
            )
            .ok_or("Soul War bundle view")?;
        let pin = crate::content::channel_content_pin::ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            frame,
            world.map_revision_digest(),
            (33619, 31400, 10),
        );
        let mut runtime = crate::foundation::ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            h.node.fact().node_id(),
            h.node.fact().registration_revision(),
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                crate::foundation::MovementLocalPosition {
                    x: 33619,
                    y: 31400,
                    floor: 10,
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        deps.room = room.clone();
        let premium = if qualify_outfit {
            Some(super::qengine_endgame_stories_fixtures::current_premium(&h).await?)
        } else {
            None
        };
        let mut owner = deps.owner(&h, &authority);
        owner.premium_coordinator = premium.as_ref().map(|premium| premium as _);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
        owner.quest_catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.runtime_actor = Some(deps.actor);
        let fence = current(&owner, admitted.game_session_id).await?;
        admitted.item_fence = Some(fence.item_fence());
        // The replacement qualified carrier must carry the same native Character
        // lease as the committed fresh session before any NPC or USE preflight.
        owner
            .runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        owner.admit_quest_session(&admitted).await;
        assert!(
            crate::gameplay_transport::hazard::qualified_source_frame(
                &*owner.runtime.lock().await,
                &cells
            )
            .is_some(),
            "source USE must run under a qualified bundle frame"
        );
        let session = deps.admitted.game_session_id;
        let fence = current(&owner, session).await?;
        let initial = h
            .root
            .read_soul_war_state(&authority, &h.node, fence)
            .await
            .map_err(debug)?;
        assert!(!initial.access());
        assert!(!initial.chain_ready());
        let refused = command(&owner, session, 501, SoulWarCommand::ReportCompletion).await?;
        assert_eq!(refused.result, Err(SoulWarRefusal::CompletionRequired));
        assert_eq!(
            refused.character_revision,
            fence.expected_character_revision
        );
        kill(
            &owner,
            &h.pool,
            session,
            199,
            "oteryn:creature.goshnar_s_malice",
            true,
        )
        .await?;
        assert!(
            !h.root
                .read_soul_war_state(&authority, &h.node, current(&owner, session).await?)
                .await
                .map_err(debug)?
                .access()
        );
        let npc = place_npc(&owner, deps.actor, session).await?;
        for (number, text) in [(510, "hi"), (511, "task"), (512, "yes")] {
            talk(&owner, deps.actor, session, npc, number, text).await?;
        }
        let started = h
            .root
            .reconcile_soul_war_command(
                &authority,
                fence,
                CommandRef::new(session, CommandId::new(512).map_err(debug)?),
                SoulWarCommand::Start,
            )
            .await
            .map_err(debug)?
            .ok_or("real NPC task receipt")?;
        assert!(started.state.access());
        let replay = h
            .root
            .reconcile_soul_war_command(
                &authority,
                fence,
                CommandRef::new(session, CommandId::new(512).map_err(debug)?),
                SoulWarCommand::Start,
            )
            .await
            .map_err(debug)?
            .ok_or("start receipt")?;
        assert!(replay.replayed);
        assert_eq!(replay.state, started.state);
        assert_eq!(replay.character_revision, started.character_revision);
        let conflict = h
            .root
            .reconcile_soul_war_command(
                &authority,
                fence,
                CommandRef::new(session, CommandId::new(512).map_err(debug)?),
                SoulWarCommand::Cleanse,
            )
            .await;
        assert!(matches!(
            conflict,
            Err(CharacterProgressionError::ConflictingOccurrence)
        ));
        let missing = h
            .root
            .reconcile_soul_war_command(
                &authority,
                fence,
                CommandRef::new(session, CommandId::new(599).map_err(debug)?),
                SoulWarCommand::Start,
            )
            .await
            .map_err(debug)?;
        assert!(
            missing.is_none(),
            "reconciliation must never execute a missing command"
        );
        let clean_fence = current(&owner, session).await?;
        let wrong_generation = CurrentCharacterGameplayFence {
            connection_generation: crate::foundation::ConnectionGeneration::new(
                clean_fence.connection_generation.get() + 1,
            )
            .map_err(debug)?,
            ..clean_fence
        };
        assert!(
            h.root
                .reconcile_soul_war_command(
                    &authority,
                    wrong_generation,
                    CommandRef::new(session, CommandId::new(512).map_err(debug)?),
                    SoulWarCommand::Start
                )
                .await
                .map_err(debug)?
                .is_none()
        );
        for stale in [
            wrong_generation,
            CurrentCharacterGameplayFence {
                scope_ownership_generation: crate::foundation::ScopeOwnershipGeneration::new(
                    clean_fence.scope_ownership_generation.get() + 1,
                )
                .map_err(debug)?,
                ..clean_fence
            },
            CurrentCharacterGameplayFence {
                character_lease_generation: clean_fence.character_lease_generation + 1,
                ..clean_fence
            },
            CurrentCharacterGameplayFence {
                expected_character_revision: fence.expected_character_revision,
                ..clean_fence
            },
        ] {
            assert!(
                h.root
                    .commit_soul_war_command(
                        &authority,
                        &h.node,
                        stale,
                        CommandRef::new(session, CommandId::new(503).map_err(debug)?),
                        SoulWarCommand::Cleanse
                    )
                    .await
                    .is_err()
            );
        }
        // A shared-XP bystander has a native member receipt but is not a taint recipient.
        let bystander_death = kill(
            &owner,
            &h.pool,
            session,
            201,
            "oteryn:creature.goshnar_s_malice",
            false,
        )
        .await?;
        let mut historic = h.pool.begin().await?;
        let wrong_grant = apply_soul_war_kill_in_transaction(
            &mut historic,
            &h.node,
            &bystander_death,
            "oteryn:creature.goshnar_s_malice",
            &[(current(&owner, session).await?, true)],
        )
        .await;
        assert!(
            matches!(
                wrong_grant,
                Err(CharacterProgressionError::AuthorityRejected)
            ),
            "a retained historic XP receipt cannot create a new taint"
        );
        historic.rollback().await?;
        assert_eq!(
            h.root
                .read_soul_war_state(&authority, &h.node, current(&owner, session).await?)
                .await
                .map_err(debug)?
                .defeated_bosses,
            0
        );
        let bosses = [
            (SoulWarBoss::Malice, "malice"),
            (SoulWarBoss::Hatred, "hatred"),
            (SoulWarBoss::Spite, "spite"),
            (SoulWarBoss::Cruelty, "cruelty"),
            (SoulWarBoss::Greed, "greed"),
        ];
        let mut first_clock = None;
        for (index, (boss, name)) in bosses.into_iter().enumerate() {
            kill(
                &owner,
                &h.pool,
                session,
                202 + index as u8,
                &format!("oteryn:creature.goshnar_s_{name}"),
                true,
            )
            .await?;
            let state = h
                .root
                .read_soul_war_state(&authority, &h.node, current(&owner, session).await?)
                .await
                .map_err(debug)?;
            first_clock = first_clock.or(state.first_taint_at);
            assert_eq!(state.first_taint_at, first_clock);
            assert_ne!(state.defeated_bosses & boss.bit(), 0);
            let mut tx = h.pool.begin().await?;
            assert_eq!(
                soul_war_experience_in_transaction(
                    &mut tx,
                    fence.character_id,
                    "oteryn:creature.bony_sea_devil",
                    1_000_000
                )
                .await
                .map_err(debug)?,
                oteryn_gameplay_quests::quest::soul_war::taint_experience(
                    1_000_000,
                    index as u8 + 1
                )
                .map_err(debug)?
            );
            assert_eq!(
                soul_war_experience_in_transaction(
                    &mut tx,
                    fence.character_id,
                    "oteryn:creature.rat",
                    1_000_000
                )
                .await
                .map_err(debug)?,
                1_000_000
            );
            tx.rollback().await?;
        }
        assert!(
            h.root
                .read_soul_war_state(&authority, &h.node, current(&owner, session).await?)
                .await
                .map_err(debug)?
                .chain_ready()
        );
        let full = h
            .root
            .read_soul_war_state(&authority, &h.node, current(&owner, session).await?)
            .await
            .map_err(debug)?;
        {
            use crate::durability::endgame_terminal::{
                EndgameFailureRequest, FinalEncounter, TestOwnerState, TestSharedTerminalOwner,
            };
            let request = EndgameFailureRequest {
                attempt: [73; 32],
                encounter: FinalEncounter::Megalomania,
                participants: vec![current(&owner, session).await?],
            };
            let terminal =
                TestSharedTerminalOwner::new(request.clone(), TestOwnerState::TerminalUnsuccessful);
            let mut terminal_slot = owner.revision_sequencer.acquire(fence.character_id).await;
            let failed = h
                .root
                .apply_endgame_failure(&authority, &h.node, &terminal, request.clone())
                .await
                .map_err(debug)?;
            terminal_slot.observe_endgame_progression();
            drop(terminal_slot);
            let pair = failed.removed_flags.ok_or("retained failure pair")?;
            assert_eq!(pair.count_ones(), 2);
            assert!(!failed.replayed);
            let reduced = h
                .root
                .read_soul_war_state(&authority, &h.node, current(&owner, session).await?)
                .await
                .map_err(debug)?;
            assert_eq!(reduced.defeated_bosses, 31 & !pair);
            assert_eq!(reduced.first_taint_at, full.first_taint_at);
            assert_eq!(reduced.defeated_bosses.count_ones(), 3);
            let mut terminal_slot = owner.revision_sequencer.acquire(fence.character_id).await;
            let replay = h
                .root
                .apply_endgame_failure(&authority, &h.node, &terminal, request)
                .await
                .map_err(debug)?;
            terminal_slot.observe_endgame_progression();
            drop(terminal_slot);
            assert!(replay.replayed);
            assert_eq!(replay.removed_flags, Some(pair));
            // A failed attempt reopens only the two selected minibosses.
            for (index, (boss, name)) in [
                (SoulWarBoss::Malice, "malice"),
                (SoulWarBoss::Hatred, "hatred"),
                (SoulWarBoss::Spite, "spite"),
                (SoulWarBoss::Cruelty, "cruelty"),
                (SoulWarBoss::Greed, "greed"),
            ]
            .into_iter()
            .enumerate()
            {
                if pair & boss.bit() != 0 {
                    kill(
                        &owner,
                        &h.pool,
                        session,
                        220 + index as u8,
                        &format!("oteryn:creature.goshnar_s_{name}"),
                        true,
                    )
                    .await?;
                }
            }
        }
        let first = h
            .root
            .read_soul_war_state(&authority, &h.node, current(&owner, session).await?)
            .await
            .map_err(debug)?;
        assert_eq!(first, full);
        kill(
            &owner,
            &h.pool,
            session,
            210,
            "oteryn:creature.goshnar_s_malice",
            true,
        )
        .await?;
        assert_eq!(
            h.root
                .read_soul_war_state(&authority, &h.node, current(&owner, session).await?)
                .await
                .map_err(debug)?,
            first
        );
        kill(
            &owner,
            &h.pool,
            session,
            211,
            "oteryn:creature.goshnar_s_megalomania_blue",
            true,
        )
        .await?;
        let final_state = h
            .root
            .read_soul_war_state(&authority, &h.node, current(&owner, session).await?)
            .await
            .map_err(debug)?;
        assert!(final_state.completed());
        assert!(!final_state.chain_ready());
        assert_eq!(final_state.first_taint_at, None);
        let pending = h
            .root
            .pending_soul_war_reward(&authority, fence.character_id)
            .await
            .map_err(debug)?
            .ok_or("earned final item")?;
        assert!((34082..=34099).contains(&pending.item_source_id));
        assert!(
            !final_state.reward_claimed(),
            "earned entitlement is not an item delivery"
        );
        let outfit = "oteryn:appearance-grant/quest/soul-war/revenant-base";
        let before_report: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlocks u JOIN game_character_roots c USING(account_id) WHERE c.character_id=encode($1,'hex')::uuid AND u.unlock_key=$2")
            .bind(fence.character_id.as_bytes().as_slice()).bind(outfit).fetch_one(&h.pool).await?;
        assert_eq!(
            before_report, 0,
            "the native final earns the item; Revenant waits for the task report"
        );
        talk(&owner, deps.actor, session, npc, 504, "task").await?;
        let completion = h
            .root
            .reconcile_soul_war_command(
                &authority,
                current(&owner, session).await?,
                CommandRef::new(session, CommandId::new(504).map_err(debug)?),
                SoulWarCommand::ReportCompletion,
            )
            .await
            .map_err(debug)?
            .ok_or("NPC completion receipt")?;
        assert!(completion.result.is_ok());
        let again = command(&owner, session, 505, SoulWarCommand::ReportCompletion).await?;
        assert!(again.result.is_ok());
        assert_eq!(again.character_revision, completion.character_revision);
        let outfit_rows: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_account_appearance_unlocks u JOIN game_character_roots c USING(account_id) WHERE c.character_id=encode($1,'hex')::uuid AND u.unlock_key=$2),(SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE granting_character_id=encode($1,'hex')::uuid AND unlock_key=$2 AND source_kind='quest' AND content_revision='crystal-201189f-soul-war-revenant-v1')")
            .bind(fence.character_id.as_bytes().as_slice()).bind(outfit).fetch_one(&h.pool).await?;
        assert_eq!(
            outfit_rows,
            (1, 1),
            "repeated task report retains one source-qualified account entitlement and receipt"
        );
        assert_eq!(
            h.root
                .pending_soul_war_reward(&authority, fence.character_id)
                .await
                .map_err(debug)?,
            Some(pending.clone())
        );
        reward_delivery(
            &mut owner,
            &h,
            &mut admitted,
            &world,
            pending.item_source_id,
        )
        .await?;
        if qualify_outfit {
            admitted.runtime_actor = Some(deps.actor);
            super::qengine_endgame_stories_fixtures::accessories(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                "revenant",
            )
            .await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }

        kill(
            &owner,
            &h.pool,
            session,
            212,
            "oteryn:creature.goshnar_s_greed",
            true,
        )
        .await?;
        talk(&owner, deps.actor, session, npc, 506, "taints").await?;
        let cleansed = h
            .root
            .reconcile_soul_war_command(
                &authority,
                current(&owner, session).await?,
                CommandRef::new(session, CommandId::new(506).map_err(debug)?),
                SoulWarCommand::Cleanse,
            )
            .await
            .map_err(debug)?
            .ok_or("NPC cleanse receipt")?;
        assert_eq!(cleansed.state.defeated_bosses, 0);
        assert!(cleansed.state.completed());
        // Source lifetime test: historical persisted state with a DB-clock anchor.
        kill(
            &owner,
            &h.pool,
            session,
            213,
            "oteryn:creature.goshnar_s_spite",
            true,
        )
        .await?;
        let mut historical = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *historical)
            .await?;
        sqlx::query("UPDATE game_character_soul_war SET first_taint_at=floor(extract(epoch FROM transaction_timestamp()))::bigint-$2 WHERE character_id=encode($1,'hex')::uuid")
            .bind(fence.character_id.as_bytes().as_slice()).bind(TAINT_DURATION_SECONDS).execute(&mut *historical).await?;
        historical.commit().await?;
        let expired = h
            .root
            .read_soul_war_state(&authority, &h.node, current(&owner, session).await?)
            .await
            .map_err(debug)?;
        assert_eq!(expired.defeated_bosses, 0);
        assert_eq!(expired.first_taint_at, None);
        assert!(expired.completed());
        let before_reload = current(&owner, session).await?.expected_character_revision;
        assert_eq!(
            h.root
                .read_soul_war_state(&authority, &h.node, current(&owner, session).await?)
                .await
                .map_err(debug)?,
            expired
        );
        assert_eq!(
            current(&owner, session).await?.expected_character_revision,
            before_reload
        );
        let denied = sqlx::query("UPDATE game_character_soul_war SET defeated_bosses=0,first_taint_at=NULL WHERE character_id=encode($1,'hex')::uuid")
            .bind(fence.character_id.as_bytes().as_slice()).execute(&h.pool).await;
        assert!(denied.is_err(), "unguarded taint mutation must roll back");
        eprintln!("SOUL_WAR POSTGRES_CASE_COMPLETED npc_commands_native_xp_taints_reset_reload");
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn reward_delivery(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    world: &crate::map::boot::BundleWorld,
    item_source: u32,
) -> TestResult {
    use crate::durability::item_mint::TypedDefinitionRef;
    use crate::durability::quest_state::soul_war::{
        SOUL_WAR_REWARD_CLAIM, SOUL_WAR_REWARD_PLACEMENT, SOUL_WAR_REWARD_REVISION,
    };
    use crate::durability::reward_claim_mint::{
        RewardClaimMintError, RewardClaimMintRequest, RewardClaimRefusal,
    };
    use crate::gameplay_transport::capabilities::SelectedCapabilities;
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;
    use crate::gameplay_transport::item_ref_admission::{
        NativeItemWeights, generation_item_facts, pinned_index,
    };
    use oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_VIEW_MOVE_V1;
    use oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1;
    use oteryn_protocol_oteryn::world_object::{
        CAPABILITY_ITEM_USE_V1, CAPABILITY_WORLD_INTERACTION_V1, UseDisposition, WorldObjectTarget,
    };
    use oteryn_protocol_oteryn::world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES;
    // NoRoom is a negotiated world-interaction disposition. A fresh client
    // without capability 11 intentionally receives the legacy Rejected code.
    // Selection drops capabilities whose dependencies are absent: 11 requires
    // 15 + 18, 15 requires 4, 18 requires 4 + 6, and 4 requires 6.
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        owner.offered_capabilities(),
        &[
            CAPABILITY_ITEM_VIEW_MOVE_V1,
            CAPABILITY_WORLD_SPATIAL_ENTITIES,
            CAPABILITY_WORLD_INTERACTION_V1,
            CAPABILITY_ITEM_USE_V1,
            CAPABILITY_WORLD_MAP_VIEW_V1,
        ],
    )
    .ok_or("source reward world-interaction capability not offered")?;
    assert!(
        admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_WORLD_INTERACTION_V1),
        "source USE requires the complete world-interaction capability dependency set"
    );
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native reward facts")?;
    let session = admitted.game_session_id;
    let fence = current(owner, session).await?;
    let item_key = format!("oteryn:item.tibia.i{item_source}");
    let revision = {
        let runtime = owner.runtime.lock().await;
        pinned_index(&runtime, owner.active_generation)
            .ok_or("item index")?
            .revision(&item_key)
            .ok_or("soul item revision")?
            .to_owned()
    };
    let item = generation_item_facts(
        native,
        &TypedDefinitionRef {
            family: "Item".into(),
            production_key: item_key.clone(),
            revision_ref: revision,
        },
    )
    .map_err(debug)?;
    let bag_definition = TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i2854".into(),
        revision_ref: "definition-r1".into(),
    };
    let backpack = generation_item_facts(native, &bag_definition).map_err(debug)?;
    let mut request = RewardClaimMintRequest {
        backpack_outputs: false,
        additional_items: Vec::new(),
        command: CommandRef::new(session, CommandId::new(601).map_err(debug)?),
        claim: TypedDefinitionRef {
            family: "RewardClaim".into(),
            production_key: SOUL_WAR_REWARD_CLAIM.into(),
            revision_ref: SOUL_WAR_REWARD_REVISION.into(),
        },
        source_placement: SOUL_WAR_REWARD_PLACEMENT.into(),
        item,
        quantity: 1,
        backpack,
        contents: Vec::new(),
        root_text: None,
        root_fluid_subtype: None,
        root_charges_remaining: None,
        content_revision: format!(
            "sha256:{}",
            native
                .source_digest()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ),
        ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION.into(),
        sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
        achievement: None,
        quest_transition: None,
        key_number: None,
    };
    assert!(
        h.root
            .reconcile_soul_war_reward(owner.character, fence.character_id, &request)
            .await
            .map_err(debug)?
            .is_none()
    );
    let mut stale = fence;
    stale.character_lease_generation += 1;
    assert!(matches!(
        h.root
            .claim_pending_soul_war_reward(
                owner.character,
                owner.holder,
                stale,
                request.clone(),
                &NativeItemWeights(native)
            )
            .await,
        Err(RewardClaimMintError::AuthorityRejected)
    ));
    let no_room = h
        .root
        .claim_pending_soul_war_reward(
            owner.character,
            owner.holder,
            fence,
            request.clone(),
            &NativeItemWeights(native),
        )
        .await;
    assert!(
        matches!(
            no_room,
            Err(RewardClaimMintError::Refused(
                RewardClaimRefusal::NoMainBackpack
            ))
        ),
        "missing bag: {no_room:?}"
    );
    assert!(
        h.root
            .reconcile_soul_war_reward(owner.character, fence.character_id, &request)
            .await
            .map_err(debug)?
            .is_none()
    );
    assert_eq!(
        h.root
            .pending_soul_war_reward(owner.character, fence.character_id)
            .await
            .map_err(debug)?
            .ok_or("retained after no room")?
            .item_source_id,
        item_source
    );
    let key = oteryn_world_bundle::bundle::placement_key(-10, 33620, 31400, 1)
        .ok_or("source chest key")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: 0,
    };
    {
        let current_fence = current(owner, session).await?;
        assert_eq!(
            admitted.item_fence,
            Some(current_fence.item_fence()),
            "source USE retained item fence"
        );
        let map = owner
            .bundle_map()
            .ok_or("source USE bundle map is unavailable")?;
        let generation = owner
            .active_generation
            .ok_or("source USE active generation")?;
        let source = crate::gameplay_transport::world_map::MapViewSource {
            overlay: &map.overlay,
            facts: &map.facts,
            content_generation: generation.identity().client_artifact_digest(),
            reset_epoch: owner.reset_epoch,
        };
        let resolved = crate::gameplay_transport::world_map::resolve_map_target(
            &source,
            &target,
            |_| None::<()>,
        )
        .map_err(|error| format!("source USE target resolution: {error:?}"))?;
        let position = crate::map::view::placement_of(resolved.placement_key)
            .ok_or("source USE placement")?
            .0;
        assert_eq!(
            position,
            crate::map::overlay::TilePos {
                x: 33620,
                y: 31400,
                floor: -10
            },
            "source USE position"
        );
        let runtime = owner.runtime.lock().await;
        let actor = admitted.runtime_actor.ok_or("source USE actor")?;
        assert!(
            crate::gameplay_transport::hazard::qualified_source_frame(
                &runtime,
                owner.movement_cells()
            )
            .is_some(),
            "source USE frame"
        );
        assert!(
            runtime
                .player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none()),
            "source USE current control"
        );
        assert!(
            runtime
                .bound_attacker_lease(actor, session)
                .is_ok_and(|lease| lease.character_id().as_bytes()
                    == current_fence.character_id.as_bytes()
                    && lease.generation() == current_fence.character_lease_generation),
            "source USE bound Character lease"
        );
        let at = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        let native_at = crate::map::overlay::TilePos {
            x: u16::try_from(at.x)?,
            y: u16::try_from(at.y)?,
            floor: -i8::try_from(at.floor)?,
        };
        assert!(
            crate::gameplay_transport::map_door::reachable(native_at, resolved.pos),
            "source USE reach"
        );
        let states = owner.states.spell_states.lock().await;
        assert!(!states.is_dead(actor), "source USE alive");
        assert!(
            states.spell_idle(&runtime, actor, session),
            "source USE idle"
        );
        assert!(
            owner.states.player_trades.physical(session, None),
            "source USE physical interaction"
        );
    }
    let payload =
        oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?;
    let result = super::quest_world_callbacks_fixtures::wire(owner, admitted, 1, 2, payload.clone()).await?;
    assert!(
        admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_WORLD_INTERACTION_V1),
        "source USE must retain world-interaction capability through serve_admitted"
    );
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(debug)?,
        UseDisposition::NoRoom
    );
    let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
    h.root
        .initialize_admission_equipment(
            owner.character,
            owner.holder,
            &fence.item_fence(),
            owner
                .active_generation
                .ok_or("active")?
                .identity()
                .server_artifact_digest(),
            fresh.operation(),
        )
        .await
        .map_err(debug)?;
    h.root
        .grant_starter_items(
            owner.character,
            owner.holder,
            &fence.item_fence(),
            &NativeItemWeights(native),
            &crate::durability::starter_grant::StarterTemplate {
                definition: bag_definition,
                ..crate::durability::starter_grant::StarterTemplate::main()
            },
        )
        .await
        .map_err(debug)?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let result = super::quest_world_callbacks_fixtures::wire(owner, admitted, 2, 2, payload.clone()).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(debug)?,
        UseDisposition::Committed
    );
    let state = h
        .root
        .read_soul_war_state(
            owner.character,
            owner.holder,
            current(owner, session).await?,
        )
        .await
        .map_err(debug)?;
    assert!(state.completed());
    assert!(state.reward_claimed());
    assert!(
        h.root
            .pending_soul_war_reward(owner.character, fence.character_id)
            .await
            .map_err(debug)?
            .is_none()
    );
    let rows: (i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_reward_claim_mint_receipts WHERE character_id=encode($1,'hex')::uuid AND claim_production_key=$2),(SELECT count(*) FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.definition_production_key=$3),(SELECT count(*) FROM game_character_soul_war_reward_entitlements WHERE character_id=encode($1,'hex')::uuid AND state='DELIVERED')")
        .bind(fence.character_id.as_bytes().as_slice()).bind(SOUL_WAR_REWARD_CLAIM).bind(&item_key).fetch_one(&h.pool).await?;
    assert_eq!(rows, (1, 1, 1));
    let replay = super::quest_world_callbacks_fixtures::wire(owner, admitted, 2, 2, payload.clone()).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&replay).map_err(debug)?,
        UseDisposition::Committed
    );
    request.command = CommandRef::new(session, CommandId::new(2).map_err(debug)?);
    let receipt = h
        .root
        .reconcile_soul_war_reward(owner.character, fence.character_id, &request)
        .await
        .map_err(debug)?
        .ok_or("MINT receipt-only recovery")?;
    assert_eq!(receipt.quantity, 1);
    let mut conflicting = request.clone();
    conflicting.quantity = 2;
    assert!(matches!(
        h.root
            .reconcile_soul_war_reward(owner.character, fence.character_id, &conflicting)
            .await,
        Err(RewardClaimMintError::InvalidInput)
    ));
    let mut conflicting = request.clone();
    conflicting.content_revision.push_str("changed");
    assert!(matches!(
        h.root
            .reconcile_soul_war_reward(owner.character, fence.character_id, &conflicting)
            .await,
        Err(RewardClaimMintError::ConflictingCause)
    ));
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let second = super::quest_world_callbacks_fixtures::wire(owner, admitted, 3, 2, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&second).map_err(debug)?,
        UseDisposition::Rejected
    );
    assert_eq!(
        h.root
            .reconcile_soul_war_reward(owner.character, fence.character_id, &request)
            .await
            .map_err(debug)?,
        Some(receipt)
    );
    eprintln!("SOUL_WAR_REWARD POSTGRES_CASE_COMPLETED source_use_no_room_mint_replay_fences");
    Ok(())
}
