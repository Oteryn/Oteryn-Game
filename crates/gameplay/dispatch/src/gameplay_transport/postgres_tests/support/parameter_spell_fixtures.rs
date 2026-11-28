#![allow(clippy::expect_used)]

use super::appearance_change_postgres_fixtures::observer_visibility_tests as observer;
use tokio::io::AsyncWriteExt;

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
    ChannelContentPin, CharacterLease, CommandId, CommandRef, GameSessionId, MessageType,
    decode_wire_envelope,
};

use crate::gameplay_transport::capabilities::SelectedCapabilities;

use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};

use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};

use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, actor_spell, chat, decode_command_result,
    encode_client_command,
};

use std::time::Duration;

pub(super) fn scenario(case: u8) -> TestResult {
    scenario_in_world(case, None)
}

pub(super) fn scenario_in_world(case: u8, world_type: Option<i16>) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(move || {
            run(async move |admin| scenario_inner(admin, case, world_type).await)
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "parameter spell thread panicked")?
        .map_err(Into::into)
}

const LOGOUT_WITH_COOLDOWN: u8 = 254;

pub(super) fn logout_with_cooldown(admin: String) -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("spell-cooldown-logout", move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(scenario_inner(admin, LOGOUT_WITH_COOLDOWN, None))
    })
}

async fn scenario_inner(admin: String, case: u8, world_type: Option<i16>) -> TestResult {
    let h = Harness::create_with_admission_seed(
        admin,
        &format!("parameter_spell_{case}"),
        false,
        false,
    )
    .await?;
    eprintln!(
        "PARAM-CAST-WIRE-1 PostgreSQL harness created, case={case}, world_type={world_type:?}"
    );
    let recovery = if matches!(case, 104 | 106) {
        let source = super::house_transfer_fixtures::RecoverySource::start(&h).await?;
        super::house_transfer_fixtures::prepare_owned_session(&h).await?;
        Some(source)
    } else {
        prepare_current_session(&h).await?;
        None
    };
    h.seed_pvp_target().await?;
    let cooldown_cast_replay_case = matches!(case, COOLDOWN_CASE | COOLDOWN_WITHOUT_PRESENTATION_CASE);
    let has_observer = !matches!(case, 1 | 5 | 6 | LOGOUT_WITH_COOLDOWN) && !cooldown_cast_replay_case;
    if has_observer {
        observer::admit(&h).await?;
    }
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) = if matches!(case, 4..=6) {
        super::item_spell_augments_fixtures::activate(world, case == 6)?
    } else {
        production_generation_with_source(
            world,
            "content-1",
            Some(include_bytes!(
                "../../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
            )),
        )
        .map_err(debug)?
    };
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
        if cooldown_cast_replay_case {
            (12, 14, -7) // Walkable, ground-only protection-zone tile in the pinned Thalom map.
        } else {
            (2, 11, -7)
        },
    );
    let mut deps = Dependencies::with_content_and_capacity(
        cp,
        room.clone(),
        native.spell_book().clone(),
        if matches!(case, 5 | 6) { 4 } else { 2 },
    )?;
    deps.combat_room = Some(room);
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(active);
    if let Some(recovery) = &recovery {
        owner.evidence = &recovery.evidence;
    }
    let mut admitted = deps.admitted;
    let actor = deps.actor;
    let session = admitted.game_session_id;
    owner.admit_progression_session(&mut admitted).await;
    let progression = owner.progression_binding(session).ok_or("progression")?;
    let formula = native.training_formula().ok_or("training")?;
    let druid = convert_vocation(formula, &DurableBuildState::default(), "druid").map_err(debug)?;
    h.root
        .commit_character_build(
            &authority,
            &h.node,
            current_fence(&owner, session).await?,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(90)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                before: DurableBuildState::default(),
                after: druid.clone(),
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
                amount: oteryn_simulation_determinism::ExactI64::new(98_800),
                context: progression.context.clone(),
                policy_revision: progression.policy_revision.clone(),
                reward_revision: progression.reward_revision.clone(),
                policy: progression.policy.clone(),
            },
        )
        .await
        .map_err(debug)?;
    let mut augment_target = None;
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
        if matches!(case, 5 | 6) {
            // This equipment-projection case remains nonlethal, so actual HP
            // delta equals the admitted damage. Separate owner regressions
            // require uncapped overkill Leech.
            let p = runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position();
            let position = crate::foundation::MovementLocalPosition {
                x: p.x + 1,
                y: p.y,
                floor: p.floor,
            };
            runtime
                .install_companion_policies(native.policies().clone())
                .map_err(debug)?;
            let health = runtime
                .companion_policy("oteryn:creature.demon")
                .map_err(debug)?
                .maximum_health;
            runtime
                .realize_activation_spawns(vec![(
                    vec![(position.x, position.y, position.floor)],
                    60_000,
                    5_000,
                    "oteryn:creature.demon".into(),
                    u64::try_from(health)?,
                )])
                .map_err(debug)?;
            let target = runtime
                .visible_entities()
                .creatures
                .first()
                .ok_or("nonlethal leech target")?
                .actor;
            augment_target = Some((target, position));
        }
        let mut states = owner.states.spell_states.lock().await;
        assert!(
            states
                .initialize(
                    &runtime,
                    actor,
                    session,
                    crate::spell::cast::CharacterCastFacts {
                        level: 20,
                        ..crate::gameplay_transport::actor_spell::tests::FACTS
                    },
                    (0, 0),
                    owner.owner_now()
                )
                .is_some()
        );
        assert!(states.load_owned_build(&runtime, actor, session, &druid));
        states
            .get_mut(&runtime, actor, session)
            .ok_or("caster")?
            .enable_owned_training(&druid, formula)
            .map_err(debug)?;
    }
    let target_session = GameSessionId::decode(&id(110)).map_err(debug)?;
    let target_actor = if has_observer {
        Some(observer::initialize(&deps, &owner).await?)
    } else {
        None
    };
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1),(encode($2,'hex')::uuid,1)").bind(id(41).as_slice()).bind(id(101).as_slice()).execute(&h.pool).await?;
    if matches!(case, 4..=6) {
        let mut tx = h.pool.begin().await?;
        // Fixture custody only; tested equipment reads use the production fences.
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($1,'hex')::uuid)")
            .bind(id(112).as_slice()).bind(id(42).as_slice()).bind(if case == 6 { "oteryn:item.tibia.i6529" } else { "oteryn:item.tibia.i2854" }).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$4,encode($3,'hex')::uuid,encode($3,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(112).as_slice()).bind(if case == 6 { 4_i16 } else { 3 }).execute(&mut *tx).await?;
        tx.commit().await?;
        let mut states = owner.states.spell_states.lock().await;
        crate::gameplay_transport::actor_spell::tests::wound(&mut states, actor, session, 1);
    }
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
    for current_session in [
        Some(session),
        has_observer.then_some(target_session),
    ]
    .into_iter()
    .flatten()
    {
        h.root
            .initialize_familiar_group(
                &authority,
                &h.node,
                current_fence(&owner, current_session).await?,
            )
            .await
            .map_err(debug)?;
    }
    if matches!(case, 0 | 3 | 255) {
        allow_location(&owner, target_session, native.source_digest(), case != 3).await?;
    }
    let current =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(session)
            .await?
            .0;
    admitted.item_fence = item_fence_of(current);
    if case == 6 {
        // The fixture seeds custody after admission; install its derived abilities through
        // the same fenced reader and installer as a real equipment-bearing admission.
        let _permit = owner.spell_lane_permit().await.ok_or("spell lane")?;
        let effects = owner
            .read_admission_effects(session)
            .await
            .map_err(|_| "equipment effects")?;
        let runtime = deps.runtime.lock().await;
        owner
            .install_admission_suppressions(&runtime, actor, session, effects)
            .await
            .ok_or("install equipment effects")?;
    }
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[6, 7])
            .ok_or("chat capabilities")?;
    if case == 255 {
        use oteryn_protocol_oteryn::actor_spell_v2 as wire;
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 608])
                .ok_or("parameter cast capability")?;
        assert!(admitted.continuity.selected_capabilities.contains(608));
        let spell = (1..=deps.spells.source_len())
            .filter_map(|i| u32::try_from(i).ok().and_then(std::num::NonZeroU32::new))
            .find(|i| {
                deps.spells
                    .indexed(*i)
                    .is_some_and(|s| s.name == "Find Person")
            })
            .ok_or("find person index")?;
        let payload = wire::encode_parameter_spell_cast_intent(&wire::ParameterSpellCastIntent {
            intent: actor_spell::SpellCastIntent {
                spell,
                target: actor_spell::SpellTarget::None,
                aim_at_target: false,
            },
            parameter: Some("Fixture Watcher".into()),
        })
        .map_err(debug)?;
        let before = owner
            .observe_vitals(actor, session)
            .await
            .ok_or("before vitals")?
            .1
            .mana;
        let requests = [(6380, payload.clone()), (6380, payload)];
        let (results, unsolicited) =
            exchange(&owner, &mut admitted, &requests, true, false).await?;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].1, results[1].1);
        assert_eq!(results[0].0, CommandStatus::Accepted);
        assert_eq!(results[1].0, CommandStatus::DuplicateReplay);
        let result = wire::decode_parameter_spell_cast_result(&results[0].1).map_err(debug)?;
        assert_eq!(result.disposition, actor_spell::SpellCastDisposition::Cast);
        assert_eq!(
            result.feedback.ok_or("private feedback")?.text,
            "Fixture Watcher is standing next to you."
        );
        assert!(unsolicited.is_empty());
        assert_eq!(
            owner
                .observe_vitals(actor, session)
                .await
                .ok_or("after vitals")?
                .1
                .mana,
            before - 20
        );
        drop(owner);
        drop(authority);
        drop(seal);
        return h.cleanup().await;
    }
    if cooldown_cast_replay_case {
        cooldown_cast_replay::check(&owner, admitted, native.spell_book(), case == COOLDOWN_CASE).await?;
        drop(owner);
        drop(authority);
        drop(seal);
        return h.cleanup().await;
    }
    if (101..=108).contains(&case) {
        cooldowns::exercise(&owner, admitted, native.spell_book(), case).await?;
        drop(owner);
        drop(authority);
        drop(seal);
        return h.cleanup().await;
    }
    if case == LOGOUT_WITH_COOLDOWN {
        use oteryn_protocol_oteryn::logout::CAPABILITY_LOGOUT_V1;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[6, 7, CAPABILITY_LOGOUT_V1],
        )
        .ok_or("spell logout capabilities")?;
        assert!(admitted.continuity.selected_capabilities.contains(CAPABILITY_LOGOUT_V1));
        cast_and_logout(&owner, &h, admitted).await?;
        drop(owner);
        drop(authority);
        drop(seal);
        return h.cleanup().await;
    }
    if case >= 7 {
        return exiva_settings::exercise(
            &h,
            &owner,
            admitted,
            target_actor.ok_or("target actor")?,
            case,
            world_type,
        )
        .await;
    }
    let before = owner
        .observe_vitals(actor, session)
        .await
        .ok_or("before vitals")?
        .1
        .mana;
    let words = if matches!(case, 5 | 6) {
        "exori vis"
    } else if matches!(case, 2 | 4) {
        "exura"
    } else {
        "  EXIVA prefix\"Fixture Watcher\"  "
    };
    let payload = chat::encode_chat_intent(&chat::ChatIntent::Say {
        mode: chat::ChatSpeechMode::Say,
        text: words.into(),
    })
    .map_err(debug)?;
    let mut commands = vec![
        (chat::COMMAND_TYPE_CHAT_INTENT, payload.clone()),
        (chat::COMMAND_TYPE_CHAT_INTENT, payload),
    ];
    if let Some((_, position)) = augment_target {
        let index = (1..=deps.spells.source_len())
            .filter_map(|i| u32::try_from(i).ok().and_then(std::num::NonZeroU32::new))
            .find(|i| {
                deps.spells
                    .indexed(*i)
                    .is_some_and(|s| s.name == "Energy Strike")
            })
            .ok_or("energy strike")?;
        let payload = actor_spell::encode_spell_cast_intent(&actor_spell::SpellCastIntent {
            spell: index,
            target: actor_spell::SpellTarget::Position(actor_spell::SpellTargetPosition {
                x: position.x,
                y: position.y,
                floor: position.floor.abs(),
            }),
            aim_at_target: false,
        });
        commands = vec![
            (
                actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT,
                payload.clone(),
            ),
            (
                actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT,
                payload,
            ),
        ];
    }
    let (results, feedback) = exchange(
        &owner,
        &mut admitted,
        &commands,
        true,
        !matches!(case, 2 | 4 | 5 | 6),
    )
    .await?;
    assert_eq!(results[1].0, CommandStatus::DuplicateReplay);
    if let Some((target, _)) = augment_target {
        assert_eq!(
            actor_spell::decode_spell_cast_result(&results[0].1).map_err(debug)?,
            actor_spell::SpellCastDisposition::Cast
        );
        let (_, vitals) = owner
            .observe_vitals(actor, session)
            .await
            .ok_or("leech vitals")?;
        let runtime = deps.runtime.lock().await;
        let hp = runtime
            .creature_combat_facts(target, owner.owner_now().get() / 1000)
            .map_err(debug)?;
        assert!(
            hp.health > 0,
            "equipment projection fixture must be nonlethal"
        );
        let real = hp.maximum_health - hp.health;
        assert!(real > 0);
        assert_eq!(
            i64::from(vitals.health),
            (1 + real).min(i64::from(vitals.max_health))
        );
        assert_eq!(
            i64::from(vitals.mana),
            (i64::from(before) - 20 + real).min(i64::from(vitals.max_mana))
        );
        drop(runtime);
        if case == 6 {
            // Mana training checkpoints below its advance threshold; flush through the
            // production save owner, then prove the duplicate did not train twice.
            assert_eq!(
                owner.save_spell_training(actor, session, true).await,
                crate::gameplay_transport::actor_spell::TrainingSave::Saved
            );
            let durable = h
                .root
                .read_character_build_state(
                    &authority,
                    crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?,
                )
                .await
                .map_err(debug)?;
            assert_eq!(durable.magic(), (0, 20));
        }
        return Ok(());
    }
    let result = chat::decode_chat_intent_result(&results[0].1).map_err(debug)?;
    assert_eq!(
        result.disposition,
        if case == 1 || case == 3 {
            chat::ChatDisposition::Rejected
        } else {
            chat::ChatDisposition::Ok
        },
        "{results:?}"
    );
    let after = owner
        .observe_vitals(actor, session)
        .await
        .ok_or("after vitals")?
        .1
        .mana;
    assert_eq!(before - after, if case == 1 || case == 3 { 0 } else { 20 });
    if !matches!(case, 2 | 4) {
        let expected = if case == 1 {
            "A player with this name is not online."
        } else if case == 3 {
            "The character you are trying to find with Exiva is currently protected from your spell."
        } else {
            "Fixture Watcher is standing next to you."
        };
        assert!(feedback.iter().any(|line| matches!(line, chat::ChatLine::Private { speaker_name, text } if speaker_name == "Game" && text == expected)), "{feedback:?}");
        assert!(owner.states.chat.drain(target_session).is_empty());
    }
    if case == 4 {
        let (_, vitals) = owner.observe_vitals(actor, session).await.ok_or("healed")?;
        assert!(vitals.health >= 100, "augmented healing {vitals:?}");
        let runtime = deps.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let state = states.get(&runtime, actor, session).ok_or("state")?;
        let spell = deps
            .spells
            .spells
            .iter()
            .find(|s| s.name == "Light Healing")
            .ok_or("spell")?;
        let ready = state
            .cooldowns
            .spell_ready_at(&spell.key)
            .ok_or("cooldown")?
            .get();
        assert!(ready <= owner.owner_now().get() + spell.cooldown_micros / 2);
        drop(states);
        drop(runtime);
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM game_character_equipment_slots WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(112).as_slice()).execute(&mut *tx).await?;
        sqlx::query("UPDATE game_character_equipment_state SET revision=revision+1 WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let mut states = owner.states.spell_states.lock().await;
        crate::gameplay_transport::actor_spell::tests::wound(&mut states, actor, session, 1);
    }
    // A second input after the source cooldown uses a new command, not another cast path.
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(5))
        .ok_or("clock advance")?;
    if case == 1 {
        commands = vec![(
            chat::COMMAND_TYPE_CHAT_INTENT,
            chat::encode_chat_intent(&chat::ChatIntent::Say {
                mode: chat::ChatSpeechMode::Say,
                text: "exiva Nobody".into(),
            })
            .map_err(debug)?,
        )];
    } else if matches!(case, 2 | 4) {
        let index = (1..=deps.spells.source_len())
            .filter_map(|i| u32::try_from(i).ok().and_then(std::num::NonZeroU32::new))
            .find(|i| {
                deps.spells
                    .indexed(*i)
                    .is_some_and(|s| s.name == "Light Healing")
            })
            .ok_or("exura index")?;
        commands = vec![(
            actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT,
            actor_spell::encode_spell_cast_intent(&actor_spell::SpellCastIntent {
                spell: index,
                target: actor_spell::SpellTarget::None,
                aim_at_target: false,
            }),
        )];
    } else {
        return Ok(());
    }
    assert_eq!(admitted.continuity.next_command_id, 2);
    let (next, _) = exchange(&owner, &mut admitted, &commands, false, case == 1).await?;
    if case == 1 {
        assert_eq!(
            chat::decode_chat_intent_result(&next[0].1)
                .map_err(debug)?
                .disposition,
            chat::ChatDisposition::Rejected
        );
        assert_eq!(
            owner
                .observe_vitals(actor, session)
                .await
                .ok_or("offline mana")?
                .1
                .mana,
            after
        );
    } else {
        assert_eq!(
            actor_spell::decode_spell_cast_result(&next[0].1).map_err(debug)?,
            actor_spell::SpellCastDisposition::Cast
        );
        assert_eq!(
            after
                - owner
                    .observe_vitals(actor, session)
                    .await
                    .ok_or("ordinary mana")?
                    .1
                    .mana,
            20
        );
    }
    if case == 4 {
        let (_, vitals) = owner
            .observe_vitals(actor, session)
            .await
            .ok_or("unequipped")?;
        assert!(vitals.health < 100, "unequipped healing {vitals:?}");
    }
    Ok(())
}

async fn allow_location(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    digest: [u8; 32],
    allowed: bool,
) -> TestResult {
    use crate::durability::spell_item_transaction as items;
    let fence = current_fence(owner, session).await?.item_fence();
    let permit = owner.spell_lane_permit().await.ok_or("spell lane")?;
    let command = CommandRef::new(session, CommandId::new(1).map_err(debug)?);
    let mut context = (owner, permit);
    owner
        .root
        .try_issue_semantic_pass()?
        .run_with_context(&mut context, move |holder, deadline, (owner, permit)| {
            Box::pin(async move {
                let mut tx = items::begin_spell_owner_transaction(holder, deadline).await?;
                let authority = items::assert_spell_item_authority_in_transaction(
                    &mut tx,
                    permit,
                    owner.root,
                    owner.character,
                    owner.holder,
                    &fence,
                    command,
                    digest,
                )
                .await
                .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                let mut preferences =
                    crate::durability::spell_privacy::ExivaPreferences::source_initial_preferences(
                    );
                preferences.allow_all = allowed;
                preferences.allow_own_guild = false;
                preferences.allow_own_party = false;
                preferences.allow_vip_list = false;
                crate::durability::spell_privacy::write_exiva_preferences_in_transaction(
                    &mut tx,
                    &authority,
                    None,
                    &preferences,
                )
                .await
                .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                tx.commit().await?;
                Ok(())
            })
        })
        .await?;
    Ok(())
}

async fn cast_and_logout(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: AdmittedSession,
) -> TestResult {
    use super::autowalk_fixtures::{command_reply, read, send};
    use crate::durability::fresh_admission::FreshAdmissionStore;
    use crate::foundation::GameSessionState;
    use crate::gameplay_transport::composed_admission::{
        CONDITION_RELEASE_PROBE, HolderCheckoutProbe,
    };
    use crate::gameplay_transport::connection::ConnectionEnd;
    use oteryn_protocol_oteryn::logout::{
        COMMAND_TYPE_LOGOUT_INTENT, LogoutResult, decode_logout_result,
    };
    use std::{cell::Cell, sync::Arc};

    let actor = admitted.runtime_actor.ok_or("caster")?;
    let session = admitted.game_session_id;
    let (index, spell) = (1..=owner.spells.source_len())
        .filter_map(|i| u32::try_from(i).ok().and_then(std::num::NonZeroU32::new))
        .filter_map(|i| owner.spells.indexed(i).map(|spell| (i, spell)))
        .find(|(_, spell)| spell.name == "Light Healing")
        .ok_or("Light Healing")?;
    let cast = actor_spell::encode_spell_cast_intent(&actor_spell::SpellCastIntent {
        spell: index,
        target: actor_spell::SpellTarget::None,
        aim_at_target: false,
    });

    let probe = Arc::new(HolderCheckoutProbe::default());
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let drive = async {
        loop {
            let frame = read(&mut client).await?;
            if decode_wire_envelope(&frame)?.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        let command_id = admitted.continuity.next_command_id;
        send(
            &mut client,
            command_id,
            actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT,
            &cast,
        )
        .await?;
        let (status, payload) = command_reply(&mut client, command_id).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            actor_spell::decode_spell_cast_result(&payload).map_err(debug)?,
            actor_spell::SpellCastDisposition::Cast,
        );
        {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let state = states.get(&runtime, actor, session).ok_or("live caster")?;
            let ready = state
                .cooldowns
                .spell_ready_at(&spell.key)
                .ok_or("cooldown")?;
            assert!(
                ready.get() > owner.owner_now().get(),
                "cooldown must be active"
            );
        }
        let command_id = command_id + 1;
        send(&mut client, command_id, COMMAND_TYPE_LOGOUT_INTENT, &[]).await?;
        let (_, payload) = tokio::try_join!(
            async {
                probe.contend(&harness.root).await?;
                Ok::<_, Box<dyn std::error::Error>>(())
            },
            async {
                let (status, payload) = command_reply(&mut client, command_id).await?;
                assert_eq!(status, CommandStatus::Accepted);
                Ok(payload)
            },
        )?;
        client.shutdown().await?;
        assert_eq!(
            decode_logout_result(&payload).map_err(debug)?,
            LogoutResult::Accepted
        );
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (end, result) = tokio::time::timeout(
        Duration::from_secs(30),
        join_two(
            CONDITION_RELEASE_PROBE.scope(
                Cell::new(Some(Arc::clone(&probe))),
                serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS),
            ),
            drive,
        ),
    )
    .await?;
    result?;
    assert!(matches!(end, ConnectionEnd::AdmittedThenLoggedOut(_)));
    let store = FreshAdmissionStore::from_root(harness.root.clone());
    assert_eq!(
        store.current_session(session).await?.session_state(),
        GameSessionState::Terminal
    );
    Ok(())
}

type WireResults = (Vec<(CommandStatus, Vec<u8>)>, Vec<chat::ChatLine>);

pub(super) async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    commands: &[(u32, Vec<u8>)],
    replay: bool,
    feedback_expected: bool,
) -> TestResult<WireResults> {
    exchange_from(
        owner,
        admitted,
        commands,
        replay,
        feedback_expected,
        admitted.continuity.next_command_id,
    )
    .await
}

pub(super) async fn exchange_from(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    commands: &[(u32, Vec<u8>)],
    replay: bool,
    feedback_expected: bool,
    first_command_id: u64,
) -> TestResult<WireResults> {
    let session = *admitted;
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
        let mut feedback = Vec::new();
        for (offset, (kind, payload)) in commands.iter().enumerate() {
            let command_id = first_command_id + if replay { 0 } else { offset as u64 };
            let frame = encode_client_command(
                session.continuity.connection_generation,
                &ClientCommandValue {
                    command_id,
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
                if envelope.message_type() == MessageType::StateDelta {
                    let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                        .map_err(debug)?;
                    if delta.domain_id == 12 && delta.delta_type == 1 {
                        feedback.push(chat::decode_chat_line(delta.payload).map_err(debug)?);
                    }
                }
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    results.push((result.status, result.payload.to_vec()));
                    break;
                }
            }
        }
        while feedback_expected && feedback.is_empty() {
            let frame = super::item_look_fixtures::read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::StateDelta {
                let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                    .map_err(debug)?;
                if delta.domain_id == 12 && delta.delta_type == 1 {
                    feedback.push(chat::decode_chat_line(delta.payload).map_err(debug)?);
                }
            }
        }
        client.shutdown().await?;
        Ok((results, feedback))
    };
    let (end, results): (_, TestResult<_>) =
        tokio::time::timeout(Duration::from_secs(45), join_two(serving, driving)).await?;
    match end {
        crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(current) => {
            *admitted = current
        }
        other => return Err(format!("parameter serving ended: {other:?}").into()),
    }
    results
}

#[path = "parameter_spell/exiva_settings.rs"]
pub(super) mod exiva_settings;

#[path = "parameter_spell/cooldown_cast_replay.rs"]
mod cooldown_cast_replay;

#[path = "parameter_spell/cooldowns.rs"]
mod cooldowns;

pub(super) const COOLDOWN_WITHOUT_PRESENTATION_CASE: u8 = 253;
pub(super) const COOLDOWN_CASE: u8 = 254;
