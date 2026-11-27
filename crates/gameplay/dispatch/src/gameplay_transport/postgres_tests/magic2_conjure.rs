//! MAGIC-2: conjure inventory failure is terminal and failure-atomic through real dispatch.
use super::death_ml_loss_composed_fixtures::{current_fence, production_generation_with_source};
use super::fixture::{Harness, debug, id, run};
use super::item_look_fixtures::{large_stack, seed_main_backpack};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};
use crate::foundation::{ChannelContentPin, CharacterLease};
use crate::gameplay_transport::connection::{AdmissionRefusal, FreshAdmissionAuthority};
use crate::gameplay_transport::{AdmissionRetryBudget, SeamRooms};
use crate::spell::cast::CharacterCastFacts;
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellCastIntent, SpellTarget};
use std::num::NonZeroU32;

const BLANK: &str = "oteryn:item.tibia.i3147";
const MISSILE: &str = "oteryn:item.tibia.i3198";

#[derive(Clone, Copy)]
enum Case {
    Full,
    MissingBackpack,
    MissingReagent,
    WireHand,
}

#[test]
fn magic2_conjure_postgres_full_bag_rolls_back_then_merges_mints_and_replays() -> TestResult {
    large_stack(|| scenarios(&[Case::Full]))
}

#[test]
fn magic2_conjure_postgres_missing_bag_rolls_back_and_missing_reagent_is_terminal() -> TestResult {
    large_stack(|| scenarios(&[Case::MissingReagent, Case::MissingBackpack]))
}

#[test]
fn magic2_conjure_postgres_publishes_paid_inventory_and_vitals() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    large_stack(|| scenarios(&[Case::WireHand]))
}

fn scenarios(cases: &[Case]) -> TestResult {
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
        for &case in cases {
            scenario(admin.clone(), world, &controller, &pin, case).await?;
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
    case: Case,
) -> TestResult {
    let full = matches!(case, Case::Full);
    let missing_reagent = matches!(case, Case::MissingReagent);
    let wire = matches!(case, Case::WireHand);
    let druid = full || missing_reagent || wire;
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
        if druid {
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
                    vocation: if druid {
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
    let intent = find(if wire { 3174 } else if full || missing_reagent { 3198 } else { 3447 })?;
    assert!(
        AdmissionRetryBudget::for_root(&session, &h.root)
            .retry_until(async || owner.ensure_source_map_initialized().await)
            .await,
        "source-map initialization before conjure"
    );
    if wire {
        let result = check_conjure_wire(&h, &deps, &owner, intent).await;
        drop(owner);
        drop(authority);
        drop(seal);
        let cleanup = h.cleanup().await;
        return result.and(cleanup);
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
    if full {
        let holder = h.root.try_acquire_ready().map_err(debug)?;
        assert!(
            owner.cast_spell_completion(actor, session, 1, intent).await.is_none(),
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

async fn check_conjure_wire(
    h: &Harness,
    deps: &Dependencies,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    intent: SpellCastIntent,
) -> TestResult {
    use crate::durability::fresh_admission::FreshAdmissionStore;
    use crate::gameplay_transport::capabilities::SelectedCapabilities;
    use crate::gameplay_transport::connection::FirstEntryOutcome;
    use oteryn_protocol_oteryn::{
        ClientCommandValue, CommandStatus, MessageType, actor_spell, decode_command_result,
        decode_snapshot_chunk, decode_state_delta, decode_wire_envelope, encode_client_command,
        item_view,
    };

    const LIGHT_MISSILE: &str = "oteryn:item.tibia.i3174";
    const HAND_SLOT: i16 = 5;
    seed_main_backpack(h, &[]).await?;
    {
        let mut tx = h.pool.begin().await?;
        // Fixture custody only. The rune consumption, mint and cast payment use the real owners.
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(
            crate::durability::item_fee_burn::test_support::item(70, BLANK, "definition-r1", 1),
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($4,'hex')::uuid)")
            .bind(id(41).as_slice())
            .bind(id(42).as_slice())
            .bind(HAND_SLOT)
            .bind(id(70).as_slice())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
    }
    let item_index = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .and_then(|native| native.item_index())
        .ok_or("native item index")?;
    let blank = item_index
        .definition_ref("Item", BLANK, "definition-r1")
        .ok_or("blank rune")?;
    let missile = item_index
        .definition_ref("Item", LIGHT_MISSILE, "definition-r1")
        .ok_or("light missile rune")?;
    let mut admitted = deps.admitted;
    let current = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(admitted.game_session_id)
        .await?
        .0;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 7, 12])
            .ok_or("inventory and spell capabilities")?;
    let generation = admitted.continuity.connection_generation;
    let payload = actor_spell::encode_spell_cast_intent(&intent);
    let mut input = Vec::new();
    for command_id in [1, 2] {
        let frame = encode_client_command(
            generation,
            &ClientCommandValue {
                command_id,
                command_type: actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT,
                payload: &payload,
            },
        )?;
        input.extend(u32::try_from(frame.len())?.to_be_bytes());
        input.extend(frame);
    }
    let (_, output) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::weather_wire_fixtures::fixture_access::serve_input(owner, admitted, &input),
    )
    .await??;
    let mut before = None;
    let mut held = None;
    let mut debited = None;
    let mut inventory = None;
    let mut results = Vec::new();
    let mut frames = output.as_slice();
    while !frames.is_empty() {
        let length =
            u32::from_be_bytes(frames.get(..4).ok_or("frame length")?.try_into()?) as usize;
        let envelope = decode_wire_envelope(frames.get(4..4 + length).ok_or("frame")?)?;
        frames = &frames[4 + length..];
        match envelope.message_type() {
            MessageType::SnapshotChunk => {
                for domain in decode_snapshot_chunk(envelope.payload())?.1 {
                    match domain.domain_id {
                        actor_spell::STATE_DOMAIN_ACTOR_VITALS => {
                            before = Some(
                                actor_spell::decode_actor_vitals(domain.payload).map_err(debug)?,
                            );
                        }
                        item_view::STATE_DOMAIN_CHARACTER_INVENTORY => {
                            let snapshot = item_view::decode_character_inventory_with_equip_drop(
                                domain.payload,
                                true,
                            )
                            .map_err(debug)?;
                            held = snapshot
                                .equipment
                                .iter()
                                .find(|entry| entry.item.item_definition_ref == blank)
                                .copied();
                        }
                        _ => {}
                    }
                }
            }
            MessageType::CommandResult => {
                let result = decode_command_result(envelope.payload())?;
                let disposition =
                    actor_spell::decode_spell_cast_result(result.payload).map_err(debug)?;
                results.push((result.command_id, result.status, disposition));
            }
            MessageType::StateDelta => {
                let delta = decode_state_delta(envelope.payload())?;
                match delta.domain_id {
                    actor_spell::STATE_DOMAIN_ACTOR_VITALS => {
                        assert_eq!(
                            results.len(),
                            1,
                            "only the committed cast publishes a debit"
                        );
                        assert!(
                            debited
                                .replace(
                                    actor_spell::decode_actor_vitals(delta.payload)
                                        .map_err(debug)?
                                )
                                .is_none()
                        );
                    }
                    item_view::STATE_DOMAIN_CHARACTER_INVENTORY => {
                        assert_eq!(
                            results.len(),
                            1,
                            "inventory must follow the commit before the next command's refusal"
                        );
                        let changed = item_view::decode_character_inventory_with_equip_drop(
                            delta.payload,
                            true,
                        )
                        .map_err(debug)?;
                        assert!(inventory.replace(changed).is_none());
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    assert_eq!(results.len(), 2);
    assert_eq!(
        results[0],
        (1, CommandStatus::Accepted, SpellCastDisposition::Cast)
    );
    assert_ne!(results[1].2, SpellCastDisposition::Cast);
    let before = before.ok_or("initial vitals snapshot")?;
    let held = held.ok_or("blank rune in the initial equipment snapshot")?;
    assert_eq!(held.slot, item_view::EquipmentSlot::RightHand);
    let debited = debited.ok_or("committed conjure must publish ActorVitals")?;
    assert_eq!(before.mana.checked_sub(120), Some(debited.mana));
    assert_eq!(before.soul.checked_sub(1), Some(debited.soul));
    let inventory = inventory.ok_or("committed conjure must publish the inventory delta")?;
    assert!(
        !inventory
            .equipment
            .iter()
            .any(|entry| entry.slot == held.slot)
    );
    assert!(
        !inventory
            .entries
            .iter()
            .any(|entry| entry.item_definition_ref == blank)
    );
    // The current durable mint places the output in the backpack.
    let rune = inventory
        .entries
        .iter()
        .find(|entry| entry.item_definition_ref == missile)
        .ok_or("conjured rune in committed inventory")?;
    assert_eq!(rune.count.get(), 10);
    Ok(())
}

async fn quantity(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.definition_production_key=$1 AND i.lifecycle=1 AND e.character_id=encode($2,'hex')::uuid")
        .bind(key).bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}
