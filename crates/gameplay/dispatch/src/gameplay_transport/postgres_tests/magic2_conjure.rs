//! MAGIC-2: conjure inventory failure is terminal and failure-atomic through real dispatch.
use super::death_ml_loss_composed_fixtures::{current_fence, production_generation_with_source};
use super::fixture::{Harness, debug, id, run};
use super::item_look_fixtures::{large_stack, seed_main_backpack};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};
use crate::foundation::{ChannelContentPin, CharacterLease};
use crate::gameplay_transport::actor_spell::NativeCastDispatch;
use crate::gameplay_transport::connection::{AdmissionRefusal, FreshAdmissionAuthority};
use crate::gameplay_transport::spell_access_facts::{
    AccessFactsError, CurrentSpellAccessOwner, owner_registration,
};
use crate::gameplay_transport::{AdmissionRetryBudget, SeamRooms};
use crate::spell::cast::CharacterCastFacts;
use crate::spell::owned_cast_facts::{AccessProjections, CastFactsBinding};
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellCastIntent, SpellTarget};
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicU64, Ordering};

const BLANK: &str = "oteryn:item.tibia.i3147";
const LIGHT_MISSILE: &str = "oteryn:item.tibia.i3174";
const MISSILE: &str = "oteryn:item.tibia.i3198";

#[test]
fn magic2_conjure_postgres_full_bag_rolls_back_then_merges_mints_and_replays() -> TestResult {
    large_stack(|| scenarios(&[(true, false)], false, false))
}

#[test]
fn magic2_conjure_postgres_missing_bag_rolls_back_and_missing_reagent_is_terminal() -> TestResult {
    large_stack(|| scenarios(&[(false, true), (false, false)], false, false))
}

#[test]
fn c10_light_magic_missile_conjure_uses_preparation_clock_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    large_stack(|| scenarios(&[(false, false)], false, true))
}

struct AdvanceDuringAccess<'a> {
    clock: &'a AtomicU64,
    ready_at: u64,
}

impl owner_registration::Registered for AdvanceDuringAccess<'_> {}
impl CurrentSpellAccessOwner for AdvanceDuringAccess<'_> {
    fn read_current(
        &self,
        _: &CastFactsBinding,
        _: u64,
    ) -> Result<AccessProjections, AccessFactsError> {
        self.clock.store(self.ready_at + 1, Ordering::Relaxed);
        Ok(AccessProjections::default())
    }
}

#[test]
fn magic2_conjure_postgres_without_qualified_room() -> TestResult {
    large_stack(|| scenarios(&[(true, false), (false, false)], true, false))
}

fn scenarios(cases: &[(bool, bool)], combat_room_only: bool, cooldown_clock: bool) -> TestResult {
    run(async |admin| {
        // Compile content before opening the admission holder; cold Native setup can exceed
        // the holder's idle lifetime on a contended CI worker.
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = production_generation_with_source(
            world,
            "content-1",
            Some(include_bytes!(
                "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
            )),
        )?;
        for &(full, missing_reagent) in cases {
            scenario(
                admin.clone(),
                world,
                &controller,
                &pin,
                full,
                missing_reagent,
                cooldown_clock,
                combat_room_only,
            )
            .await?;
        }
        Ok(())
    })
}

// Each case has fresh database/runtime state, but the identical sealed content generation
// is immutable and can be prepared once before either admission begins.
async fn scenario(
    admin: String,
    world: crate::foundation::WorldId,
    controller: &crate::content::ContentActivationController,
    pin: &crate::content::NativeEntryContentPin,
    full: bool,
    missing_reagent: bool,
    cooldown_clock: bool,
    combat_room_only: bool,
) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "magic2_conjure", true, false).await?;
    eprintln!("POSTGRES_STARTED MAGIC-2 conjure full={full}");
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
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
    let deps = Dependencies::with_content(channel_pin, room.clone(), native.spell_book().clone())?;
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
        if full || missing_reagent || cooldown_clock {
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
                    vocation: if full || missing_reagent || cooldown_clock {
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
    if cooldown_clock {
        seed_main_backpack(&h, &[(BLANK, 2)]).await?;
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
    let intent = find(if cooldown_clock {
        3174
    } else if full || missing_reagent {
        3198
    } else {
        3447
    })?;
    assert!(
        AdmissionRetryBudget::for_root(&session, &h.root)
            .retry_until(async || owner.ensure_source_map_initialized().await)
            .await,
        "source-map initialization before conjure"
    );
    if combat_room_only {
        owner.rooms = SeamRooms::new(None, Some(room));
    }
    // A pending response keeps the original command outstanding. The direct test
    // owner must resubmit that command to obtain its terminal result.
    let cast = async |command| {
        AdmissionRetryBudget::for_root(&session, &h.root)
            .retry_unavailable(async || {
                owner
                    .cast_spell_completion(actor, session, command, intent)
                    .await
                    .ok_or(AdmissionRefusal::Unavailable)
            })
            .await
            .ok()
    };
    let before = owner.observe_vitals(actor, session).await.ok_or("vitals")?;
    if cooldown_clock {
        let clock = &owner.states.test_owner_time;
        let now = owner.owner_now().get();
        clock.store(now + 1, Ordering::Relaxed);
        let first = cast(2).await.ok_or("first Light Magic Missile cast")?;
        assert_eq!(first.disposition, SpellCastDisposition::Cast);
        let after = owner
            .observe_vitals(actor, session)
            .await
            .ok_or("paid vitals")?;
        assert_eq!(after.1.soul, before.1.soul - 1);
        let (spell, _) = native
            .spell_book()
            .source_indexed(intent.spell)
            .ok_or("spell")?;
        let ready_at = now + spell.cooldown_micros;
        assert_eq!(spell.cooldown_micros, 2_000_000);
        clock.store(ready_at, Ordering::Relaxed); // One microsecond before expiry.
        let access = AdvanceDuringAccess { clock, ready_at };
        let refused = owner
            .cast_world_items(actor, session, 3, &intent, &access)
            .await;
        let NativeCastDispatch::Outcome(refused) = refused else {
            return Err("second Light Magic Missile cast did not settle".into());
        };
        assert_eq!(
            refused.disposition,
            SpellCastDisposition::CoolingDown,
            "a cast prepared before expiry refuses even when its access read crosses expiry"
        );
        assert_eq!(owner.observe_vitals(actor, session).await, Some(after));
        assert_eq!(quantity(&h, BLANK).await?, 1);
        assert_eq!(quantity(&h, LIGHT_MISSILE).await?, 10);
        assert_eq!(receipt_count(&h).await?, 1);
        drop(owner);
        drop(authority);
        drop(seal);
        return h.cleanup().await;
    }
    if full {
        let holder = h.root.try_acquire_ready().map_err(debug)?;
        assert!(
            owner
                .cast_spell_completion(actor, session, 1, intent)
                .await
                .is_none(),
            "a busy holder leaves the full-bag cast pending"
        );
        drop(holder);
    }
    let outcome = cast(1).await.ok_or("terminal no-room result")?;
    assert_eq!(outcome.disposition, SpellCastDisposition::Rejected);
    if full {
        assert_eq!(outcome.vitals, Some(before), "full-bag rollback result");
    }
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
        sqlx::query(
            "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(id(73).as_slice())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
    } else {
        seed_main_backpack(&h, &[]).await?;
    }
    let outcome = cast(3).await.ok_or("cast result after room repair")?;
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
        cast(3).await.ok_or("replay")?.disposition,
        SpellCastDisposition::Cast
    );
    assert_eq!(owner.observe_vitals(actor, session).await, Some(after));
    assert_eq!(receipt_count(&h).await?, 1);
    assert_eq!(
        cast(4).await.ok_or("cooldown result")?.disposition,
        SpellCastDisposition::CoolingDown
    );
    assert_eq!(owner.observe_vitals(actor, session).await, Some(after));
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
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
