//! Canonical PostgreSQL 17.6: genuine fresh sessions, real swings, queue and kill drain.
#![allow(clippy::expect_used)]
use crate::bestiary_postgres_harness::{
    self as fixture, CHANNEL, Harness, TestResult, WORLD, debug, id,
};
use crate::foundation::{
    ChannelId, ChannelRuntimeV1, CommandId, CommandRef, MovementLocalPosition, PartyAction, WorldId,
};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, FirstEntryOutcome, SessionContinuity},
    kill_reward::DrainOutcome,
};
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use std::time::Duration;
use tokio::sync::Mutex;

#[derive(Clone, Copy)]
enum Case {
    Shared,
    Modifiers,
    Inactive,
    Replay,
    Pz,
    Rollback,
    Concurrent,
    Exhausted,
    Foreign,
    Stale,
    OneHit,
    PzContention,
}
#[test]
fn party_xp_real_dispatch_equal_split() -> TestResult {
    run(Case::Shared)
}
#[test]
fn party_xp_real_dispatch_prey_then_stamina() -> TestResult {
    run(Case::Modifiers)
}
#[test]
fn party_xp_real_dispatch_inactive_member_disables_sharing() -> TestResult {
    run(Case::Inactive)
}
#[test]
fn party_xp_concurrent_replay_and_recovered_sequencers_do_not_award_twice() -> TestResult {
    run(Case::Replay)
}

#[test]
fn party_xp_real_dispatch_leader_pz_disables_sharing() -> TestResult {
    run(Case::Pz)
}
#[test]
fn party_xp_real_dispatch_last_member_failure_rolls_back_all_awards() -> TestResult {
    run(Case::Rollback)
}

#[test]
fn party_xp_real_dispatch_concurrent_kills_serialize_character_awards() -> TestResult {
    run(Case::Concurrent)
}
#[test]
fn party_xp_real_dispatch_exhausted_share_is_not_redistributed() -> TestResult {
    run(Case::Exhausted)
}

#[test]
fn party_xp_real_dispatch_preserves_foreign_damage_share() -> TestResult {
    run(Case::Foreign)
}

#[test]
fn party_xp_real_dispatch_stale_member_lease_cannot_award() -> TestResult {
    run(Case::Stale)
}

#[test]
fn party_xp_real_dispatch_lethal_first_hit_qualifies_activity() -> TestResult {
    run(Case::OneHit)
}

#[test]
fn party_xp_real_dispatch_contended_absent_pz_row_defers_without_awards() -> TestResult {
    run(Case::PzContention)
}

fn run(case: Case) -> TestResult {
    // Without the PostgreSQL route the case skips, as the other dispatch PostgreSQL tests do.
    if fixture::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .name("party-xp-pg".into())
        .stack_size(64 << 20)
        .spawn(move || {
            fixture::runtime()
                .map_err(|e| e.to_string())?
                .block_on(async move {
                    let admin = fixture::configured_admin()
                        .ok_or("canonical PostgreSQL admin URL required")?;
                    // One tag per case: the tag names the database and the retained
                    // recovery directory, which concurrent cases must not share.
                    let tag = format!("party_xp_{}", case as u8);
                    let h = Harness::create(admin, &tag, false).await?;
                    eprintln!("PARTY-XP-1 POSTGRES_DATABASE_STARTED PostgreSQL=170006");
                    let result = scenario(&h, case).await;
                    h.cleanup().await?;
                    result
                })
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "party XP PG thread panicked")?
        .map_err(Into::into)
}
async fn scenario(h: &Harness, case: Case) -> TestResult {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let first =
        crate::world_party_runtime_postgres_cases::committed_fresh_player(h, 60, 62, 70, now)
            .await?;
    let second =
        crate::world_party_runtime_postgres_cases::committed_fresh_player(h, 61, 63, 71, now)
            .await?;
    let third = if matches!(case, Case::Foreign) {
        Some(
            crate::world_party_runtime_postgres_cases::committed_fresh_player(h, 62, 64, 72, now)
                .await?,
        )
    } else {
        None
    };
    let seal = h.recovery.seal_current().map_err(debug)?;
    let recovery = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
    let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(
            world,
            Some(include_bytes!(
                "../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
            )),
        );
    let active = controller.active().expect("active");
    let native = active.native_gameplay().expect("native");
    let room = pin.qualified_room().clone();
    let position = MovementLocalPosition {
        x: 2,
        y: 11,
        floor: -7,
    };
    let pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        room.map_revision_digest(),
        (position.x, position.y, position.floor),
    );
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,NULL,$4,$5,$6)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(pin.activation_sequence().to_string())
        .bind(pin.server_artifact_digest().as_slice()).bind(pin.client_artifact_digest().as_slice()).bind(pin.frame_binding_digest().as_slice())
        .execute(&h.pool).await?;
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        channel,
        h.node.fact().node_id(),
        h.node.fact().registration_revision(),
        1,
        1,
        "runtime-scope-assignment:1",
        5,
        pin.clone(),
    )
    .map_err(debug)?;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    // Explicit qualification placement through the normal activation owner. Policies and
    // health come from the active artifact; distinct sources permit two pending deaths.
    let mut sources = Vec::new();
    for (key, y) in [
        ("oteryn:creature.rat", 11),
        ("oteryn:creature.cave_rat", 12),
    ] {
        if y == 12 && !matches!(case, Case::Concurrent | Case::OneHit) {
            continue;
        }
        let policy = runtime.companion_policy(key).map_err(debug)?;
        sources.push((
            vec![(1, y, -7)],
            60_000,
            5_000,
            key.to_owned(),
            u64::try_from(policy.maximum_health)?,
        ));
    }
    runtime.realize_activation_spawns(sources).map_err(debug)?;
    let mut players = Vec::new();
    let mut currents = vec![&first, &second];
    if let Some(third) = &third {
        currents.push(third);
    }
    for current in currents {
        let session = current.current_game_session_id();
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, session, current.current_character_lease())
            .map_err(debug)?;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(current.current_character_lease().character_id().as_bytes().as_slice()).execute(&h.pool).await?;
        players.push((actor, session));
    }
    // PARTY-1 remains the sole writer: invite, accept and leader toggle use its real receipts.
    let lane = crate::durability::spell_owner_commit::SpellLane::new(world, channel);
    let permit = lane.acquire().await.map_err(|_| "lane")?;
    let mut party = None;
    for (index, number, action) in [
        (
            0,
            1,
            PartyAction::Invite {
                target: *second.current_character_lease().character_id().as_bytes(),
            },
        ),
        (1, 1, PartyAction::Leave),
        (0, 2, PartyAction::SetSharedExperience { enabled: true }),
    ] {
        let current = if index == 0 { &first } else { &second };
        let item_fence = crate::durability::item_transfer::CurrentCharacterItemFence {
            character_id: crate::domain::CharacterId::from_bytes(
                *current.current_character_lease().character_id().as_bytes(),
            )
            .map_err(debug)?,
            game_session_id: current.current_game_session_id(),
            connection_generation: current.current_connection_generation(),
            character_lease_generation: current.current_character_lease().generation(),
            runtime_scope: current.current_runtime_scope(),
            scope_ownership_generation: current.current_scope_generation(),
        };
        let mut tx = h.pool.begin().await?;
        let authority =
            crate::durability::spell_item_transaction::assert_spell_item_authority_in_transaction(
                &mut tx,
                &permit,
                &h.root,
                &recovery,
                &h.node,
                &item_fence,
                CommandRef::new(
                    current.current_game_session_id(),
                    CommandId::new(number).map_err(debug)?,
                ),
                [1; 32],
            )
            .await
            .map_err(debug)?;
        let action = if index == 1 {
            PartyAction::Accept {
                party: party.ok_or("party")?,
            }
        } else {
            action
        };
        let visible = if matches!(action, PartyAction::Invite { .. }) {
            crate::durability::admission_journal::party_target_binding::prove_visible_party_target_in_transaction(&mut tx,&h.root,&authority,&runtime,
                players[0].0,players[1].0,players[1].1).await.map_err(debug)?
        } else {
            None
        };
        let receipt = crate::durability::world_party::apply_world_party_command_in_transaction(
            &mut tx,
            &authority,
            action,
            visible.as_ref(),
        )
        .await
        .map_err(debug)?;
        assert_eq!(receipt.disposition, "APPLIED");
        party = receipt.party_id;
        tx.commit().await?;
    }
    drop(permit);
    let creature = runtime
        .visible_entities()
        .creatures
        .first()
        .copied()
        .expect("rat");
    let rat = oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
        identity: creature.actor.placement_identity(),
        generation: creature.generation,
    };
    let door_content = crate::world_runtime::ReferenceContentGeneration::from_content(room.door())
        .map_err(debug)?;
    let door_fence = crate::world_runtime::ScopeContentGenerationFence::for_test(
        first.current_runtime_scope(),
        first.current_scope_generation(),
        door_content,
    );
    let door = Mutex::new(
        crate::world_runtime::bind_native_entry_door(
            room.door(),
            &door_fence,
            first.current_runtime_scope(),
            first.current_scope_generation(),
        )
        .map_err(debug)?,
    );
    let descriptor=crate::native_admission_source::descriptor::ProducerDescriptor::new("platform-test".into(),("127.0.0.1".into(),1),"source.test".into(),"source.test".into(),
        vec![rustls::pki_types::CertificateDer::from(include_bytes!("../../../../../apps/game-server/tests/support/spell-save-tls-fixture/ca.der").to_vec())],
        vec![rustls::pki_types::CertificateDer::from(include_bytes!("../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client.der").to_vec())],
        rustls::pki_types::PrivateKeyDer::try_from(include_bytes!("../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client-key.der").to_vec())?).map_err(debug)?;
    let evidence = crate::gameplay_transport::fresh_evidence::FreshEvidenceSource::new(descriptor);
    let spells = native.spell_book();
    let achievements =
        crate::achievement_catalogue::AchievementCatalogue::embedded().map_err(debug)?;
    let charms =
        crate::content::charm_source::CanonicalCharmCatalogue::embedded().map_err(debug)?;
    let runtime = Mutex::new(runtime);
    let mut owner = crate::gameplay_transport::composed_test_owner::ComposedTestOwner {
        root: &h.root,
        character: &recovery,
        holder: &h.node,
        evidence: &evidence,
        world_id: world,
        channel_id: channel,
        required_capabilities:
            crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
                &[],
                first.current_runtime_scope(),
                1,
                "policy-1",
                crate::gameplay_transport::world_required_capabilities::offered_for_scope(false, None),
            )
            .map_err(debug)?,
        runtime: &runtime,
        door: &door,
        spells,
        achievements: &achievements,
        imported_charms: &charms,
        room: &room,
    }
    .build();
    owner.active_generation = Some(active);
    owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
    owner.clock_origin = std::time::Instant::now()
        .checked_sub(Duration::from_secs(60))
        .expect("clock");
    for (index, (actor, session)) in players.iter().copied().enumerate() {
        let (observed, _) =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(session)
                .await
                .map_err(|e| format!("party XP current session: {e:?}"))?;
        let character = crate::domain::CharacterId::from_bytes(
            *observed.current_character_lease().character_id().as_bytes(),
        )
        .map_err(debug)?;
        h.root
            .read_current_character(&recovery, character)
            .await
            .map_err(|e| format!("party XP current root: {e:?}"))?;
        owner
            .admit_progression_session(&mut AdmittedSession {
                game_session_id: session,
                world_id: world,
                channel_id: channel,
                runtime_actor: Some(actor),
                first_entry: FirstEntryOutcome::NotApplicable,
                controller: None,
                continuity: SessionContinuity::FRESH,
                item_fence: None,
            })
            .await;
        let (facts, build) = initialize_build(
            &owner,
            session,
            if index == 0 || matches!(case, Case::Modifiers) {
                "monk"
            } else {
                "knight"
            },
            90 + index as u8,
        )
        .await?;
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        assert!(
            states
                .initialize(&runtime, actor, session, facts, (0, 0), owner.owner_now())
                .is_some()
        );
        assert!(states.load_owned_build(&runtime, actor, session, &build));
    }
    for (actor, session) in &players {
        assert!(owner.refresh_source_party_presence(*actor, *session).await);
    }
    owner
        .states
        .chat
        .admit(players[0].1, "Party Leader", owner.owner_now());
    owner
        .states
        .chat
        .admit(players[1].1, "Party Member", owner.owner_now());
    for (index, command_id, text, expected) in [
        (
            0,
            3,
            "/sharedxp off",
            oteryn_protocol_oteryn::chat::ChatDisposition::Ok,
        ),
        (
            1,
            2,
            "/sharedxp on",
            oteryn_protocol_oteryn::chat::ChatDisposition::Rejected,
        ),
        (
            0,
            4,
            "/sharedxp on",
            oteryn_protocol_oteryn::chat::ChatDisposition::Ok,
        ),
    ] {
        let (actor, session) = players[index];
        let result = crate::gameplay_transport::connection::FreshAdmissionAuthority::chat_intent(
            &owner,
            actor,
            session,
            command_id,
            oteryn_protocol_oteryn::chat::ChatIntent::Say {
                mode: oteryn_protocol_oteryn::chat::ChatSpeechMode::Say,
                text: text.into(),
            },
        )
        .await;
        assert_eq!(result.disposition, expected);
        let enabled: bool = sqlx::query_scalar("SELECT shared_xp_enabled FROM game_parties")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(enabled, command_id == 4);
    }
    assert_eq!(owner.states.chat.drain(players[0].1).len(), 2);
    assert_eq!(owner.states.chat.drain(players[1].1).len(), 1);
    let party_spell = (1..=u32::try_from(spells.source_len())?)
        .filter_map(std::num::NonZeroU32::new)
        .find(|index| {
            spells.source_indexed(*index).is_some_and(|(spell, _)| {
                matches!(spell.execution, crate::spell::Execution::PartyBuff(_))
            })
        })
        .ok_or("production party spell")?;
    let feedback = owner
        .cast_spell_dispatch(
            players[0].0,
            players[0].1,
            20,
            crate::gameplay_transport::actor_spell::SpellCastIntent {
                spell: party_spell,
                target: oteryn_protocol_oteryn::actor_spell::SpellTarget::None,
                aim_at_target: false,
            },
        )
        .await
        .ok_or("party cast completed")?;
    assert_ne!(
        feedback.disposition,
        oteryn_protocol_oteryn::actor_spell::SpellCastDisposition::Cast,
        "unbuilt/unentitled fixture cannot cast"
    );
    assert!(
        matches!(owner.states.chat.next(players[0].1),Some(oteryn_protocol_oteryn::chat::ChatLine::Private{speaker_name,text})
        if speaker_name=="Game" && text.starts_with("Party spell"))
    );
    if matches!(case, Case::Shared) {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        let denied=sqlx::query("UPDATE game_character_build_state SET stamina_minutes=0 WHERE character_id=encode($1,'hex')::uuid")
            .bind(first.current_character_lease().character_id().as_bytes().as_slice()).execute(&mut *tx).await.expect_err("XP consumer cannot assign stamina");
        assert_eq!(
            denied.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("42501")
        );
        tx.rollback().await?;
    }
    let first_id = first.current_character_lease().character_id();
    let second_id = second.current_character_lease().character_id();
    if matches!(case, Case::Modifiers) {
        sqlx::query("INSERT INTO game_character_prey_slots(character_id,slot,state,creature_key,bonus_type,grade,bonus_percent,hunting_s_left,slot_epoch) VALUES(encode($1,'hex')::uuid,1,'ACTIVE','oteryn:creature.rat','XP',10,40,7200,1)")
            .bind(second_id.as_bytes().as_slice()).execute(&h.pool).await?;
        sqlx::query("UPDATE game_character_build_state SET stamina_minutes=840 WHERE character_id=encode($1,'hex')::uuid")
            .bind(second_id.as_bytes().as_slice()).execute(&h.pool).await?;
    }
    if matches!(case, Case::Exhausted) {
        sqlx::query("UPDATE game_character_build_state SET stamina_minutes=0 WHERE character_id=encode($1,'hex')::uuid")
            .bind(second_id.as_bytes().as_slice()).execute(&h.pool).await?;
    }
    if matches!(case, Case::Pz) {
        let fence = owner
            .current_quest_fence(players[0].1)
            .await
            .map_err(|_| "PZ fence")?
            .ok_or("PZ fence")?;
        h.root
            .write_ahead_pvp_deadlines(
                &recovery,
                &h.node,
                fence,
                vec![crate::durability::pvp_state::PvpWriteAhead {
                    character_id: fence.character_id,
                    kinds: crate::durability::pvp_state::PvpDeadlineKinds {
                        pz_block: true,
                        ..Default::default()
                    },
                }],
            )
            .await
            .map_err(debug)?;
    }
    if matches!(case, Case::Foreign) {
        let (actor, session) = players[2];
        {
            let runtime = owner.runtime.lock().await;
            assert_eq!(
                owner.states.attack.lock().await.set_target(
                    &runtime,
                    Some(&room),
                    actor,
                    session,
                    owner.owner_now(),
                    Some(rat),
                    30
                ),
                oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
            );
        }
        for pass in 0..64 {
            owner.refresh_attack_weapon(actor, session).await;
            owner.drain_auto_attacks().await;
            let runtime = owner.runtime.lock().await;
            let health = runtime
                .visible_entities()
                .creatures
                .iter()
                .find(|c| c.actor.placement_identity() == rat.identity)
                .and_then(|c| runtime.companion_snapshot(c.actor).ok())
                .ok_or("living rat")?
                .health;
            if health <= 15 {
                owner.states.attack.lock().await.set_target(
                    &runtime,
                    Some(&room),
                    actor,
                    session,
                    owner.owner_now(),
                    None,
                    31,
                );
                break;
            }
            assert!(pass < 63, "foreign contributor hit");
            drop(runtime);
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(Duration::from_secs(3))
                .expect("clock");
        }
    }
    if matches!(case, Case::OneHit) {
        // The ally earns real activity first; the killer's first hit is lethal.
        let (actor, session) = players[1];
        let target_actor = {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .commit_monster_lab_damage(creature.actor, b"party-xp-one-hit-fixture", 19)
                .map_err(debug)?;
            runtime
                .visible_entities()
                .creatures
                .iter()
                .find(|c| c.actor != creature.actor)
                .map(|c| c.actor)
                .ok_or("activity creature")?
        };
        {
            let runtime = owner.runtime.lock().await;
            let visible = runtime.visible_entities();
            let row = visible
                .creatures
                .iter()
                .find(|c| c.actor == target_actor)
                .ok_or("activity target")?;
            let target = oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                identity: target_actor.placement_identity(),
                generation: row.generation,
            };
            assert_eq!(
                owner.states.attack.lock().await.set_target(
                    &runtime,
                    Some(&room),
                    actor,
                    session,
                    owner.owner_now(),
                    Some(target),
                    30
                ),
                oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
            );
        }
        for pass in 0..64 {
            owner.refresh_attack_weapon(actor, session).await;
            owner.drain_auto_attacks().await;
            let runtime = owner.runtime.lock().await;
            let state = runtime.companion_snapshot(target_actor).map_err(debug)?;
            if state.health < state.maximum_health {
                owner.states.attack.lock().await.set_target(
                    &runtime,
                    Some(&room),
                    actor,
                    session,
                    owner.owner_now(),
                    None,
                    31,
                );
                break;
            }
            assert!(pass < 63, "ally earned committed activity");
            drop(runtime);
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(Duration::from_secs(3))
                .expect("clock");
        }
    }
    for (index, (actor, session)) in players.iter().copied().enumerate() {
        if index == 2 || (index == 1 && matches!(case, Case::Inactive | Case::OneHit)) {
            continue;
        }
        let runtime = owner.runtime.lock().await;
        if matches!(case, Case::Inactive) {
            // This is a level-one solo fist attacker (the build seed is not a level).
            // Balanced-mode chip damage depended on the old content-seeded draws to
            // kill within 64 passes. Use the real offensive mode for this XP fixture.
            assert!(owner.states.attack.lock().await.set_modes(
                &runtime,
                actor,
                session,
                oteryn_protocol_oteryn::attack::FightModes {
                    fight_mode: oteryn_protocol_oteryn::attack::FightMode::Offensive,
                    ..oteryn_protocol_oteryn::attack::FightModes::DEFAULT
                },
            ));
        }
        assert_eq!(
            owner.states.attack.lock().await.set_target(
                &runtime,
                Some(&room),
                actor,
                session,
                owner.owner_now(),
                Some(rat),
                30
            ),
            oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
        );
    }
    for pass in 0..64 {
        for (actor, session) in &players {
            owner.refresh_attack_weapon(*actor, *session).await;
        }
        owner.drain_auto_attacks().await;
        if players.iter().any(|(_, session)| {
            owner
                .states
                .attack
                .try_lock()
                .is_ok_and(|a| a.kills().has_session(*session))
        }) {
            break;
        }
        assert!(pass < 63, "real swings killed the rat");
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(Duration::from_secs(3))
            .expect("clock");
    }
    if matches!(case, Case::Concurrent) {
        let first_principal = {
            let attack = owner.states.attack.lock().await;
            players
                .iter()
                .find(|(_, s)| attack.kills().has_session(*s))
                .map(|(_, s)| *s)
                .ok_or("first queued kill")?
        };
        let (actor, session) = *players
            .iter()
            .find(|(_, s)| *s != first_principal)
            .ok_or("other principal")?;
        let target = {
            let runtime = owner.runtime.lock().await;
            let creature = runtime
                .visible_entities()
                .creatures
                .iter()
                .find(|c| c.actor.placement_identity() != rat.identity)
                .map(|c| c.actor)
                .ok_or("second creature")?;
            let visible = runtime.visible_entities();
            let row = visible
                .creatures
                .iter()
                .find(|c| c.actor == creature)
                .ok_or("second creature")?;
            let target = oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                identity: creature.placement_identity(),
                generation: row.generation,
            };
            assert_eq!(
                owner.states.attack.lock().await.set_target(
                    &runtime,
                    Some(&room),
                    actor,
                    session,
                    owner.owner_now(),
                    Some(target),
                    40
                ),
                oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
            );
            target
        };
        let mut other_active = false;
        for pass in 0..64 {
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(Duration::from_secs(3))
                .expect("clock");
            for (attacker, attacker_session) in &players {
                owner
                    .refresh_attack_weapon(*attacker, *attacker_session)
                    .await;
            }
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
            if !other_active {
                let runtime = owner.runtime.lock().await;
                let snapshot = runtime
                    .visible_entities()
                    .creatures
                    .iter()
                    .find(|c| c.actor.placement_identity() == target.identity)
                    .and_then(|c| runtime.companion_snapshot(c.actor).ok())
                    .ok_or("second creature")?;
                if snapshot.health <= snapshot.maximum_health / 2 {
                    let (other, other_session) = *players
                        .iter()
                        .find(|(_, s)| *s == first_principal)
                        .ok_or("other actor")?;
                    owner.states.attack.lock().await.set_target(
                        &runtime,
                        Some(&room),
                        other,
                        other_session,
                        owner.owner_now(),
                        Some(target),
                        40,
                    );
                    other_active = true;
                }
            }
            assert!(pass < 63, "second real kill");
        }
    }
    if matches!(case, Case::Stale) {
        sqlx::query("UPDATE game_durability_admission_character_guards SET lease_generation=lease_generation+1 WHERE character_id=encode($1,'hex')::uuid")
            .bind(second_id.as_bytes().as_slice()).execute(&h.pool).await?;
        let principal = {
            let attack = owner.states.attack.lock().await;
            players
                .iter()
                .find(|(_, s)| attack.kills().has_session(*s))
                .map(|(_, s)| *s)
                .ok_or("queued kill")?
        };
        assert_eq!(
            owner.drain_kill_rewards(principal).await,
            DrainOutcome::Deferred
        );
        let counts:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_character_xp_receipts),(SELECT count(*) FROM game_party_kill_xp_receipts)")
            .fetch_one(&h.pool).await?;
        assert_eq!(counts, (0, 0));
        eprintln!("PARTY-XP-1 POSTGRES_ASSERTIONS_COMPLETED stale_lease_rejected");
        return Ok(());
    }
    if matches!(case, Case::PzContention) {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:party-xp-pz:' || encode($1,'hex'),0))")
            .bind(first_id.as_bytes().as_slice()).execute(&mut *tx).await?;
        let principal = {
            let attack = owner.states.attack.lock().await;
            players
                .iter()
                .find(|(_, s)| attack.kills().has_session(*s))
                .map(|(_, s)| *s)
                .ok_or("queued kill")?
        };
        assert_eq!(
            owner.drain_kill_rewards(principal).await,
            DrainOutcome::Deferred
        );
        let heads: i64 = sqlx::query_scalar("SELECT count(*) FROM game_party_kill_xp_receipts")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(heads, 0);
        tx.rollback().await?;
        h.root.maintain_ready_once().await?;
    }
    if matches!(case, Case::Replay) {
        let taken = {
            let attack = owner.states.attack.lock().await;
            let session = players
                .iter()
                .find(|(_, s)| attack.kills().has_session(*s))
                .map(|(_, s)| *s)
                .ok_or("queued kill")?;
            match attack.kills().take(session, 2) {
                crate::gameplay_transport::kill_reward::TakeOutcome::Taken(t) => t,
                _ => return Err("queued kill".into()),
            }
        };
        let permit = owner.spell_lane_permit().await.ok_or("lane")?;
        let entry = taken.entry().ok_or("entry")?;
        let first = owner
            .settle_party_experience(&permit, entry)
            .await
            .map_err(debug)?;
        drop(first);
        drop(permit);
        let (a, b) = join_two(
            async {
                let permit=owner.spell_lane_permit().await.ok_or(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected)?;
                owner.settle_party_experience(&permit,entry).await.map(drop)
            },
            async {
                let permit=owner.spell_lane_permit().await.ok_or(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected)?;
                owner.settle_party_experience(&permit,entry).await.map(drop)
            },
        )
        .await;
        a.map_err(debug)?;
        b.map_err(debug)?;
        taken.requeue();
        owner.revision_sequencer = Default::default();
    }
    if matches!(case, Case::Rollback) {
        let hex = second_id
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let function = format!(
            "CREATE FUNCTION party_xp_test_fault() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF uuid_send(NEW.character_id)=decode('{hex}','hex') THEN RAISE EXCEPTION 'PARTY_XP_TEST_MEMBER_FAILURE'; END IF; RETURN NEW; END $$"
        );
        sqlx::query(sqlx::AssertSqlSafe(function))
            .execute(&h.pool)
            .await?;
        sqlx::query("CREATE TRIGGER party_xp_test_fault BEFORE INSERT ON game_character_xp_receipts FOR EACH ROW EXECUTE FUNCTION party_xp_test_fault()")
            .execute(&h.pool).await?;
        let principal = {
            let attack = owner.states.attack.lock().await;
            players
                .iter()
                .find(|(_, s)| attack.kills().has_session(*s))
                .map(|(_, s)| *s)
                .ok_or("queued kill")?
        };
        assert_eq!(
            owner.drain_kill_rewards(principal).await,
            DrainOutcome::Deferred
        );
        let counts:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_character_xp_receipts),(SELECT count(*) FROM game_party_kill_xp_receipts)")
            .fetch_one(&h.pool).await?;
        assert_eq!(counts, (0, 0));
        let stamina: Vec<i32> = sqlx::query_scalar(
            "SELECT stamina_minutes FROM game_character_build_state ORDER BY character_id LIMIT 3",
        )
        .fetch_all(&h.pool)
        .await?;
        assert!(stamina.iter().all(|value| *value == 2520));
        sqlx::query("DROP TRIGGER party_xp_test_fault ON game_character_xp_receipts")
            .execute(&h.pool)
            .await?;
        sqlx::query("DROP FUNCTION party_xp_test_fault()")
            .execute(&h.pool)
            .await?;
        h.root.maintain_ready_once().await?;
    }
    if matches!(case, Case::Concurrent) {
        let (a, b) = join_two(
            owner.drain_kill_rewards(players[0].1),
            owner.drain_kill_rewards(players[1].1),
        )
        .await;
        // Native semantic-pass admission can defer the contender before it reaches the
        // lane. A typed loser stays queued; retry must finish without duplicating XP.
        assert!(matches!(
            (a, b),
            (DrainOutcome::Drained, DrainOutcome::Drained)
                | (DrainOutcome::Drained, DrainOutcome::Deferred)
                | (DrainOutcome::Deferred, DrainOutcome::Drained)
        ));
        h.root.maintain_ready_once().await?;
        for (_, session) in &players {
            assert_eq!(
                owner.drain_kill_rewards(*session).await,
                DrainOutcome::Drained
            );
        }
    } else {
        for (_, session) in &players {
            assert_eq!(
                owner.drain_kill_rewards(*session).await,
                DrainOutcome::Drained
            );
        }
    }
    let rows:Vec<(Vec<u8>,i64)>=sqlx::query_as("SELECT uuid_send(character_id),xp_awarded FROM game_party_kill_xp_members ORDER BY character_id").fetch_all(&h.pool).await?;
    let first_award = rows
        .iter()
        .filter(|r| r.0 == first_id.as_bytes())
        .map(|r| r.1)
        .sum::<i64>();
    let second_award = rows
        .iter()
        .filter(|r| r.0 == second_id.as_bytes())
        .map(|r| r.1)
        .sum::<i64>();
    if matches!(case, Case::Inactive) {
        assert_eq!((first_award, second_award), (5, 0));
    } else if matches!(case, Case::Concurrent) {
        assert_eq!((first_award, second_award), (11, 11));
    } else if matches!(case, Case::Foreign) {
        let foreign = third
            .as_ref()
            .ok_or("third character")?
            .current_character_lease()
            .character_id();
        let award = rows
            .iter()
            .filter(|r| r.0 == foreign.as_bytes())
            .map(|r| r.1)
            .sum::<i64>();
        assert_eq!((first_award, second_award, award), (2, 2, 1));
    } else if matches!(case, Case::Exhausted) {
        assert_eq!((first_award, second_award), (4, 0));
    } else if matches!(case, Case::Pz) {
        assert!(first_award + second_award <= 5);
        let shared: bool = sqlx::query_scalar("SELECT shared FROM game_party_kill_xp_receipts")
            .fetch_one(&h.pool)
            .await?;
        assert!(!shared);
    } else {
        assert_eq!(
            (first_award, second_award),
            if matches!(case, Case::Modifiers) {
                (3, 2)
            } else {
                (4, 4)
            }
        );
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_party_kill_xp_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        count,
        if matches!(case, Case::Concurrent) {
            2
        } else {
            1
        }
    );
    let xp: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_xp_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        xp,
        if matches!(case, Case::Concurrent) {
            4
        } else if matches!(case, Case::Foreign) {
            3
        } else if matches!(case, Case::Inactive | Case::Exhausted) {
            1
        } else {
            2
        }
    );
    if matches!(case, Case::Modifiers) {
        let grade:(i16,i16,i32)=sqlx::query_as("SELECT prey_grade,prey_percent,stamina_minutes FROM game_party_kill_xp_members WHERE character_id=encode($1,'hex')::uuid")
            .bind(second_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(grade, (10, 40, 840));
        let remaining: i32 =
            sqlx::query_scalar("SELECT hunting_s_left FROM game_character_prey_slots")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(remaining, 7200);
    }
    eprintln!("PARTY-XP-1 POSTGRES_ASSERTIONS_COMPLETED receipts={xp}");
    Ok(())
}

#[path = "party_xp_postgres_support.rs"]
mod support;
use support::{initialize_build, join_two};
