//! Crystal 201189f8 potions.lua:21-26,36-42,57,62,78-110,141-147.
//! Real USE frames, actual player condition owner, fenced PostgreSQL burn and flask.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::domain::condition::{ConflictKey, DotElement};
use crate::foundation::ChannelContentPin;
use crate::foundation::{
    ApplicationFacts, ConditionDefinition, ConditionSourceKind, ConditionValues,
};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, decode_use_result, encode_item_use_intent,
};
use std::num::{NonZeroU32, NonZeroU64};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

pub(super) async fn exchange(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: super::AdmittedSession,
    command_id: u64,
    payload: &[u8],
    current_source: Option<NonZeroU32>,
    current_target: Option<NonZeroU32>,
) -> TestResult<(super::AdmittedSession, CommandStatus, Vec<u8>, Vec<u8>)> {
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use oteryn_protocol_oteryn::world_object::MapUseTarget;
    use std::future::Future;
    use std::task::Poll;

    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let driving = async {
        let mut source = None;
        let mut target = None;
        loop {
            let raw = super::item_look::read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::SnapshotChunk {
                let (_, domains) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())?;
                for domain in domains {
                    if domain.domain_id
                        == oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY
                    {
                        let inventory =
                            oteryn_protocol_oteryn::item_view::decode_character_inventory(
                                domain.payload,
                            )
                            .map_err(debug)?;
                        source = inventory
                            .entries
                            .iter()
                            .find(|entry| Some(entry.item_definition_ref) == current_source)
                            .map(|entry| entry.handle);
                        target = inventory
                            .entries
                            .iter()
                            .find(|entry| Some(entry.item_definition_ref) == current_target)
                            .map(|entry| entry.handle);
                    }
                }
            }
            if envelope.message_type() == crate::foundation::MessageType::SnapshotCommit {
                break;
            }
        }
        let actual = if current_source.is_some() || current_target.is_some() {
            let mut intent = oteryn_protocol_oteryn::world_object::decode_use_intent_target(
                admitted.continuity.selected_capabilities.as_slice(),
                payload,
            )
            .map_err(debug)?;
            if current_source.is_some() {
                intent.target = UseTarget::Item(source.ok_or("current source handle")?);
            }
            if current_target.is_some() {
                intent.use_with_map_item =
                    Some(MapUseTarget::Item(target.ok_or("current target handle")?));
            }
            encode_item_use_intent(&intent).map_err(debug)?
        } else {
            payload.to_vec()
        };
        let frame = oteryn_protocol_oteryn::encode_client_command(
            admitted.continuity.connection_generation,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id,
                command_type: 2,
                payload: &actual,
            },
        )?;
        client.write_u32(u32::try_from(frame.len())?).await?;
        client.write_all(&frame).await?;
        client.shutdown().await?;
        loop {
            let raw = super::item_look::read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::CommandResult {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                if result.command_id == command_id {
                    return Ok::<_, Box<dyn std::error::Error>>((
                        result.status,
                        result.payload.to_vec(),
                        actual,
                    ));
                }
            }
        }
    };
    let serving = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let mut serving = std::pin::pin!(serving);
    let mut driving = std::pin::pin!(driving);
    let mut end = None;
    let mut result = None;
    tokio::time::timeout(
        Duration::from_secs(30),
        std::future::poll_fn(|context| {
            if end.is_none()
                && let Poll::Ready(value) = serving.as_mut().poll(context)
            {
                end = Some(value);
            }
            if result.is_none()
                && let Poll::Ready(value) = driving.as_mut().poll(context)
            {
                result = Some(value);
            }
            if end.is_some() && result.is_some() {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }),
    )
    .await?;
    let end = end.ok_or("server did not finish")?;
    let ConnectionEnd::AdmittedThenDisconnected(admitted) = end else {
        return Err(format!("{end:?}").into());
    };
    let (status, result, actual) = result.ok_or("client did not finish")??;
    Ok((admitted, status, result, actual))
}

#[test]
fn antidote_potion_postgres_use_cures_poison_flask_cooldown_replay() -> TestResult {
    run_case(7644)
}

#[test]
fn magic_shield_potion_postgres_use_capacity_expiry_requirements_replay() -> TestResult {
    run_case(35563)
}

fn run_case(item: u32) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            scenario(item, "druid", 50).map_err(|error| error.to_string())?;
            if item == 35563 {
                scenario(item, "druid", 13).map_err(|error| error.to_string())?;
                scenario(item, "knight", 50).map_err(|error| error.to_string())?;
            }
            Ok::<_, String>(())
        })?
        .join()
        .map_err(|_| "special potion thread panicked")??;
    Ok(())
}

async fn inventory_fingerprint(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id)::text FROM game_item_instances i",
    )
    .fetch_one(&h.pool)
    .await?)
}

async fn potion_progress(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE((SELECT progress FROM game_character_achievement_progress WHERE character_id=encode($1,'hex')::uuid AND achievement_key='oteryn:achievement/potion_addict'),0)")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}

fn scenario(item: u32, vocation: &'static str, level: u32) -> TestResult {
    run(async move |admin| {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "i14_special_potion", true, false).await?;
        eprintln!("PG_MARKER i14-special-potion-start {item}");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let channel = crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?;
        let (controller, activation) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let generation = controller.active().ok_or("active generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("item index")?;
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let (bytes, map_pins) = crate::map::boot::tests::bundle_with_item_ids(
            (
                "oteryn:item.tibia.i2854",
                reference("oteryn:item.tibia.i2854").ok_or("bag")?.get() - 1,
            ),
            (
                "oteryn:item.tibia.i2535",
                reference("oteryn:item.tibia.i2535").ok_or("oven")?.get() - 1,
            ),
        );
        let base = crate::map::load(&bytes, &map_pins).map_err(debug)?;
        let map = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: map_pins,
                start: crate::map::overlay::TilePos {
                    x: 6,
                    y: 0,
                    floor: -7,
                },
            },
            world,
            channel,
            |key| {
                let policy = native.item_policy(key, "definition-r1")?.record();
                let crate::content::ReferenceItemField::Known(physical) =
                    &policy.semantics.physical
                else {
                    return None;
                };
                Some(crate::map::facts::ItemDefinition {
                    reference: reference(key)?,
                    solid: policy.attributes.blocks_movement,
                    blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                    pickupable: physical.pickupable
                        == crate::content::ReferenceItemField::Known(true),
                    door: false,
                })
            },
        )
        .map_err(debug)?;
        let room = activation.qualified_room().clone();
        let pin = ChannelContentPin::from_activation(
            world,
            activation.activation_sequence(),
            activation.identity().server_artifact_digest(),
            activation.identity().client_artifact_digest(),
            activation.frame_binding().digest(),
            room.map_revision_digest(),
            (6, 0, 7),
        );
        let deps = Dependencies::with_content(
            pin.clone(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let cells = map
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(pin.server_artifact_digest().as_slice())
            .bind(pin.client_artifact_digest().as_slice())
            .bind(pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&h.pool).await?;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    deps.admitted.game_session_id,
                    current.current_character_lease(),
                )
                .map_err(debug)?;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
        }
        let key = format!("oteryn:item.tibia.i{item}");
        super::item_look::seed_main_backpack(&h, &[(&key, 5)]).await?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&deps.room));
        owner.clock_origin = std::time::Instant::now()
            .checked_sub(Duration::from_secs(60))
            .ok_or("clock")?;
        {
            let runtime = deps.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let facts = crate::spell::cast::CharacterCastFacts {
                vocation: crate::spell::Vocation::from_key(vocation).ok_or("vocation")?,
                level,
                magic_level: 12,
                max_health: 500,
                max_mana: 1_000,
                max_soul: 100,
            };
            states
                .initialize(
                    &runtime,
                    deps.actor,
                    deps.admitted.game_session_id,
                    facts,
                    (0, 0),
                    owner.owner_now(),
                )
                .ok_or("potion player state")?;
            let build = crate::durability::character_build::DurableBuildState::new(
                vocation,
                (12, 0),
                [(50, 0); 7],
            )
            .map_err(debug)?;
            let state = states
                .get_mut(&runtime, deps.actor, deps.admitted.game_session_id)
                .ok_or("state")?;
            state.load_owned_build(&build).map_err(debug)?;
            if item == 7644 {
                let decision =
                    oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes([9; 32]);
                for (key, element) in [
                    ("oteryn:test.poison", DotElement::Poison),
                    ("oteryn:test.fire", DotElement::Fire),
                ] {
                    let definition = ConditionDefinition::new(
                        key,
                        1,
                        ConditionValues::DamageOverTime {
                            element,
                            total_min: 100,
                            total_max: 100,
                            per_tick: 1,
                            interval_ms: 10_000,
                            delayed: true,
                        },
                    )
                    .ok_or("fixture DOT")?;
                    state
                        .conditions
                        .apply(
                            &definition,
                            None,
                            ConditionSourceKind::SelfUse,
                            &[],
                            &ApplicationFacts {
                                now: owner.owner_now().get(),
                                base_speed: 0,
                                mana_shield_capacity: 0,
                                target_reentry_protected: false,
                                source_reentry_protected: false,
                                target_is_player: true,
                                decision_root: &decision,
                                occurrence:
                                    oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(
                                        id(90),
                                    ),
                            },
                        )
                        .map_err(debug)?;
                }
            }
        }
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 15, 18])
                .ok_or("potion capabilities")?;
        let payload = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(NonZeroU64::new(2).ok_or("potion handle")?),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(debug)?;
        let before = inventory_fingerprint(&h).await?;
        let (next, status, result, committed_payload) =
            exchange(&owner, admitted, 1, &payload, reference(&key), None).await?;
        admitted = next;
        assert_eq!(status, CommandStatus::Accepted);
        if item == 35563 && (level < 14 || vocation == "knight") {
            assert_eq!(
                decode_use_result(&result).map_err(debug)?,
                UseDisposition::RequirementNotMet
            );
            assert_eq!(inventory_fingerprint(&h).await?, before);
            eprintln!("PG_MARKER i14-magic-shield-requirement-refusal {vocation} {level}");
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        assert_eq!(
            decode_use_result(&result).map_err(debug)?,
            UseDisposition::Committed
        );
        let quantity: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(quantity, 4);
        let flasks: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i285' AND lifecycle=1")
            .fetch_one(&h.pool).await?;
        assert_eq!(
            flasks,
            i64::from(item == 7644),
            "only antidote returns the donor flask"
        );
        assert_eq!(potion_progress(&h).await?, i64::from(item == 7644));
        let (applied, expires): (String,String) = sqlx::query_as("SELECT applied_owner_micros::text,expires_owner_micros::text FROM game_item_condition_potion_receipts WHERE condition_key=$1")
            .bind(&key).fetch_one(&h.pool).await?;
        let applied: u64 = applied.parse()?;
        let expires: u64 = expires.parse()?;
        assert_eq!(expires - applied, if item == 7644 { 0 } else { 60_000_000 });
        super::cl6_status::check_timer(&owner, deps.actor, admitted.game_session_id, &key, item == 35563, 60_000).await?;
        let after_conditions = {
            let runtime = deps.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let state = states
                .get(&runtime, deps.actor, admitted.game_session_id)
                .ok_or("used state")?;
            if item == 7644 {
                assert!(
                    state
                        .conditions
                        .get(ConflictKey::Element(DotElement::Poison))
                        .is_none()
                );
                assert!(
                    state
                        .conditions
                        .get(ConflictKey::Element(DotElement::Fire))
                        .is_some()
                );
            } else {
                let shield = state
                    .conditions
                    .get(ConflictKey::ManaShield)
                    .ok_or("shield installed")?;
                assert_eq!(
                    shield.mana_shield(),
                    (764, 764),
                    "min(max mana, 300 + 7.6*50 + 7*12)"
                );
                assert_eq!(shield.remaining_duration_ms(expires), Some(0));
            }
            state.conditions.clone()
        };
        let after = inventory_fingerprint(&h).await?;
        let (next, _, replay, _) =
            exchange(&owner, admitted, 1, &committed_payload, None, None).await?;
        admitted = next;
        assert_eq!(
            decode_use_result(&replay).map_err(debug)?,
            UseDisposition::Committed
        );
        assert_eq!(inventory_fingerprint(&h).await?, after);
        assert_eq!(
            potion_progress(&h).await?,
            i64::from(item == 7644),
            "replay cannot advance the donor counter"
        );
        {
            let runtime = deps.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            assert_eq!(
                states
                    .get(&runtime, deps.actor, admitted.game_session_id)
                    .ok_or("replayed state")?
                    .conditions,
                after_conditions,
                "replay neither reapplies nor extends a condition"
            );
        }
        // Place the next framed USE inside the source action delay on the fixture's owner clock.
        owner.clock_origin = std::time::Instant::now()
            .checked_sub(Duration::from_micros(applied + 100_000))
            .ok_or("cooldown clock")?;
        let (next, _, exhausted, _) =
            exchange(&owner, admitted, 2, &payload, reference(&key), None).await?;
        admitted = next;
        assert_eq!(
            decode_use_result(&exhausted).map_err(debug)?,
            UseDisposition::Exhausted
        );
        assert_eq!(inventory_fingerprint(&h).await?, after);
        if item == 7644 {
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(Duration::from_secs(2))
                .ok_or("next potion time")?;
            let (_, _, clean_use, _) =
                exchange(&owner, admitted, 3, &payload, reference(&key), None).await?;
            assert_eq!(
                decode_use_result(&clean_use).map_err(debug)?,
                UseDisposition::Committed,
                "the donor permits antidote use even when poison is already absent"
            );
            assert_eq!(potion_progress(&h).await?, 2);
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(71).as_slice()).fetch_one(&h.pool).await?, 3);
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i285' AND lifecycle=1")
                .fetch_one(&h.pool).await?, 2);
        } else {
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(Duration::from_secs(61))
                .ok_or("shield expiry time")?;
            let runtime = deps.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            states
                .tick(
                    &runtime,
                    deps.actor,
                    admitted.game_session_id,
                    owner.owner_now(),
                )
                .ok_or("actual condition owner tick")?;
            assert!(
                states
                    .get(&runtime, deps.actor, admitted.game_session_id)
                    .ok_or("expired owner")?
                    .conditions
                    .get(ConflictKey::ManaShield)
                    .is_none(),
                "the production condition owner expires the shield"
            );
            assert_eq!(
                inventory_fingerprint(&h).await?,
                after,
                "expiry cannot consume another potion"
            );
        }
        super::cl6_status::check_timer(&owner, deps.actor, admitted.game_session_id, &key, false, 60_000).await?;
        eprintln!("PG_MARKER i14-special-potion-effect-burn-flask-replay-cooldown {item}");
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
