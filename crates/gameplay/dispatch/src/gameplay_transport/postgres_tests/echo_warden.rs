//! Physical Warden deaths -> production kill queue -> Charm and ordinary corpse Item owners.
use crate::bestiary_postgres_harness::{
    self as fixture, CHANNEL, Harness, TestResult, WORLD, debug, id,
};
use crate::foundation::{
    ChannelId, ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef, GameSessionId,
    MovementLocalPosition, WorldId,
};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    kill_reward::{DrainOutcome, KillSink},
};
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
use oteryn_gameplay_domain::domain::echo_warden::{EchoDeathEffect, EchoVariant};
use oteryn_gameplay_entities::carrier::echo_warden::EchoDeathInput;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use std::time::Duration;
use tokio::sync::Mutex;

const CREATURE: &str = "oteryn:creature.rat";
const POSITION: MovementLocalPosition = MovementLocalPosition {
    x: 2,
    y: 11,
    floor: 7,
};

#[test]
fn echo_warden_all_damagers_first_species_replay_and_real_scroll_custody_postgres() -> TestResult {
    if fixture::configured_admin().is_none() {
        eprintln!("ECHO_WARDEN POSTGRES_CASE_NOT_RUN: no configured PostgreSQL route");
        return Ok(());
    }
    std::thread::Builder::new()
        .name("echo-warden-dispatch".into())
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = fixture::runtime().map_err(|error| error.to_string())?;
            let result: TestResult = runtime.block_on(async {
                let admin = fixture::configured_admin().ok_or("PostgreSQL route required")?;
                let harness = Harness::create(admin, "echo_warden_dispatch", false).await?;
                let result = scenario(&harness).await;
                harness.cleanup().await?;
                result
            });
            result.map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Echo Warden scenario thread panicked")?
        .map_err(Into::into)
}

async fn scenario(h: &Harness) -> TestResult {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let first =
        crate::world_party_runtime_postgres_cases::committed_fresh_player(h, 60, 63, 70, now)
            .await?;
    let second =
        crate::world_party_runtime_postgres_cases::committed_fresh_player(h, 61, 64, 71, now)
            .await?;
    let bystander =
        crate::world_party_runtime_postgres_cases::committed_fresh_player(h, 62, 65, 72, now)
            .await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
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
                "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
            )),
        );
    let active = controller.active().ok_or("active native generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let room = pin.qualified_room().clone();
    let pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        room.map_revision_digest(),
        (POSITION.x, POSITION.y, POSITION.floor),
    );
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,NULL,$4,$5,$6)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(pin.activation_sequence().to_string())
        .bind(pin.server_artifact_digest().as_slice()).bind(pin.client_artifact_digest().as_slice())
        .bind(pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        channel,
        h.node.fact().node_id(),
        h.node.fact().registration_revision(),
        1,
        1,
        "runtime-scope-assignment:1",
        40,
        pin,
    )
    .map_err(debug)?;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    let mut players = Vec::new();
    for current in [&first, &second, &bystander] {
        let session = current.current_game_session_id();
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, session, current.current_character_lease())
            .map_err(debug)?;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        players.push((actor, session));
    }
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
    let descriptor = crate::native_admission_source::descriptor::ProducerDescriptor::new(
        "platform-test".into(), ("127.0.0.1".into(), 1), "source.test".into(), "source.test".into(),
        vec![rustls::pki_types::CertificateDer::from(include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/ca.der").to_vec())],
        vec![rustls::pki_types::CertificateDer::from(include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client.der").to_vec())],
        rustls::pki_types::PrivateKeyDer::try_from(include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client-key.der").to_vec())?,
    ).map_err(debug)?;
    let evidence = crate::gameplay_transport::fresh_evidence::FreshEvidenceSource::new(descriptor);
    let achievements =
        crate::achievement_catalogue::AchievementCatalogue::embedded().map_err(debug)?;
    let charms =
        crate::content::charm_source::CanonicalCharmCatalogue::embedded().map_err(debug)?;
    let runtime = Mutex::new(runtime);
    let mut owner = crate::gameplay_transport::composed_test_owner::ComposedTestOwner {
        root: &h.root, character: &authority, holder: &h.node, evidence: &evidence,
        world_id: world, channel_id: channel,
        required_capabilities: crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
            &[], first.current_runtime_scope(), 1, "policy-1",
            crate::gameplay_transport::world_required_capabilities::offered_for_scope(false, None),
        ).map_err(debug)?,
        runtime: &runtime, door: &door, spells: native.spell_book(), achievements: &achievements,
        imported_charms: &charms, room: &room,
    }.build();
    owner.active_generation = Some(active);
    owner.clock_origin = std::time::Instant::now()
        .checked_sub(Duration::from_secs(400))
        .ok_or("clock")?;
    let queue = owner.states.attack.lock().await.kills().clone();
    let stars = native
        .reward_table()
        .encounter_facts(CREATURE)
        .and_then(|facts| facts.bestiary_stars)
        .ok_or("native stars")?;
    assert_eq!(stars, 1);
    for item in 53751..=53774 {
        assert!(
            native.reward_table().admitted_source_item(item).is_some(),
            "native scroll {item} must be materializable"
        );
    }
    let principal = crate::combat::CapturedRewardPrincipal {
        character: second.current_character_lease().character_id(),
        lease_generation: second.current_character_lease().generation(),
        session: players[1].1,
        actor: players[1].0,
    };
    let mut first_occurrence = None;
    for round in 0_u64..2 {
        let target = {
            let mut runtime = owner.runtime.lock().await;
            let warden = raid(
                &mut runtime,
                players[0],
                100_000 + round * 100_000,
                round * 10 + 1,
            )?;
            let health = runtime.companion_snapshot(warden).map_err(debug)?.health;
            assert_eq!(health, 80);
            // One damage is far below the ordinary Bestiary threshold. Echo still credits it.
            damage(
                &mut runtime,
                warden,
                players[0],
                1,
                round * 10 + 10,
                399_999,
            )?;
            damage(
                &mut runtime,
                warden,
                players[1],
                health - 1,
                round * 10 + 10,
                400_000,
            )?;
            record(
                &mut runtime,
                &queue,
                native.reward_table(),
                warden,
                principal,
            );
            let occurrence = runtime
                .borrow_combat_death()
                .reward_occurrence(warden, *principal.character.as_bytes())
                .map_err(debug)?
                .0;
            first_occurrence.get_or_insert(occurrence);
            warden
        };
        drain(&owner, players[1].1).await?;
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_echo_warden_rewards")
                .fetch_one(&h.pool)
                .await?,
            2
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT sum(charm_points)::bigint FROM game_character_echo_warden_rewards"
            )
            .fetch_one(&h.pool)
            .await?,
            4
        );
        assert_eq!(scrolls(h).await?, i64::try_from(round + 1)?);
        // Replayed lethal capture re-enters the actual queue, Charm owner and loot receipts.
        {
            let mut runtime = owner.runtime.lock().await;
            record(
                &mut runtime,
                &queue,
                native.reward_table(),
                target,
                principal,
            );
        }
        drain(&owner, players[1].1).await?;
        assert_eq!(scrolls(h).await?, i64::try_from(round + 1)?);
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_character_echo_warden_kill_receipts"
            )
            .fetch_one(&h.pool)
            .await?,
            i64::try_from((round + 1) * 2)?
        );
    }
    let bystander_rewards: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_echo_warden_rewards WHERE character_id=encode($1,'hex')::uuid")
        .bind(bystander.current_character_lease().character_id().as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(bystander_rewards, 0);
    let fence = owner
        .current_quest_fence(players[1].1)
        .await
        .map_err(|()| "fence read")?
        .ok_or("current session")?;
    let occurrence = crate::durability::bestiary_progress::BestiaryKillOccurrence::from_bytes(
        first_occurrence.ok_or("first death")?,
    )
    .map_err(debug)?;
    assert!(
        h.root
            .credit_echo_warden_kill(
                &authority,
                &h.node,
                fence,
                occurrence,
                "oteryn:creature.dragon",
                stars
            )
            .await
            .is_err(),
        "one occurrence cannot change species"
    );
    let mut stale = fence;
    stale.connection_generation =
        crate::foundation::ConnectionGeneration::new(999).map_err(debug)?;
    assert!(
        h.root
            .credit_echo_warden_kill(
                &authority,
                &h.node,
                stale,
                crate::durability::bestiary_progress::BestiaryKillOccurrence::from_bytes(id(199))
                    .map_err(debug)?,
                "oteryn:creature.dragon",
                3
            )
            .await
            .is_err()
    );
    let minion = {
        let mut runtime = owner.runtime.lock().await;
        let actor = runtime
            .visible_entities()
            .creatures
            .into_iter()
            .map(|entry| entry.actor)
            .find(|actor| runtime.echo_variant(*actor) == Some(EchoVariant::Minion))
            .ok_or("raid minion")?;
        let health = runtime.companion_snapshot(actor).map_err(debug)?.health;
        damage(&mut runtime, actor, players[1], health, 30, 400_000)?;
        record(
            &mut runtime,
            &queue,
            native.reward_table(),
            actor,
            principal,
        );
        actor
    };
    drain(&owner, players[1].1).await?;
    assert_eq!(scrolls(h).await?, 2);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_character_echo_warden_kill_receipts"
        )
        .fetch_one(&h.pool)
        .await?,
        4
    );
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .drive_echo_wardens(500_000, |_| true)
            .map_err(debug)?;
        assert!(runtime.companion_snapshot_including_dead(minion).is_err());
        assert_eq!(runtime.echo_portals(500_000).count(), 0);
    }
    refuse_cross_species_reward(
        h,
        second.current_character_lease().character_id().as_bytes(),
    )
    .await?;
    spend_echo_major_points(&owner, native, [players[0].1, players[1].1]).await?;
    assert!(
        sqlx::query("DELETE FROM game_character_echo_warden_rewards")
            .execute(&h.pool)
            .await
            .is_err()
    );
    eprintln!(
        "ECHO_WARDEN POSTGRES_CASE_COMPLETED: two damagers, bystander exclusion, first-species replay, exact scroll custody, stale fence, minion exclusion, cross-species ledger rejection and spendable major Charm Points"
    );
    Ok(())
}

async fn refuse_cross_species_reward(h: &Harness, character: &[u8; 16]) -> TestResult {
    // A valid later zero-point Rat receipt must not be reusable as Dragon's first reward.
    // Both rows satisfy their individual CHECKs; the deferred relational proof rejects them.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_echo_warden_kill_receipts(character_id,occurrence_id,species_key,bestiary_stars,first_species,charm_points) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'oteryn:creature.rat',1,false,0)")
        .bind(character.as_slice()).bind(id(200).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_echo_warden_rewards(character_id,species_key,first_occurrence_id,bestiary_stars,charm_points) VALUES(encode($1,'hex')::uuid,'oteryn:creature.dragon',encode($2,'hex')::uuid,3,10)")
        .bind(character.as_slice()).bind(id(200).as_slice()).execute(&mut *tx).await?;
    let error = tx
        .commit()
        .await
        .err()
        .ok_or("cross-species Echo ledger unexpectedly committed")?;
    let sqlx::Error::Database(error) = error else {
        return Err("expected deferred Echo constraint rejection".into());
    };
    assert_eq!(error.code().as_deref(), Some("23514"));
    assert!(error.message().contains("exact first-species receipt"));
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_character_echo_warden_kill_receipts"
        )
        .fetch_one(&h.pool)
        .await?,
        4
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_echo_warden_rewards")
            .fetch_one(&h.pool)
            .await?,
        2
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT sum(charm_points)::bigint FROM game_character_echo_warden_rewards"
        )
        .fetch_one(&h.pool)
        .await?,
        4
    );
    Ok(())
}

async fn spend_echo_major_points(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    native: &crate::content::native_gameplay::NativeGameplayState,
    sessions: [GameSessionId; 2],
) -> TestResult {
    use crate::durability::charm_state::{
        BestiaryCharmEntry, BestiaryCharmFacts, CharmCommandOccurrence,
    };
    use crate::gameplay_transport::character_progression_binding::{
        ProgressionBinding, ProgressionDurable, ProgressionRounds, bind_character_progression,
    };
    use crate::gameplay_transport::charm::CharmProgressionPort;
    use crate::gameplay_transport::charm::native::{
        CharmConnectionContent, NativeCharmProgressionPort,
    };
    use oteryn_protocol_oteryn::charm::{
        CharmKind, CharmUnlockDisposition, CharmUnlockStageIntent,
    };
    // Same canonical Bestiary source and production fact reader as charm_progression.
    // No fabricated balance, progression seed or disabled relational guard is involved.
    let rows = crate::content::canonical_bestiary_rows(crate::content::ProjectEvidenceLimits {
        max_documents: 5,
        max_document_bytes: 2_500_000,
        max_total_bytes: 8_000_000,
        max_json_depth: 10,
        max_decoded_fields: 65_000,
        max_string_bytes: 1_000_000,
        max_locator_bytes: 128,
        max_locator_segments: 5,
        max_reference_records: 4096,
        max_import_records: 1,
        max_reimport_states: 1,
    })
    .map_err(debug)?;
    let facts =
        BestiaryCharmFacts::new(rows.iter().map(|(race, charm_points)| BestiaryCharmEntry {
            race: race.clone(),
            charm_points: *charm_points,
        }))
        .map_err(debug)?;
    let races: Vec<_> = rows.into_iter().map(|(race, _)| race).collect();
    let progression = native.progression().ok_or("native progression policy")?;
    for session in sessions {
        // committed_fresh_player establishes the real session/Character, but this
        // physical fixture has not run first-entry progression. Use that exact
        // production initializer with the Character root's own interpretation.
        let initialized = Box::pin(bind_character_progression(
            ProgressionDurable {
                root: owner.root,
                character: owner.character,
                holder: owner.holder,
            },
            |profile, ruleset, content| progression.policy_for(profile, ruleset, content).ok(),
            || owner.current_quest_fence(session),
            ProgressionRounds {
                attempts: 1,
                backoff: Duration::ZERO,
            },
        ))
        .await;
        assert!(
            matches!(initialized, ProgressionBinding::Bound(_)),
            "real fresh-Character progression must initialize before Charm spend: {initialized:?}"
        );
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "current Charm fence")?
            .ok_or("current Charm session")?;
        let state = owner
            .root
            .read_character_progression(owner.character, fence.character_id)
            .await
            .map_err(debug)?
            .ok_or("Charm progression")?;
        let content = CharmConnectionContent::new(
            state.context.content,
            owner.imported_charms.catalogue().clone(),
            races.clone(),
            owner.imported_charms,
        )
        .map_err(debug)?;
        let port = NativeCharmProgressionPort::bind(
            owner.root,
            owner.character,
            owner.holder,
            &owner.revision_sequencer,
            fence,
            &content,
            facts.clone(),
        )
        .await
        .map_err(debug)?;
        let view = port.charms().await.map_err(debug)?;
        assert_eq!(
            view.charm_points_available, 2,
            "physical Warden reward is spendable major currency"
        );
        assert_eq!(
            view.minor_charm_echoes_available, 0,
            "Warden does not mint Minor Charm Echoes"
        );
    }
    let fence = owner
        .current_quest_fence(sessions[1])
        .await
        .map_err(|()| "current Charm fence")?
        .ok_or("current Charm session")?;
    let species: Vec<_> = native
        .creature_profiles()
        .records
        .iter()
        .map(|record| record.profile.target.key.as_str())
        .filter(|key| {
            key.starts_with("oteryn:creature.")
                && native
                    .reward_table()
                    .encounter_facts(key)
                    .is_some_and(|facts| facts.bestiary_stars == Some(5))
        })
        .take(8)
        .collect();
    assert_eq!(species.len(), 8);
    {
        // Bounded additional server-qualified reward facts exercise the canonical earning
        // writer. The physical fanout above remains the independent source of the first2.
        let _slot = owner.revision_sequencer.acquire(fence.character_id).await;
        for (index, species) in species.into_iter().enumerate() {
            let occurrence =
                crate::durability::bestiary_progress::BestiaryKillOccurrence::from_bytes(id(
                    210 + u8::try_from(index)?
                ))
                .map_err(debug)?;
            let credit = owner
                .root
                .credit_echo_warden_kill(
                    owner.character,
                    owner.holder,
                    fence,
                    occurrence,
                    species,
                    5,
                )
                .await
                .map_err(debug)?;
            assert!(credit.first_species && !credit.replayed);
            assert_eq!(credit.points, 30);
        }
    }
    let state = owner
        .root
        .read_character_progression(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("Charm progression")?;
    let content = CharmConnectionContent::new(
        state.context.content,
        owner.imported_charms.catalogue().clone(),
        races,
        owner.imported_charms,
    )
    .map_err(debug)?;
    let mut port = NativeCharmProgressionPort::bind(
        owner.root,
        owner.character,
        owner.holder,
        &owner.revision_sequencer,
        fence,
        &content,
        facts,
    )
    .await
    .map_err(debug)?;
    let before = port.charms().await.map_err(debug)?;
    assert_eq!(before.charm_points_available, 242);
    let charm = before
        .charms
        .iter()
        .find(|charm| charm.kind == CharmKind::Major && charm.next_stage_cost == 240)
        .ok_or("canonical240-point major charm")?
        .charm;
    let occurrence = CharmCommandOccurrence::from_bytes(id(220)).map_err(debug)?;
    let intent = CharmUnlockStageIntent {
        charm,
        expected_stage: 0,
    };
    assert_eq!(
        port.unlock_next_stage(occurrence, intent).await,
        CharmUnlockDisposition::Unlocked
    );
    let after = port.charms().await.map_err(debug)?;
    assert_eq!(after.charm_points_available, 2);
    assert_eq!(
        after.minor_charm_echoes_available, 50,
        "minor currency comes from the existing major-stage unlock rule"
    );
    assert!(
        after
            .charms
            .iter()
            .any(|entry| entry.charm == charm && entry.unlocked_stage == 1)
    );
    assert_eq!(
        port.unlock_next_stage(occurrence, intent).await,
        CharmUnlockDisposition::Unlocked
    );
    assert_eq!(
        port.charms().await.map_err(debug)?,
        after,
        "unlock replay cannot spend the Echo earnings twice"
    );
    Ok(())
}

fn damage(
    runtime: &mut ChannelRuntimeV1,
    target: ExactActorRef,
    player: (ExactActorRef, GameSessionId),
    amount: i64,
    command: u64,
    at: u64,
) -> TestResult {
    runtime
        .borrow_exact_actor_commit_at(at)
        .commit_damage_for_bound_attacker(
            target,
            player.0,
            CommandRef::new(player.1, CommandId::new(command).map_err(debug)?),
            0,
            crate::foundation::OwnerDamageCommand {
                target: CREATURE.as_bytes(),
                occurrence: b"echo-owner-kill",
                binding: b"echo-owner-kill-v1",
                damage: amount,
            },
        )
        .map_err(debug)?;
    Ok(())
}

fn raid(
    runtime: &mut ChannelRuntimeV1,
    player: (ExactActorRef, GameSessionId),
    now_ms: u64,
    command: u64,
) -> TestResult<ExactActorRef> {
    let initial = runtime
        .admit_encounter_actor(CREATURE, 20, POSITION)
        .map_err(debug)?;
    runtime
        .install_creature_policy(initial, CREATURE)
        .map_err(debug)?;
    damage(runtime, initial, player, 20, command, now_ms)?;
    let character = runtime
        .bound_attacker_lease(player.0, player.1)
        .map_err(debug)?
        .character_id();
    let (death, occurrence) = {
        let mut owner = runtime.borrow_combat_death();
        crate::combat::project_fixed_one_creature_death(&mut owner, initial).map_err(debug)?;
        (
            owner.projected_death(initial).map_err(debug)?.0,
            owner
                .reward_occurrence(initial, *character.as_bytes())
                .map_err(debug)?
                .0,
        )
    };
    // Deterministic fixture boundary for the 0.05% event; the exact physical death key is real.
    assert_eq!(
        runtime
            .record_echo_death(EchoDeathInput {
                death,
                occurrence,
                bestiary_stars: Some(1),
                has_boss_race: false,
                draw: 1,
                now_ms
            })
            .map_err(debug)?,
        EchoDeathEffect::SchedulePortal
    );
    runtime.remove_encounter_actor(initial).map_err(debug)?;
    runtime
        .drive_echo_wardens(now_ms + 30_000, |_| true)
        .map_err(debug)?;
    assert!(
        runtime
            .activate_echo_portal(player.0, player.1, now_ms + 30_000, 7)
            .map_err(debug)?
    );
    runtime
        .drive_echo_wardens(now_ms + 30_000, |_| true)
        .map_err(debug)?;
    runtime
        .visible_entities()
        .creatures
        .into_iter()
        .map(|entry| entry.actor)
        .find(|actor| runtime.echo_variant(*actor) == Some(EchoVariant::Warden))
        .ok_or_else(|| "physical Warden missing".into())
}

fn record(
    runtime: &mut ChannelRuntimeV1,
    queue: &crate::gameplay_transport::kill_reward::KillSettlementQueue,
    rewards: &crate::content::creature_reward::CreatureRewardTable,
    target: ExactActorRef,
    principal: crate::combat::CapturedRewardPrincipal,
) {
    crate::gameplay_transport::kill_reward::record_creature_kill(
        runtime,
        KillSink {
            prey: None,
            activity: &[],
            queue,
            rewards,
            charms: None,
        },
        target,
        principal,
        400_000,
        Some(oteryn_simulation_determinism::SemanticTimeMicros::from_micros(400_000_000)),
    );
}

async fn drain(owner: &ComposedFreshAdmission<'_, '_, '_>, session: GameSessionId) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while owner.drain_kill_rewards(session).await != DrainOutcome::Drained {
        if tokio::time::Instant::now() >= deadline {
            return Err("Echo kill settlement did not drain".into());
        }
        tokio::task::yield_now().await;
    }
    Ok(())
}

async fn scrolls(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_family='Item' AND definition_production_key BETWEEN 'oteryn:item.tibia.i53751' AND 'oteryn:item.tibia.i53774'")
        .fetch_one(&h.pool).await?)
}
