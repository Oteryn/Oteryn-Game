//! WU26-14 (Winter Update 2026 test server): a wave cast aimed at the cursor tile through the
//! wire command, the cast route and the ordinary combat preparation, with real Character owners.
//! The old `aim_at_target` flag with a POSITION is the CURSOR option.
use super::death_ml_loss_composed::{current_fence, production_generation_with_source};
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
use crate::gameplay_transport::capabilities::SelectedCapabilities;
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
use tokio::io::AsyncWriteExt;

/// The rat's health before the cast: far above one hit, so the health lost is the damage.
const RAT_HEALTH: i64 = 10_000;
/// Scorch: a level-1 sorcerer wave whose matrix centre is the tile next to the caster.
const SPELL: &str = "candidate:spell/scorch";
const MANA: i64 = 8;

#[test]
fn spell_aim_mode_postgres_cursor_wave_hits_the_cursor_direction_from_the_caster() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(move || {
            run(async move |admin| scenario(admin).await).map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "spell aim mode thread panicked")?
        .map_err(Into::into)
}

async fn scenario(admin: String) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "spell_aim_mode", false, false).await?;
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
        3,
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
    h.root
        .commit_character_experience(
            &authority,
            &h.node,
            current_fence(&owner, session).await?,
            ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(id(91)).map_err(debug)?,
                amount: oteryn_simulation_determinism::ExactI64::new(6_000_000),
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
    let target;
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
        let health = runtime
            .companion_policy("oteryn:creature.rat")
            .map_err(debug)?
            .maximum_health;
        // The rat stands on the tile east of the caster, the aimed wave's origin.
        runtime
            .realize_activation_spawns(vec![(
                vec![(caster.x + 1, caster.y, caster.floor)],
                60_000,
                5_000,
                "oteryn:creature.rat".into(),
                u64::try_from(health)?,
            )])
            .map_err(|error| format!("realize aim target spawn: {error:?}"))?;
        target = runtime
            .visible_entities()
            .creatures
            .first()
            .ok_or("rat")?
            .actor;
        runtime
            .set_test_creature_health(target, RAT_HEALTH)
            .map_err(debug)?;
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
        SelectedCapabilities::select(owner.offered_capabilities(), &[6, 7])
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
    let payload = actor_spell::encode_spell_cast_intent(&actor_spell::SpellCastIntent {
        spell,
        target: actor_spell::SpellTarget::Position(actor_spell::SpellTargetPosition {
            x: caster.x + 5,
            y: caster.y,
            floor: caster.floor,
        }),
        aim_at_target: true,
    });
    let results = exchange(
        &owner,
        &mut admitted,
        &[(
            actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT,
            payload,
        )],
    )
    .await?;
    assert_eq!(
        actor_spell::decode_spell_cast_result(&results[0].1).map_err(debug)?,
        actor_spell::SpellCastDisposition::Cast
    );
    let hit = deps
        .runtime
        .lock()
        .await
        .live_creature_health(target)
        .ok_or("rat survives")?;
    assert!(
        hit < u64::try_from(RAT_HEALTH)?,
        "the aimed wave must damage the creature next to the caster: {hit}"
    );
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

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    commands: &[(u32, Vec<u8>)],
) -> TestResult<Vec<(CommandStatus, Vec<u8>)>> {
    let session = *admitted;
    let first_command_id = admitted.continuity.next_command_id;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async move { serve_admitted(&mut server, session, owner, IDLE_LIVENESS).await };
    let driving = async move {
        loop {
            let frame = super::item_look::read_frame(&mut client).await?;
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
                let frame = super::item_look::read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    results.push((result.status, result.payload.to_vec()));
                    break;
                }
            }
        }
        client.shutdown().await?;
        Ok(results)
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
