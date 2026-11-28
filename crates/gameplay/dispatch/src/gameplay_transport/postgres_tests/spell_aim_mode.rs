//! WU26-14 (Winter Update 2026 test server): a wave cast aimed at the cursor tile through the
//! wire command, the cast route and the ordinary combat preparation, with real Character owners.
//! The old `aim_at_target` flag with a POSITION is the CURSOR option; a session holding capability
//! 480 sends field 5 `aim_mode` CURSOR instead, which a session without it has refused.
use super::death_ml_loss_composed_fixtures::{current_fence, production_generation_with_source};
use super::fixture::{Harness, debug, id, join_two, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};
use crate::durability::character_progression::{
    ExperienceAwardRequest, ExperienceRewardOccurrence,
};
use crate::foundation::{
    ChannelContentPin, CharacterLease, MessageType, WorldId, decode_wire_envelope,
};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::spell_character_facts::{CastFactsLoad, load_character_cast_facts};
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, actor_spell, decode_command_result, encode_client_command,
};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Scorch: a level-1 sorcerer wave whose matrix centre is the tile next to the caster.
const SPELL: &str = "candidate:spell/scorch";
const MANA: i64 = 8;

#[test]
fn spell_aim_mode_postgres_cursor_wave_hits_the_cursor_direction_from_the_caster() -> TestResult {
    run_case("spell_aim_mode", false)
}

/// Capability 480: field 5 `aim_mode` CURSOR is refused without the capability and casts the
/// aimed wave with it, through the negotiated decode of the cast route.
#[test]
fn spell_aim_mode_postgres_capability_480_aim_mode_cursor_needs_the_capability() -> TestResult {
    run_case("spell_aim_mode_480", true)
}

fn run_case(tag: &'static str, negotiated: bool) -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(move || {
            run(async move |admin| scenario(admin, tag, negotiated).await)
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "spell aim mode thread panicked")?
        .map_err(Into::into)
}

async fn scenario(admin: String, tag: &str, negotiated: bool) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, tag, false, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) = production_generation_with_source(
        world,
        "content-1",
        Some(include_bytes!(
            "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
        )),
    )
    .map_err(debug)?;
    let active = controller.active().ok_or("active")?;
    let native = active.native_gameplay().ok_or("native")?;
    let room = pin.qualified_room().clone();
    let cp = ChannelContentPin::from_activation(
        world,
        1,
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (2, 11, -7),
    );
    let mut deps = Dependencies::with_content_and_capacity(
        cp,
        room.clone(),
        native.spell_book().clone(),
        6,
    )?;
    deps.combat_room = Some(room);
    let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
    h.root
        .latch_premium_activation(
            &activation.activation_id,
            activation.switch_over_us,
            activation.switch_over_us + 60_000_000,
        )
        .await
        .map_err(debug)?;
    let premium = SpellPremiumCoordinator::new(
        consumer,
        Some(clock),
        Some(activation),
        Some(h.root.clone()),
    );
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(active);
    owner.premium_coordinator = Some(&premium);
    let mut admitted = deps.admitted;
    let actor = deps.actor;
    let session = admitted.game_session_id;
    owner.admit_premium(id(40), session);
    owner.admit_progression_session(&mut admitted).await;
    let progression = owner.progression_binding(session).ok_or("progression")?;
    let formula = native.training_formula().ok_or("training")?;
    let build =
        convert_vocation(formula, &DurableBuildState::default(), "sorcerer").map_err(debug)?;
    h.root
        .commit_character_build(
            &authority,
            &h.node,
            current_fence(&owner, session).await?,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(90)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                before: DurableBuildState::default(),
                after: build.clone(),
                pruned_stance: None,
            },
            formula,
        )
        .await
        .map_err(debug)?;
    let caster_experience = progression.policy.thresholds.iter().find(|entry| entry.level == 15)
        .ok_or("level 15 experience threshold")?.minimum_experience;
    h.root
        .commit_character_experience(
            &authority,
            &h.node,
            current_fence(&owner, session).await?,
            ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(id(91)).map_err(debug)?,
                amount: caster_experience,
                context: progression.context.clone(),
                policy_revision: progression.policy_revision.clone(),
                reward_revision: progression.reward_revision.clone(),
                policy: progression.policy.clone(),
            },
        )
        .await
        .map_err(debug)?;
    let CastFactsLoad::Ready { facts, .. } = load_character_cast_facts(
        &h.root,
        &authority,
        current_fence(&owner, session).await?.character_id,
    )
    .await
    else {
        return Err("durable caster facts".into());
    };
    assert!(i64::from(facts.max_mana) >= MANA, "max mana");
    let targets;
    let moving_rat;
    let rat_health;
    let caster;
    {
        let mut runtime = deps.runtime.lock().await;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                session,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                    2,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        // A first entry has no facing yet: the aim gives the wave its direction.
        caster = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        runtime
            .install_companion_policies(native.policies().clone())
            .map_err(debug)?;
        // Start at full fixture health so the hit changes the wire percentage without killing.
        // The front rat stays at the wave's origin. The other starts beside its impact tile.
        rat_health = u64::try_from(runtime.companion_policy("oteryn:creature.rat").map_err(debug)?.maximum_health)?;
        runtime
            .realize_activation_spawns(
                [(1, 0), (2, 1)]
                    .map(|(dx, dy)| (
                        vec![(caster.x + dx, caster.y + dy, caster.floor)],
                        60_000,
                        5_000,
                        "oteryn:creature.rat".into(),
                        rat_health,
                    ))
                    .to_vec(),
            )
            .map_err(|error| format!("realize aim target spawns: {error:?}"))?;
        targets = runtime
            .visible_entities()
            .creatures
            .into_iter()
            .map(|creature| creature.actor)
            .collect::<Vec<_>>();
        assert_eq!(targets.len(), 2);
        moving_rat = targets
            .iter()
            .copied()
            .find(|target| {
                runtime
                    .read_actor_position(*target)
                    .is_ok_and(|at| at.position().y == caster.y + 1)
            })
            .ok_or("moving rat")?;
        let mut states = owner.states.spell_states.lock().await;
        assert!(
            states
                .initialize(&runtime, actor, session, facts, (0, 0), owner.owner_now())
                .is_some()
        );
        assert!(states.load_owned_build(&runtime, actor, session, &build));
        states
            .get_mut(&runtime, actor, session)
            .ok_or("caster")?
            .enable_owned_training(&build, formula)
            .map_err(debug)?;
    }
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&h.pool).await?;
    {
        let mut tx = h.pool.begin().await?;
        // Fixture custody only, as in the crosshair cases.
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        // Combat mode 1 is Attack; the attack formulas refuse an unknown mode.
        sqlx::query("UPDATE game_character_equipment_state SET combat_mode=1 WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
    }
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
    h.root
        .initialize_familiar_group(
            &authority,
            &h.node,
            current_fence(&owner, session).await?,
        )
        .await
        .map_err(debug)?;
    let current =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(session)
            .await?
            .0;
    admitted.item_fence = item_fence_of(current);
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[6, 7, 17, 23])
            .ok_or("chat capabilities")?;
    let spell = (1..=deps.spells.source_len())
        .find_map(|i| {
            std::num::NonZeroU32::new(u32::try_from(i).ok()?).filter(|i| {
                deps.spells
                    .source_indexed(*i)
                    .is_some_and(|(s, active)| active && s.key == SPELL)
            })
        })
        .ok_or("spell index")?;
    let before = owner
        .observe_vitals(actor, session)
        .await
        .ok_or("before vitals")?
        .1
        .mana;
    // The cursor lies four tiles beyond the rat: a position cast would start the wave there and
    // miss it; the aimed wave turns east and starts next to the caster.
    let cursor = actor_spell::SpellTarget::Position(actor_spell::SpellTargetPosition {
        x: caster.x + 5,
        y: caster.y,
        floor: caster.floor.abs(),
    });
    let payload = if negotiated {
        actor_spell::encode_aimed_spell_cast_intent(&actor_spell::AimedSpellCastIntent {
            intent: actor_spell::SpellCastIntent {
                spell,
                target: cursor,
                aim_at_target: false,
            },
            aim_mode: Some(actor_spell::SpellAimMode::Cursor),
        })
        .map_err(debug)?
    } else {
        actor_spell::encode_spell_cast_intent(&actor_spell::SpellCastIntent {
            spell,
            target: cursor,
            aim_at_target: true,
        })
    };
    if negotiated {
        // Without capability 480 field 5 is unknown: the route refuses it and nothing is spent.
        let (refused, _) = exchange(
            &owner,
            &mut admitted,
            &[(
                actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT,
                payload.clone(),
            )],
        )
        .await?;
        assert_eq!(
            actor_spell::decode_spell_cast_result(&refused[0].1).map_err(debug)?,
            actor_spell::SpellCastDisposition::Rejected
        );
        let runtime = deps.runtime.lock().await;
        for &target in &targets {
            assert_eq!(
                runtime.live_creature_health(target),
                Some(rat_health),
                "the refused cast must not damage either creature"
            );
        }
        drop(runtime);
        let unchanged = owner
            .observe_vitals(actor, session)
            .await
            .ok_or("refused vitals")?
            .1
            .mana;
        assert_eq!(unchanged, before, "the refused cast must not spend mana");
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[6, 7, 17, 23, actor_spell::CAPABILITY_SPELL_AIM_MODE_V1],
        )
        .ok_or("aim mode capabilities")?;
        assert!(
            admitted
                .continuity
                .selected_capabilities
                .contains(actor_spell::CAPABILITY_SPELL_AIM_MODE_V1),
            "capability 480 selected"
        );
    }
    let viewer_session = crate::foundation::GameSessionId::decode(&id(120)).map_err(debug)?;
    let viewer = {
        let mut runtime = deps.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(viewer_session).map_err(debug)?;
        let spectator = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime.initialize_first_entry_position(spectator).map_err(debug)?;
        spectator
    };
    let (cursor, _) = owner.observe_sounds(viewer, viewer_session, u64::MAX)
        .await.ok_or("spectator presentation cursor")?;
    let observer = AdmittedSession {
        game_session_id: viewer_session,
        runtime_actor: Some(viewer),
        controller: None,
        item_fence: None,
        continuity: crate::gameplay_transport::connection::SessionContinuity {
            selected_capabilities: SelectedCapabilities::select(owner.offered_capabilities(), &[6, 23])
                .ok_or("spectator presentation capabilities")?,
            ..crate::gameplay_transport::connection::SessionContinuity::FRESH
        },
        ..admitted
    };
    let commands = [(actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT, payload)];
    let (cast, spectator_frames) =
        crate::gameplay_transport::connection::observe_test_spell_damage(
            &owner, observer, async {
                // The spectator has its initial cut, and no refresh runs between the move
                // and the hit. The caster joins after the move and sees the impact tile.
                let mut runtime = deps.runtime.lock().await;
                let before = runtime.read_actor_position(moving_rat).map_err(debug)?;
                let destination = crate::foundation::MovementLocalPosition {
                    y: caster.y,
                    ..before.position()
                };
                runtime
                    .borrow_movement_position()
                    .commit_cardinal(before, destination)
                    .map_err(debug)?;
                drop(runtime);
                exchange(&owner, &mut admitted, &commands).await
            }
        ).await?;
    let (results, caster_frames) = cast?;
    assert!(admitted.continuity.combat.sent.is_some_and(|state| state.in_fight),
        "a committed offensive spell must immediately project the battle sign");
    assert_eq!(
        actor_spell::decode_spell_cast_result(&results[0].1).map_err(debug)?,
        actor_spell::SpellCastDisposition::Cast
    );
    let (_, events) = owner.observe_sounds(viewer, viewer_session, cursor)
        .await.ok_or("spectator presentation")?;
    use oteryn_protocol_oteryn::presentation::{
        PresentationColor, PresentationEventKind as Kind, PresentationSource, PresentationValueKind,
    };
    let decision = events.iter().find_map(|event| match event.kind {
        Kind::SpellWords { caster: source, spell: index, .. }
            if source.identity == actor.placement_identity() && index == spell.get() => Some(event.decision_ordinal),
        _ => None,
    }).ok_or("spectator sees Scorch words")?;
    let mut cells = events.iter().filter_map(|event| match event.kind {
        Kind::MagicEffect { position, source: PresentationSource::Others, .. }
            if event.decision_ordinal == decision => Some((position.x, position.y, position.floor)),
        _ => None,
    }).collect::<Vec<_>>();
    // Thalom's northeast wall and solid tiles on the far edge clip this east-facing cone.
    let expected = [(1, 0), (2, 0), (2, 1), (3, 0), (4, 1)]
        .map(|(dx, dy)| (caster.x + dx, caster.y + dy, caster.floor.abs()));
    cells.sort_unstable();
    assert_eq!(cells, expected, "resolved Scorch cone");
    let runtime = deps.runtime.lock().await;
    for target in targets {
        let hit = runtime.live_creature_health(target).ok_or("rat survives")?;
        let damage = rat_health - hit;
        assert!(damage > 0);
        let target_ref = oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: target.placement_identity(),
            generation: target.actor_local_generation(),
        };
        let percent = u8::try_from((hit * 100 / rat_health).max(1))?;
        assert!(percent < 100);
        assert_damage_then_health(&caster_frames, target_ref, damage, percent)?;
        assert_damage_then_health(&spectator_frames, target_ref, damage, percent)?;
        assert_eq!(events.iter().filter(|event| event.decision_ordinal == decision && matches!(
            event.kind,
            Kind::ValueText { kind: PresentationValueKind::DamageOthers, target: shown,
                attacker: Some(attacker), primary, .. }
                if shown.identity == target.placement_identity()
                    && shown.generation == target.actor_local_generation()
                    && attacker.identity == actor.placement_identity()
                    && attacker.generation == actor.actor_local_generation()
                    && primary.value == damage && primary.color == PresentationColor::Orange
        )).count(), 1, "spectator sees the rat's fire damage once in Scorch's decision: {events:?}");
    }
    drop(runtime);
    let after = owner
        .observe_vitals(actor, session)
        .await
        .ok_or("after vitals")?
        .1
        .mana;
    assert_eq!(i64::from(before) - i64::from(after), MANA, "mana");
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

type CastExchange = (Vec<(CommandStatus, Vec<u8>)>, Vec<u8>);

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    commands: &[(u32, Vec<u8>)],
) -> TestResult<CastExchange> {
    let session = *admitted;
    let first_command_id = admitted.continuity.next_command_id;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async move { serve_admitted(&mut server, session, owner, IDLE_LIVENESS).await };
    let driving = async move {
        loop {
            let frame = super::item_look_fixtures::read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        let mut results = Vec::new();
        for (offset, (kind, payload)) in commands.iter().enumerate() {
            let frame = encode_client_command(
                session.continuity.connection_generation,
                &ClientCommandValue {
                    command_id: first_command_id + offset as u64,
                    command_type: *kind,
                    payload,
                },
            )
            .map_err(debug)?;
            client.write_u32(u32::try_from(frame.len())?).await?;
            client.write_all(&frame).await?;
            loop {
                let frame = super::item_look_fixtures::read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    results.push((result.status, result.payload.to_vec()));
                    break;
                }
            }
        }
        client.shutdown().await?;
        let mut frames = Vec::new();
        client.read_to_end(&mut frames).await?;
        Ok((results, frames))
    };
    let (end, results): (_, TestResult<_>) =
        tokio::time::timeout(Duration::from_secs(45), join_two(serving, driving)).await?;
    match end {
        crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(current) => {
            *admitted = current
        }
        other => return Err(format!("spell aim mode serving ended: {other:?}").into()),
    }
    results
}

fn assert_damage_then_health(
    mut frames: &[u8],
    target: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    damage: u64,
    percent: u8,
) -> TestResult {
    use oteryn_protocol_oteryn::{decode_state_delta, presentation as cues, world_spatial_entities as spatial};
    let mut impact = None;
    while !frames.is_empty() {
        let length = u32::from_be_bytes(frames.get(..4).ok_or("frame length")?.try_into()?) as usize;
        let envelope = decode_wire_envelope(frames.get(4..4 + length).ok_or("frame")?)?;
        frames = &frames[4 + length..];
        if envelope.message_type() != MessageType::StateDelta {
            continue;
        }
        let delta = decode_state_delta(envelope.payload())?;
        if delta.domain_id == cues::STATE_DOMAIN_WORLD_PRESENTATION {
            for event in cues::decode_presentation_batch(delta.payload).map_err(debug)? {
                if let cues::PresentationEventKind::ValueText { target: shown, position, primary, .. } = event.kind
                    && shown == target && primary.value == damage
                {
                    impact = Some(position);
                }
            }
        } else if delta.domain_id == oteryn_protocol_oteryn::world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY {
            let update = spatial::decode_world_spatial_entities_delta(delta.payload).map_err(debug)?;
            for entry in update.update {
                if entry.entity == target && matches!(entry.detail, spatial::EntityDetail::Actor { health_percent, .. } if health_percent == percent) {
                    let impact = impact.ok_or("the creature health must follow its damage text")?;
                    assert_eq!(
                        entry.position, impact,
                        "the damaged generation must be at its impact tile"
                    );
                    return Ok(());
                }
            }
        }
    }
    Err(format!("missing creature health {percent} after damage {damage}, impact={impact:?}").into())
}
