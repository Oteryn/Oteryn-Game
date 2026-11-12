//! WAVE-A/M11: the parameter dispatcher consumes qualified vertical-movement plans.
//! Pinned source tiles and booted bundle tiles exercise the real parameter dispatcher.
use super::death_ml_loss_composed::{current_fence, production_generation_with_source};
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};
use crate::durability::character_progression::{
    ExperienceAwardRequest, ExperienceRewardOccurrence,
};
use crate::foundation::{ChannelContentPin, CharacterLease, MovementFacing, MovementLocalPosition};
use crate::gameplay_transport::SeamRooms;
use crate::gameplay_transport::actor_spell::native_combat_cast::NativeCastDispatch;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::gameplay_transport::spell_access_facts::UnavailableAccessOwner;
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellCastIntent, SpellTarget};
use oteryn_protocol_oteryn::actor_spell_v2::{ParameterSpellCastIntent, PrivateSpellEffect};
use serde_json::json;
use std::num::NonZeroU32;

#[test]
fn vertical_move_postgres_parameter_dispatch_levitate_and_magic_rope() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(|| {
            run(async |admin| {
                for case in 0..5 {
                    scenario(admin.clone(), case, false).await?;
                }
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "vertical movement thread panicked")?
        .map_err(Into::into)
}

fn at(x: i32, y: i32, floor: i16) -> MovementLocalPosition {
    MovementLocalPosition { x, y, floor }
}

pub(super) async fn scenario(admin: String, case: u8, use_bundle: bool) -> TestResult {
    let h =
        Harness::create_with_admission_seed(admin, &format!("vertical_move_{case}"), false, false)
            .await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) = production_generation_with_source(
        world,
        "content-1",
        Some(include_bytes!(
            "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
        )),
    )?;
    let active = controller.active().ok_or("active")?;
    let native = active.native_gameplay().ok_or("native")?;
    let bundle = use_bundle.then(|| {
        crate::map::spell_tiles::tests::vertical_movement::world(
            world,
            crate::foundation::ChannelId::decode(&id(43)).expect("fixture channel"),
        )
    });
    let bundle_room = bundle.as_ref().map(|bundle| {
        pin.qualified_room()
            .bundle_view(
                bundle
                    .movement_cells(pin.qualified_room().movement_cells())
                    .expect("bundle movement cells"),
                bundle.frame_binding_digest(world),
                bundle.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: 10,
                    y: 10,
                    floor: 6,
                },
            )
            .expect("bundle room")
    });
    let room = bundle_room.as_ref().unwrap_or(pin.qualified_room());
    let start = if use_bundle {
        match case {
            0 => at(10, 10, 6),
            1 => at(10, 9, 5),
            _ => at(10, 10, 8),
        }
    } else {
        match case {
            0 => at(1, 5, -6),
            1 => at(1, 4, -5),
            2 => at(-4, -3, -6),
            _ => at(7, 14, -8),
        }
    };
    let channel_pin = ChannelContentPin::from_activation(
        world,
        1,
        room.compiled().server_digest(),
        room.compiled().client_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let deps = Dependencies::with_content(channel_pin, room.clone(), native.spell_book().clone())?;
    let (actor, session) = (deps.actor, deps.admitted.game_session_id);
    let premium = if use_bundle {
        use crate::gameplay_transport::spell_premium_coordinator::{
            SpellPremiumCoordinator, purchase_fixture,
        };
        let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
        h.root
            .latch_premium_activation(
                &activation.activation_id,
                activation.switch_over_us,
                activation.switch_over_us + 60_000_000,
            )
            .await
            .map_err(debug)?;
        Some(SpellPremiumCoordinator::new(
            consumer,
            Some(clock),
            Some(activation),
            Some(h.root.clone()),
        ))
    } else {
        None
    };
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(active);
    owner.premium_coordinator = premium.as_ref();
    owner.movement_cells = (room.movement_cells()).into();
    // No qualified room: the same accessor path a bundle World uses.
    owner.rooms = SeamRooms::new(None, Some(room));
    let mut admitted = deps.admitted;
    owner.admit_progression_session(&mut admitted).await;
    if use_bundle {
        owner.admit_premium(id(40), session);
        owner
            .states
            .chat
            .admit(session, "Movement", owner.owner_now());
    }
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
        let before = runtime.read_actor_position(actor).map_err(debug)?;
        runtime
            .borrow_movement_position()
            .commit_turn(
                before,
                if case == 1 {
                    MovementFacing::South
                } else {
                    MovementFacing::North
                },
            )
            .map_err(debug)?;
    }
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
    let progression = owner.progression_binding(session).ok_or("progression")?;
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
    {
        let runtime = deps.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
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
                owner.owner_now(),
            )
            .ok_or("caster")?;
        assert!(states.load_owned_build(&runtime, actor, session, &build));
        states
            .get_mut(&runtime, actor, session)
            .ok_or("caster")?
            .enable_owned_training(&build, formula)
            .map_err(debug)?;
    }
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
        .bind(id(41).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice())
        .bind(room.frame_binding().digest().as_slice()).execute(&h.pool).await?;
    if case >= 3 && !use_bundle {
        seed_rope_blockers(&h, &deps, room, native, case == 4).await?;
    }
    let name = if case >= 3 { "Magic Rope" } else { "Levitate" };
    let spell = (1..=u32::try_from(deps.spells.source_len())?)
        .filter_map(NonZeroU32::new)
        .find(|index| {
            deps.spells
                .source_indexed(*index)
                .is_some_and(|(spell, enabled)| enabled && spell.name == name)
        })
        .ok_or("source spell")?;
    let intent = ParameterSpellCastIntent {
        intent: SpellCastIntent {
            spell,
            target: SpellTarget::None,
            aim_at_target: false,
        },
        parameter: (case < 3).then(|| if case == 1 { "down" } else { "up" }.into()),
    };
    let before = owner.observe_vitals(actor, session).await.ok_or("vitals")?;
    if use_bundle {
        let text = match case {
            0 => "exani hur \"up",
            1 => "exani hur \"down",
            _ => "exani tera",
        };
        let (chat, cast) = owner
            .complete_chat_intent(
                actor,
                session,
                1,
                &oteryn_protocol_oteryn::chat::ChatIntent::Say {
                    mode: oteryn_protocol_oteryn::chat::ChatSpeechMode::Say,
                    text: text.into(),
                },
            )
            .await
            .ok_or("spoken cast")?;
        assert_eq!(
            chat.disposition,
            oteryn_protocol_oteryn::chat::ChatDisposition::Ok,
            "case {case}: {cast:?}"
        );
        assert_eq!(
            cast.ok_or("chat spell outcome")?.disposition,
            SpellCastDisposition::Cast
        );
    }
    let dispatch = owner
        .cast_parameters(actor, session, 1, &intent, &UnavailableAccessOwner)
        .await;
    assert!(
        matches!(dispatch.cast, NativeCastDispatch::Outcome(_)),
        "case {case}"
    );
    let result = dispatch.result.ok_or("parameter result")?;
    let disposition = if case == 2 {
        SpellCastDisposition::Rejected
    } else {
        SpellCastDisposition::Cast
    };
    assert_eq!(result.disposition, disposition, "case {case}");
    let feedback = result.feedback.as_ref().ok_or("source feedback")?;
    assert_eq!(
        feedback.text,
        if case == 2 {
            "Sorry, not possible."
        } else {
            ""
        }
    );
    assert_eq!(
        feedback.effect,
        if case == 2 {
            PrivateSpellEffect::Poff
        } else {
            PrivateSpellEffect::Teleport
        }
    );
    let destination = if use_bundle {
        match case {
            0 => at(10, 9, 5),
            1 => at(10, 10, 6),
            _ => at(10, 11, 7),
        }
    } else {
        match case {
            0 => at(1, 4, -5),
            1 => at(1, 5, -6),
            3 => at(7, 13, -7),
            4 => at(7, 15, -7),
            _ => start,
        }
    };
    assert_eq!(
        deps.runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        destination
    );
    let after = owner
        .observe_vitals(actor, session)
        .await
        .ok_or("paid vitals")?;
    assert_eq!(
        after.1.mana,
        before.1.mana
            - match case {
                2 => 0,
                3 | 4 => 20,
                _ => 50,
            }
    );
    // All candidates reject the projectile barrier; the source south fallback ignores it.
    let replay = owner
        .cast_parameters(actor, session, 1, &intent, &UnavailableAccessOwner)
        .await;
    assert_eq!(replay.result, Some(result));
    assert_eq!(owner.observe_vitals(actor, session).await, Some(after));
    assert_eq!(
        deps.runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        destination
    );
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

async fn seed_rope_blockers(
    h: &Harness,
    deps: &Dependencies,
    room: &crate::content::QualifiedNativeEntryRoom,
    native: &crate::content::native_gameplay::NativeGameplayState,
    all: bool,
) -> TestResult {
    let policy = native
        .item_policy_for_source_id(10181)
        .ok_or("projectile barrier")?;
    assert_eq!(
        oteryn_gameplay_items::native_world_items::source_barrier_collision(policy),
        Some((false, true))
    );
    let definition = &policy.record().production_definition;
    let targets = {
        let runtime = deps.runtime.lock().await;
        let points = [
            (7, 15),
            (7, 13),
            (8, 14),
            (6, 14),
            (6, 15),
            (8, 15),
            (6, 13),
            (8, 13),
        ];
        points
            .iter()
            .take(if all { 8 } else { 1 })
            .map(|&(x, y)| {
                crate::durability::native_world_items::SpellGroundTarget::for_native_tile_read(
                    room,
                    &runtime,
                    at(x, y, -7),
                )
                .map_err(Into::into)
            })
            .collect::<TestResult<Vec<_>>>()?
    };
    let cost = serde_json::to_vec(&json!({
        "version":1,"before_revision":0,"after_revision":1,
        "caster_before":vec![1;32],"caster_after":vec![2;32],
        "mana_before":100,"mana_after":50,"soul_before":5,"soul_after":4,
        "cooldowns_before":[],"cooldowns_after":[["spell:fixture",1000000]]
    }))?;
    let mut tx = h.pool.begin().await?;
    // Fixture custody only. Casts read these actual durable rows under the production fences.
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_spell_item_receipts(transaction_id,event_id,game_session_id,command_id,character_id,world_id,channel_id,ownership_generation,spell_family,spell_production_key,spell_revision,catalog_digest,binding,intent,cost,cost_binding,operation_count,occurred_at_unix_ms) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,999,encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,1,'Spell','fixture:vertical_blocker','r1',$7,sha256($8),$8,$9,sha256($9),$10,1)")
        .bind(id(120).as_slice()).bind(id(121).as_slice()).bind(id(50).as_slice())
        .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(native.source_digest().as_slice()).bind(b"vertical-blocker".as_slice()).bind(&cost)
        .bind(i32::try_from(targets.len())?).execute(&mut *tx).await?;
    for (ordinal, target) in targets.iter().enumerate() {
        let item = id(130 + u8::try_from(ordinal)?);
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($5,'hex')::uuid)")
            .bind(item.as_slice()).bind(id(42).as_slice()).bind(&definition.production_key)
            .bind(&definition.revision_ref).bind(id(120).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_spell_item_lines(transaction_id,ordinal,operation_kind,item_instance_id,definition_family,definition_production_key,definition_revision,quantity_before,quantity_after,state_revision_before,world_id,channel_id,spatial_position,map_revision,content_revision,placement_context,content_generation_digest,blocks_movement,blocks_projectile,immovable_block_solid) VALUES(encode($1,'hex')::uuid,$2,1,encode($3,'hex')::uuid,'Item',$4,$5,0,1,0,encode($6,'hex')::uuid,encode($7,'hex')::uuid,$8,$9,$10,$11,$12,false,true,false)")
            .bind(id(120).as_slice()).bind(i32::try_from(ordinal + 1)?).bind(item.as_slice())
            .bind(&definition.production_key).bind(&definition.revision_ref)
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(&target.spatial_position)
            .bind(&target.map_revision).bind(&target.content_revision).bind(&target.placement_context)
            .bind(native.source_digest().as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,$4,$5,$6,$7,$8,$9)")
            .bind(item.as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(&target.spatial_position).bind(b"vertical-blocker".as_slice())
            .bind(&target.map_revision).bind(&target.content_revision).bind(&target.placement_context)
            .bind(i64::try_from(ordinal + 1)?).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_spell_item_audit_outbox(event_id,transaction_id,occurred_at_unix_ms,expires_at_unix_ms,envelope,envelope_sha256) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,7776000001,$3,sha256($3))")
        .bind(id(121).as_slice()).bind(id(120).as_slice()).bind(b"vertical-blocker".as_slice())
        .execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
