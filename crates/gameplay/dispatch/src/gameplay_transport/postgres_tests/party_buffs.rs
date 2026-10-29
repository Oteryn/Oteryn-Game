//! S13c: production party conditions reach only nearby members, including the caster.
use crate::bestiary_postgres_harness::{
    self as fixture, CHANNEL, Harness, TestResult, WORLD, debug, id,
};
use crate::foundation::{
    ChannelId, ChannelRuntimeV1, CommandId, CommandRef, MovementLocalPosition, PartyAction, WorldId,
};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, FirstEntryOutcome, SessionContinuity},
};
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
use oteryn_gameplay_domain::domain::condition::ConditionValues;
use std::time::Duration;
use tokio::sync::Mutex;

#[test]
fn party_buffs_real_dispatch_applies_values_and_duration_only_in_range_postgres() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            fixture::runtime().map_err(debug)?.block_on(async move {
                let h = Harness::create(admin, "party_buffs", false)
                    .await
                    .map_err(debug)?;
                let result = scenario(&h).await.map_err(|error| error.to_string());
                h.cleanup().await.map_err(debug)?;
                result
            })
        })?
        .join()
        .map_err(|_| "party buff thread panicked")??;
    Ok(())
}

fn open_party_world(
    world: WorldId,
    channel: ChannelId,
) -> TestResult<crate::map::boot::BundleWorld> {
    use crate::map::boot;
    use oteryn_world_bundle::{bundle, writer};

    // The captured Thalom slice is only 32 tiles wide. Use a longer open row so
    // the distant member has real ground beyond the 36-tile party spell range.
    let (bytes, mut pins) = boot::tests::bundle();
    let mut data = bundle::read(&bytes).map_err(debug)?;
    data.manifest.world.max_x = 64;
    data.manifest.palette.truncate(1);
    data.manifest.palette[0].key = "oteryn:item.tibia.i100".into();
    let sector = data.sectors.first().ok_or("party ground sector")?;
    let ground = sector.tiles.first().ok_or("party ground tile")?.clone();
    data.sectors = (0..=1)
        .map(|sx| writer::Sector {
            floor: -7,
            sx,
            sy: 0,
            tiles: (1..=40)
                .filter(|x| x / oteryn_world_bundle::sector::SECTOR_SIZE == sx)
                .map(|x| {
                    let mut tile = ground.clone();
                    tile.x = x;
                    tile
                })
                .collect(),
        })
        .collect();
    let bytes = writer::write(&data.manifest, &data.sectors, &data.spawns).map_err(debug)?;
    pins.digest = bundle::read(&bytes).map_err(debug)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    Ok(boot::boot(
        &bytes,
        &boot::BootPins {
            bundle: pins,
            map_revision: crate::map::overlay::map_revision(&base),
            start: crate::map::overlay::TilePos {
                x: 1,
                y: 0,
                floor: -7,
            },
        },
        world,
        channel,
        |_| None,
    )
    .map_err(debug)?)
}

async fn scenario(h: &Harness) -> TestResult {
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
    let third =
        crate::world_party_runtime_postgres_cases::committed_fresh_player(h, 62, 64, 72, now)
            .await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let recovery = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
    let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;
    let (controller, pin) =
        super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
    let active = controller.active().ok_or("active")?;
    let native = active.native_gameplay().ok_or("native")?;
    let bundle = open_party_world(world, channel)?;
    let cells = bundle
        .movement_cells(pin.qualified_room().movement_cells())
        .map_err(debug)?;
    let position = MovementLocalPosition {
        x: 1,
        y: 0,
        floor: 7,
    };
    let room = pin
        .qualified_room()
        .bundle_view(
            cells,
            bundle.frame_binding_digest(world),
            bundle.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: position.x,
                y: position.y,
                floor: position.floor,
            },
        )
        .ok_or("party ground room")?;
    let pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        room.frame_binding().digest(),
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
    let mut players = Vec::new();
    let currents = [&first, &second, &third];
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
    // Establish membership through the durable party command owner.
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
        (
            0,
            2,
            PartyAction::Invite {
                target: *third.current_character_lease().character_id().as_bytes(),
            },
        ),
        (2, 1, PartyAction::Leave),
    ] {
        let current = [&first, &second, &third][index];
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
        let action = if index != 0 {
            PartyAction::Accept {
                party: party.ok_or("party")?,
            }
        } else {
            action
        };
        let visible = if matches!(action, PartyAction::Invite { .. }) {
            crate::durability::admission_journal::party_target_binding::prove_visible_party_target_in_transaction(&mut tx,&h.root,&authority,&runtime,
                players[0].0,players[if number == 1 { 1 } else { 2 }].0,players[if number == 1 { 1 } else { 2 }].1).await.map_err(debug)?
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
        vec![rustls::pki_types::CertificateDer::from(include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/ca.der").to_vec())],
        vec![rustls::pki_types::CertificateDer::from(include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client.der").to_vec())],
        rustls::pki_types::PrivateKeyDer::try_from(include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client-key.der").to_vec())?).map_err(debug)?;
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
    let (consumer, clock, activation, _) =
        crate::gameplay_transport::spell_premium_coordinator::purchase_fixture::current(id(62));
    let (_, _, _, member_premium) =
        crate::gameplay_transport::spell_premium_coordinator::purchase_fixture::current(id(63));
    consumer.prove_for_test(member_premium);
    h.root
        .latch_premium_activation(
            &activation.activation_id,
            activation.switch_over_us,
            activation.switch_over_us + 60_000_000,
        )
        .await
        .map_err(debug)?;
    let premium =
        crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
            consumer,
            Some(clock),
            Some(activation),
            Some(h.root.clone()),
        );
    owner.premium_coordinator = Some(&premium);
    owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(&room), Some(&room));
    assert!(owner.ensure_source_map_initialized().await);
    owner.clock_origin = std::time::Instant::now()
        .checked_sub(Duration::from_secs(60))
        .ok_or("clock")?;
    for (index, (actor, session)) in players.iter().copied().enumerate() {
        let (observed, _) =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(session)
                .await
                .map_err(|e| format!("party buff current session: {e:?}"))?;
        let character = crate::domain::CharacterId::from_bytes(
            *observed.current_character_lease().character_id().as_bytes(),
        )
        .map_err(debug)?;
        h.root
            .read_current_character(&recovery, character)
            .await
            .map_err(|e| format!("party buff current root: {e:?}"))?;
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
            if index == 0 { "paladin" } else { "monk" },
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
        states
            .get_mut(&runtime, actor, session)
            .ok_or("party state")?
            .enable_owned_training(&build, native.training_formula().ok_or("training formula")?)
            .map_err(debug)?;
    }
    for (actor, session) in &players {
        assert!(owner.refresh_source_party_presence(*actor, *session).await);
    }
    // Current Global supersedes Crystal: Enlighten is premium, +2 mana/2s for 120s.
    // docs/reference/spells/r21-local-candidate/bundles/instant-enlighten_party/manifest.json:100,440
    // Train retains +3 fist: Global Fandom F1182927, accepted owner decision D213.
    // docs/reference/spells/r21-local-candidate/bundles/instant-train_party/manifest.json:452
    // Membership was established while visible; 38 east steps put member 2 beyond both
    // casters' 36-tile Chebyshev range, while member 1 remains one step east.
    {
        let mut runtime = owner.runtime.lock().await;
        for (actor, steps) in [(players[1].0, 1), (players[2].0, 38)] {
            let mut snapshot = runtime.read_actor_position(actor).map_err(debug)?;
            for _ in 0..steps {
                let mut next = snapshot.position();
                next.x += 1;
                snapshot = runtime
                    .borrow_movement_position()
                    .commit_cardinal(snapshot, next)
                    .map_err(debug)?;
            }
        }
    }
    // Each eligible vocation casts on its own session; both nearby actors receive both buffs.
    for (number, key, caster) in [(20, "protect_party", 0), (21, "enlighten_party", 1)] {
        let spell = (1..=u32::try_from(spells.source_len())?)
            .filter_map(std::num::NonZeroU32::new)
            .find(|index| {
                spells
                    .source_indexed(*index)
                    .is_some_and(|(spell, _)| spell.key == format!("candidate:spell/{key}"))
            })
            .ok_or("party spell")?;
        assert!(spells.source_indexed(spell).ok_or("spell")?.0.premium);
        let result = owner
            .cast_spell_dispatch(
                players[caster].0,
                players[caster].1,
                number,
                crate::gameplay_transport::actor_spell::SpellCastIntent {
                    spell,
                    target: oteryn_protocol_oteryn::actor_spell::SpellTarget::None,
                    aim_at_target: false,
                },
            )
            .await
            .ok_or("cast result")?;
        assert_eq!(
            result.disposition,
            oteryn_protocol_oteryn::actor_spell::SpellCastDisposition::Cast,
            "{key}"
        );
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        for (index, (actor, session)) in players.iter().copied().enumerate() {
            let state = states.get(&runtime, actor, session).ok_or("party actor")?;
            let conditions: Vec<_> = state
                .owned_conditions()
                .instances()
                .iter()
                .filter(|instance| instance.definition().key().contains(key))
                .collect();
            assert_eq!(
                conditions.len(),
                usize::from(index != 2),
                "{key} member {index}"
            );
            for instance in conditions {
                assert_eq!(
                    instance.remaining_duration_ms(instance.started_at()),
                    Some(120_000)
                );
                match instance.definition().values() {
                    ConditionValues::SpellSkills {
                        duration_ms,
                        shielding,
                        ..
                    } if key == "protect_party" => {
                        assert_eq!(duration_ms, 120_000);
                        assert_eq!(shielding, 3);
                    }
                    ConditionValues::SpellRegeneration {
                        duration_ms,
                        mana_gain,
                        mana_interval_ms,
                        health_gain,
                        ..
                    } if key == "enlighten_party" => {
                        assert_eq!(
                            (duration_ms, mana_gain, mana_interval_ms, health_gain),
                            (120_000, 2, 2_000, 0)
                        );
                    }
                    other => return Err(format!("unexpected {key} condition: {other:?}").into()),
                }
            }
        }
    }
    Ok(())
}
pub(super) async fn initialize_build(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: crate::foundation::GameSessionId,
    vocation: &str,
    seed: u8,
) -> TestResult<(
    crate::spell::cast::CharacterCastFacts,
    crate::durability::character_build::DurableBuildState,
)> {
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|_| "current fence")?
        .ok_or("fence")?;
    let binding = owner
        .progression_binding(session)
        .ok_or("progression binding")?;
    owner
        .root
        .initialize_character_progression(
            owner.character,
            owner.holder,
            fence,
            crate::durability::character_progression::ProgressionInitializationRequest {
                context: binding.context.clone(),
                policy_revision: binding.policy_revision.clone(),
                reward_revision: binding.reward_revision.clone(),
                policy: binding.policy.clone(),
            },
        )
        .await
        .map_err(debug)?;
    let before = owner
        .root
        .read_character_build_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    let formula = crate::spell::mana_training::CompiledTrainingFormula::from_profile(
        include_bytes!(
            "../../../../../../tools/content-schema/native-gameplay/build-training.json"
        ),
        "content-1",
    )
    .map_err(debug)?;
    let after = crate::durability::character_build::convert_vocation(&formula, &before, vocation)
        .map_err(debug)?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    slot.commit_build(
        owner.root,
        owner.character,
        owner.holder,
        fence,
        crate::durability::character_build::BuildChangeRequest {
            occurrence: crate::durability::character_build::BuildOccurrence::from_bytes(id(seed))
                .map_err(debug)?,
            cause: crate::durability::character_build::BuildCause::VocationChoice,
            before,
            after: after.clone(),
            pruned_stance: None,
        },
        &formula,
        None,
    )
    .await
    .map_err(debug)?;
    slot.commit_experience(
        owner.root,
        owner.character,
        owner.holder,
        fence,
        crate::durability::character_progression::ExperienceAwardRequest {
            occurrence:
                crate::durability::character_progression::ExperienceRewardOccurrence::from_bytes(
                    id(seed + 10),
                )
                .map_err(debug)?,
            amount: oteryn_simulation_determinism::ExactI64::new(16_000_000),
            context: binding.context.clone(),
            policy_revision: binding.policy_revision.clone(),
            reward_revision: binding.reward_revision.clone(),
            policy: binding.policy.clone(),
        },
        None,
    )
    .await
    .map_err(debug)?;
    let crate::gameplay_transport::CastFactsLoad::Ready { facts, .. } =
        crate::gameplay_transport::load_character_cast_facts(
            owner.root,
            owner.character,
            fence.character_id,
        )
        .await
    else {
        return Err("official classed Character facts".into());
    };
    Ok((facts, after))
}
