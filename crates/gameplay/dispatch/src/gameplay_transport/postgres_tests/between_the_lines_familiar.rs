//! Between the Lines gates the production familiar cast before its resource costs.
use super::death_ml_loss_composed::{current_fence, production_generation_with_source};
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};
use crate::durability::character_progression::{
    ExperienceAwardRequest, ExperienceRewardOccurrence,
};
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{ChannelContentPin, CharacterLease, CommandId, CommandRef, WorldId};
use crate::gameplay_transport::actor_spell::{NativeCastDispatch, SpellCastOutcome};
use crate::gameplay_transport::spell_character_facts::{CastFactsLoad, load_character_cast_facts};
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellCastIntent, SpellTarget};
use oteryn_protocol_oteryn::chat::ChatLine;
use std::{num::NonZeroU32, sync::Arc};

#[test]
fn between_the_lines_captured_familiar_refuses_without_cost_and_casts_when_freed() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?
                .block_on(scenario(admin))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "familiar rescue scenario panicked")?
        .map_err(Into::into)
}

async fn scenario(admin: String) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "familiar_rescue", false, false).await?;
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
    let mut deps =
        Dependencies::with_content_and_capacity(cp, room.clone(), native.spell_book().clone(), 3)?;
    deps.combat_room = Some(room);
    // Familiar casts require a current Premium account.
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
    owner
        .states
        .chat
        .admit(session, "Familiar rescuer", owner.owner_now());
    owner.admit_premium(id(40), session);
    owner.admit_progression_session(&mut admitted).await;
    let progression = owner.progression_binding(session).ok_or("progression")?;
    let formula = native.training_formula().ok_or("training")?;
    let build = convert_vocation(formula, &DurableBuildState::default(), "druid").map_err(debug)?;
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
                amount: oteryn_simulation_determinism::ExactI64::new(150_000_000),
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
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
        .bind(id(41).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;

    assert!(owner.ensure_source_map_initialized().await);
    let spell = (1..=deps.spells.source_len())
        .find_map(|index| {
            let index = NonZeroU32::new(u32::try_from(index).ok()?)?;
            let (definition, active) = deps.spells.source_indexed(index)?;
            (active && definition.key == "candidate:spell/druid_familiar").then_some(index)
        })
        .ok_or("Summon Druid Familiar spell index")?;
    let intent = SpellCastIntent {
        spell,
        target: SpellTarget::None,
        aim_at_target: false,
    };
    let authored = oteryn_gameplay_quests::quest::loader::parse_quest_state(
        include_str!("../../../../../../content/quests/missions/between-the-lines-familiar.json"),
        "content-1",
    )
    .map_err(debug)?;
    let catalogue = Arc::new(authored.catalogue().clone());
    let sequencer = CharacterRevisionSequencer::new();
    for (stage, transition_id, cast_id) in [("captured", 11, 12), ("freed", 13, 14)] {
        let fence = current_fence(&owner, session).await?;
        let transition = sequencer
            .acquire(fence.character_id)
            .await
            .commit_quest_transition(
                &h.root,
                &authority,
                &h.node,
                fence,
                QuestTransitionRequest {
                    transition_key: format!(
                        "oteryn:quest-transition/between-the-lines/familiar-{stage}"
                    ),
                    cause: QuestCause::Command(CommandRef::new(
                        session,
                        CommandId::new(transition_id).map_err(debug)?,
                    )),
                },
                catalogue.clone(),
            )
            .await
            .map_err(debug)?;
        assert!(
            matches!(transition, QuestTransitionOutcome::Committed(_)),
            "{transition:?}"
        );
        let access = owner.refresh_and_apply_spell_access(actor, session).await;
        let before = {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            states
                .get(&runtime, actor, session)
                .ok_or("caster")?
                .clone()
        };
        let result = owner
            .cast_native_familiar(actor, session, cast_id, &intent, &*access)
            .await;
        let feedback = owner.states.chat.drain(session);
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let after = states.get(&runtime, actor, session).ok_or("caster")?;
        let summons = runtime.owned_companions(actor, session).map_err(debug)?;
        if stage == "captured" {
            assert!(
                matches!(result, NativeCastDispatch::Outcome(outcome) if outcome == SpellCastOutcome::rejected()),
                "{result:?}"
            );
            assert_eq!(after.vitals(), before.vitals());
            assert_eq!(
                after.source_cooldown_deadlines(),
                before.source_cooldown_deadlines()
            );
            assert!(summons.is_empty());
            assert_eq!(
                feedback,
                [ChatLine::Private {
                    speaker_name: "Game".into(),
                    text: "Sorry, not possible.".into(),
                }]
            );
        } else {
            assert!(
                matches!(result, NativeCastDispatch::Outcome(outcome) if outcome.disposition == SpellCastDisposition::Cast),
                "{result:?}"
            );
            assert!(after.vitals().mana < before.vitals().mana);
            assert_ne!(
                after.source_cooldown_deadlines(),
                before.source_cooldown_deadlines()
            );
            assert_eq!(summons.len(), 1);
            assert!(feedback.is_empty());
        }
    }
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}
