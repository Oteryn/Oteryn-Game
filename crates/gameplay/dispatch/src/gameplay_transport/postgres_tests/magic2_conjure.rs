//! MAGIC-2: conjure inventory failure is terminal and failure-atomic through real dispatch.
use super::death_ml_loss_composed::{current_fence, production_generation_with_source};
use super::fixture::{Harness, debug, id, run};
use super::item_look::{large_stack, seed_main_backpack};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};
use crate::foundation::{ChannelContentPin, CharacterLease};
use crate::gameplay_transport::SeamRooms;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::spell::cast::CharacterCastFacts;
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellCastIntent, SpellTarget};
use std::num::NonZeroU32;

const BLANK: &str = "oteryn:item.tibia.i3147";
const MISSILE: &str = "oteryn:item.tibia.i3198";

#[test]
fn magic2_conjure_postgres_full_bag_rolls_back_then_merges_mints_and_replays() -> TestResult {
    large_stack(|| scenario(true, false))
}

#[test]
fn magic2_conjure_postgres_missing_bag_rolls_back_and_missing_reagent_is_terminal() -> TestResult {
    large_stack(|| {
        scenario(false, true)?;
        scenario(false, false)
    })
}

fn scenario(full: bool, missing_reagent: bool) -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "magic2_conjure", true, false).await?;
        eprintln!("POSTGRES_STARTED MAGIC-2 conjure full={full}");
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
        let native = active.native_gameplay().ok_or("native gameplay")?;
        let room = pin.qualified_room();
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            room.map_revision_digest(),
            (2, 11, -7),
        );
        let deps =
            Dependencies::with_content(channel_pin, room.clone(), native.spell_book().clone())?;
        let actor = deps.actor;
        let session = deps.admitted.game_session_id;
        {
            let mut runtime = deps.runtime.lock().await;
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
            runtime
                .initialize_first_entry_position(actor)
                .map_err(debug)?;
        }
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.rooms = SeamRooms::new(Some(room), Some(room));
        let formula = native.training_formula().ok_or("training formula")?;
        let build = convert_vocation(
            formula,
            &DurableBuildState::default(),
            if full || missing_reagent {
                "druid"
            } else {
                "paladin"
            },
        )
        .map_err(debug)?;
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
        {
            let runtime = deps.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            states
                .initialize(
                    &runtime,
                    actor,
                    session,
                    CharacterCastFacts {
                        vocation: if full || missing_reagent {
                            crate::spell::Vocation::Druid
                        } else {
                            crate::spell::Vocation::Paladin
                        },
                        level: 50,
                        max_mana: 2000,
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
        if full {
            let mut entries = vec![(BLANK, 2), (MISSILE, 95)];
            entries.extend(std::iter::repeat_n(("oteryn:item.tibia.i3607", 1), 18));
            seed_main_backpack(&h, &entries).await?;
        }
        if missing_reagent {
            seed_main_backpack(&h, &[]).await?;
        }
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(pin.identity().server_artifact_digest().as_slice())
            .bind(pin.identity().client_artifact_digest().as_slice())
            .bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let find = |result| -> TestResult<SpellCastIntent> {
            let spell = (1..=u32::try_from(native.spell_book().source_len())?)
                .filter_map(NonZeroU32::new).find(|index| {
                    native.spell_book().source_indexed(*index).is_some_and(|(spell, enabled)| {
                        enabled && matches!(spell.execution, crate::spell::Execution::Conjure { result: id, .. } if id == result)
                    })
                }).ok_or("conjure source index")?;
            Ok(SpellCastIntent {
                spell,
                target: SpellTarget::None,
                aim_at_target: false,
            })
        };
        // Without a bag, use a reagent-free spell so the failure reaches the destination writer.
        let intent = find(if full || missing_reagent { 3198 } else { 3447 })?;
        let before = owner.observe_vitals(actor, session).await.ok_or("vitals")?;
        let outcome = owner
            .cast_spell_completion(actor, session, 1, intent)
            .await
            .ok_or("terminal no-room result")?;
        assert_eq!(outcome.disposition, SpellCastDisposition::Rejected);
        assert_eq!(owner.observe_vitals(actor, session).await, Some(before));
        assert_eq!(receipt_count(&h).await?, 0);
        assert_eq!(quantity(&h, BLANK).await?, if full { 2 } else { 0 });
        assert_eq!(quantity(&h, MISSILE).await?, if full { 95 } else { 0 });
        let ground: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_ground_locations g JOIN game_spell_item_lines l USING(item_instance_id) WHERE l.transaction_id IN (SELECT transaction_id FROM game_spell_item_receipts)")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(ground, 0, "a conjure never spills output onto the ground");
        if missing_reagent {
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        if full {
            // Remove one filler through fixture setup only, freeing a real direct bag slot.
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role = replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(73).as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
        } else {
            seed_main_backpack(&h, &[]).await?;
        }
        let outcome = owner
            .cast_spell_completion(actor, session, 3, intent)
            .await
            .ok_or("cast result after room repair")?;
        assert_eq!(outcome.disposition, SpellCastDisposition::Cast);
        let after = owner
            .observe_vitals(actor, session)
            .await
            .ok_or("paid vitals")?;
        // T-SOULCAP: this Premium caster has never purchased promotion.
        {
            let runtime = deps.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let state = states.get(&runtime, actor, session).ok_or("caster")?;
            assert_eq!(state.character_facts().max_soul, 100);
        }
        assert_eq!(after.1.mana, before.1.mana - if full { 350 } else { 100 });
        assert_eq!(after.1.soul, before.1.soul - if full { 2 } else { 1 });
        assert_eq!(quantity(&h, BLANK).await?, i64::from(full));
        if !full {
            assert_eq!(quantity(&h, "oteryn:item.tibia.i3447").await?, 10);
        }
        assert_eq!(quantity(&h, MISSILE).await?, if full { 105 } else { 0 });
        assert_eq!(receipt_count(&h).await?, 1);
        if full {
            let stacks: Vec<i64> = sqlx::query_scalar("SELECT i.quantity FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.definition_production_key=$1 AND i.lifecycle=1 AND e.character_id=encode($2,'hex')::uuid ORDER BY i.quantity LIMIT 21")
                .bind(MISSILE).bind(id(41).as_slice()).fetch_all(&h.pool).await?;
            assert_eq!(stacks, vec![5, 100]);
        }
        assert_eq!(
            owner
                .cast_spell_completion(actor, session, 3, intent)
                .await
                .ok_or("replay")?
                .disposition,
            SpellCastDisposition::Cast
        );
        assert_eq!(owner.observe_vitals(actor, session).await, Some(after));
        assert_eq!(receipt_count(&h).await?, 1);
        assert_eq!(
            owner
                .cast_spell_completion(actor, session, 4, intent)
                .await
                .ok_or("cooldown result")?
                .disposition,
            SpellCastDisposition::CoolingDown
        );
        assert_eq!(owner.observe_vitals(actor, session).await, Some(after));
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn receipt_count(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts")
            .fetch_one(&h.pool)
            .await?,
    )
}

async fn quantity(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.definition_production_key=$1 AND i.lifecycle=1 AND e.character_id=encode($2,'hex')::uuid")
        .bind(key).bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}
