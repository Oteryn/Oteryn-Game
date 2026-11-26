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
use oteryn_protocol_oteryn::presentation as cooldown_wire;
use std::num::NonZeroU32;

const BLANK: &str = "oteryn:item.tibia.i3147";
const MISSILE: &str = "oteryn:item.tibia.i3198";
const WAND: &str = "oteryn:item.tibia.i25760";
const CAST_TIME_MS: u64 = 1_000;

#[derive(Clone, Copy)]
enum Case {
    FullBag,
    MissingBag,
    MissingReagent,
    WandInventory,
    WandCooldowns,
}

#[test]
fn magic2_conjure_postgres_full_bag_rolls_back_then_merges_mints_and_replays() -> TestResult {
    large_stack(|| scenarios(&[Case::FullBag]))
}

#[test]
fn magic2_conjure_postgres_missing_bag_rolls_back_and_missing_reagent_is_terminal() -> TestResult {
    large_stack(|| scenarios(&[Case::MissingReagent, Case::MissingBag]))
}

#[test]
fn conjure_wand_publishes_one_inventory_delta_and_debits_mana_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    large_stack(|| scenarios(&[Case::WandInventory]))
}

#[test]
fn conjure_publishes_spell_and_group_cooldown_cue() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    large_stack(|| scenarios(&[Case::WandCooldowns]))
}

#[test]
fn magic2_conjure_postgres_without_qualified_room() -> TestResult {
    large_stack(|| scenarios_with_room(&[Case::FullBag, Case::MissingBag], true))
}

fn scenarios(cases: &[Case]) -> TestResult {
    scenarios_with_room(cases, false)
}

fn scenarios_with_room(cases: &[Case], combat_room_only: bool) -> TestResult {
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
            scenario(admin.clone(), world, &controller, &pin, case, combat_room_only).await?;
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
    combat_room_only: bool,
) -> TestResult {
    let full = matches!(case, Case::FullBag);
    let missing_reagent = matches!(case, Case::MissingReagent);
    let wand = matches!(case, Case::WandInventory | Case::WandCooldowns);
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
    if matches!(case, Case::WandCooldowns) {
        owner.states.test_owner_time.store(
            CAST_TIME_MS * 1_000 + 1,
            std::sync::atomic::Ordering::Relaxed,
        );
    }
    let formula = native.training_formula().ok_or("training formula")?;
    let (vocation_key, vocation) = match case {
        Case::FullBag | Case::MissingReagent => ("druid", crate::spell::Vocation::Druid),
        Case::MissingBag => ("paladin", crate::spell::Vocation::Paladin),
        Case::WandInventory | Case::WandCooldowns => ("sorcerer", crate::spell::Vocation::Sorcerer),
    };
    let build =
        convert_vocation(formula, &DurableBuildState::default(), vocation_key).map_err(debug)?;
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
                    vocation,
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
    if missing_reagent || wand {
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
    let intent = find(match case {
        Case::FullBag | Case::MissingReagent => 3198,
        Case::MissingBag => 3447,
        Case::WandInventory | Case::WandCooldowns => 25760,
    })?;
    assert!(
        AdmissionRetryBudget::for_root(&session, &h.root)
            .retry_until(async || owner.ensure_source_map_initialized().await)
            .await,
        "source-map initialization before conjure"
    );
    if wand {
        let (consumer, clock, activation, _) =
            crate::gameplay_transport::spell_premium_coordinator::purchase_fixture::current(id(40));
        let premium =
            crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
                consumer,
                Some(clock),
                Some(activation),
                Some(h.root.clone()),
            );
        owner.premium_coordinator = Some(&premium);
        assert!(premium.premium_gate(id(40)).await.require().is_ok());
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(session)
                .await?
                .0;
        let mut admitted = deps.admitted;
        admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
        admitted.item_fence =
            Some(crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?);
        let wand_ref = native
            .item_index()
            .ok_or("item index")?
            .definition_ref("Item", WAND, "definition-r1")
            .ok_or("wand definition")?;
        assert_eq!(quantity(&h, WAND).await?, 0);
        assert_wand_on_wire(
            &owner,
            admitted,
            intent,
            wand_ref,
            matches!(case, Case::WandCooldowns),
        )
        .await?;
        assert_eq!(quantity(&h, WAND).await?, 1, "one wand in the backpack");
        assert_eq!(
            receipt_count(&h).await?,
            1,
            "replay creates no second receipt"
        );
        eprintln!("POSTGRES_CASE_COMPLETED W09 wand inventory");
        drop(owner);
        drop(authority);
        drop(seal);
        return h.cleanup().await;
    }
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

async fn assert_wand_on_wire(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: crate::gameplay_transport::connection::AdmittedSession,
    intent: SpellCastIntent,
    wand: NonZeroU32,
    check_cooldowns: bool,
) -> TestResult {
    use crate::gameplay_transport::capabilities::SelectedCapabilities;
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
    use oteryn_protocol_oteryn::{
        ClientCommandValue, CommandStatus, MessageType, actor_spell, decode_command_result,
        decode_state_delta, decode_wire_envelope, encode_client_command, item_view,
        world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES,
    };
    use tokio::io::AsyncWriteExt;

    let mut requested = vec![
        item_view::CAPABILITY_ITEM_VIEW_MOVE_V1,
        CAPABILITY_WORLD_SPATIAL_ENTITIES,
    ];
    if check_cooldowns {
        requested.push(cooldown_wire::CAPABILITY_PRESENTATION_V1);
    }
    // This fixture starts at the admitted connection boundary. Its canary map
    // uses the legacy floor frame, so install the production presentation offer
    // explicitly when exercising the independently negotiated cooldown domain.
    let offered = crate::gameplay_transport::native_cue_offer::offered(
        owner.offered_capabilities(),
        check_cooldowns,
    );
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(&offered, &requested).ok_or("capability selection")?;
    let actor = admitted.runtime_actor.ok_or("caster")?;
    let before = owner
        .observe_vitals(actor, admitted.game_session_id)
        .await
        .ok_or("vitals")?
        .1;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let driving = async {
        let initial = read_join_cooldowns(&mut client).await?;
        assert_eq!(initial.is_some(), check_cooldowns);
        assert!(initial.is_none_or(|value| value.entries.is_empty()));
        let command_id = admitted.continuity.next_command_id;
        let payload = actor_spell::encode_spell_cast_intent(&intent);
        let cast_command = |command_id| {
            encode_client_command(
                admitted.continuity.connection_generation,
                &ClientCommandValue {
                    command_id,
                    command_type: actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT,
                    payload: &payload,
                },
            )
            .map_err(debug)
        };
        let command = cast_command(command_id)?;
        write_frame(&mut client, &command).await?;
        let mut results = 0;
        let mut inventories = Vec::new();
        let mut mana = Vec::new();
        let mut cooldowns = Vec::new();
        loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            match envelope.message_type() {
                MessageType::CommandResult => {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    if results == 2 {
                        assert_eq!(result.command_id, command_id + 1);
                        assert_eq!(result.status, CommandStatus::Accepted);
                        assert_eq!(
                            actor_spell::decode_spell_cast_result(result.payload).map_err(debug)?,
                            SpellCastDisposition::CoolingDown
                        );
                        break;
                    }
                    assert_eq!(result.command_id, command_id);
                    assert_eq!(
                        result.status,
                        if results == 0 {
                            CommandStatus::Accepted
                        } else {
                            CommandStatus::DuplicateReplay
                        },
                    );
                    assert_eq!(
                        actor_spell::decode_spell_cast_result(result.payload).map_err(debug)?,
                        SpellCastDisposition::Cast
                    );
                    results += 1;
                    if results == 1 {
                        write_frame(&mut client, &command).await?;
                    } else if check_cooldowns {
                        write_frame(&mut client, &cast_command(command_id + 1)?).await?;
                    } else {
                        break;
                    }
                }
                MessageType::StateDelta => {
                    let delta = decode_state_delta(envelope.payload()).map_err(debug)?;
                    match delta.domain_id {
                        cooldown_wire::STATE_DOMAIN_ACTOR_COOLDOWNS => {
                            assert_eq!((delta.base_revision, delta.new_revision), (0, 1));
                            assert_eq!(delta.delta_type, cooldown_wire::DELTA_TYPE_ACTOR_COOLDOWNS_V1);
                            cooldowns.push(
                                cooldown_wire::decode_actor_cooldowns(delta.payload)
                                    .map_err(debug)?,
                            );
                        }
                        item_view::STATE_DOMAIN_CHARACTER_INVENTORY => {
                            inventories.push(
                                item_view::decode_character_inventory(delta.payload)
                                    .map_err(debug)?,
                            );
                        }
                        actor_spell::STATE_DOMAIN_ACTOR_VITALS => {
                            mana.push(
                                actor_spell::decode_actor_vitals(delta.payload)
                                    .map_err(debug)?
                                    .mana,
                            );
                        }
                        _ => (),
                    }
                }
                _ => (),
            }
        }
        client.shutdown().await?;
        if check_cooldowns {
            use cooldown_wire::{ActorCooldown, ActorCooldownKind};
            let [cue] = cooldowns.as_slice() else {
                return Err("expected one committed cooldown delta and none on duplicate replay".into());
            };
            assert_eq!(cue.server_now_ms, CAST_TIME_MS);
            assert_eq!(
                cue.entries,
                [
                    ActorCooldown {
                        kind: ActorCooldownKind::Spell,
                        id: intent.spell.get(),
                        expires_at_ms: cue.server_now_ms + 1_800_000,
                    },
                    ActorCooldown {
                        kind: ActorCooldownKind::Group,
                        id: 3, // S9 Support group.
                        expires_at_ms: cue.server_now_ms + 2_000,
                    },
                ]
            );
        } else {
            assert!(cooldowns.is_empty(), "presentation capability was not selected");
        }
        assert_eq!(mana, [before.mana - 250], "one debit of exactly 250 mana");
        assert_eq!(
            inventories.len(),
            1,
            "the committed spell must publish one inventory delta"
        );
        let [inventory] = inventories.as_slice() else {
            return Err("expected one inventory delta".into());
        };
        assert_eq!(
            inventory
                .entries
                .iter()
                .filter(|item| item.item_definition_ref == wand)
                .map(|item| item.count.get())
                .sum::<u32>(),
            1,
            "one Wand of Darkness in the caster's published backpack"
        );
        Ok(cooldowns.pop())
    };
    let (end, result): (_, TestResult<Option<cooldown_wire::ActorCooldowns>>) =
        tokio::time::timeout(std::time::Duration::from_secs(45), async {
            tokio::join!(serving, driving)
        })
        .await?;
    if let Some(live) = result? {
        let ConnectionEnd::AdmittedThenDisconnected(resumed) = end else {
            return Err("expected disconnect with a resumable session".into());
        };
        owner.states.test_owner_time.store(
            (CAST_TIME_MS + 500) * 1_000 + 1,
            std::sync::atomic::Ordering::Relaxed,
        );
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        let serving = async {
            serve_admitted(&mut server, resumed, owner, IDLE_LIVENESS).await;
            drop(server);
        };
        let driving = async {
            let snapshot = read_join_cooldowns(&mut client)
                .await?
                .ok_or("resume omitted the negotiated cooldown domain")?;
            client.shutdown().await?;
            assert_eq!(snapshot.server_now_ms, live.server_now_ms + 500);
            assert_eq!(snapshot.entries, live.entries, "resume keeps the original deadlines");
            Ok(())
        };
        let ((), result): ((), TestResult) =
            tokio::time::timeout(std::time::Duration::from_secs(45), async {
                tokio::join!(serving, driving)
            })
            .await?;
        result?;
    }
    Ok(())
}

async fn read_join_cooldowns(
    client: &mut tokio::io::DuplexStream,
) -> TestResult<Option<cooldown_wire::ActorCooldowns>> {
    use crate::gameplay_transport::tcp_tls::read_frame;
    use oteryn_protocol_oteryn::{
        MessageType, decode_snapshot_body, decode_snapshot_chunk_framing, decode_wire_envelope,
    };

    let mut body = Vec::new();
    loop {
        let frame = read_frame(client).await?;
        let envelope = decode_wire_envelope(&frame).map_err(debug)?;
        match envelope.message_type() {
            MessageType::SnapshotChunk => {
                let (_, bytes) = decode_snapshot_chunk_framing(envelope.payload()).map_err(debug)?;
                body.extend_from_slice(bytes);
            }
            MessageType::SnapshotCommit => break,
            _ => (),
        }
    }
    let domains = decode_snapshot_body(&body).map_err(debug)?;
    let Some(domain) = domains
        .iter()
        .find(|domain| domain.domain_id == cooldown_wire::STATE_DOMAIN_ACTOR_COOLDOWNS)
    else {
        return Ok(None);
    };
    assert_eq!(domain.revision, 0);
    assert_eq!(domain.snapshot_type, cooldown_wire::SNAPSHOT_TYPE_ACTOR_COOLDOWNS_V1);
    Ok(Some(cooldown_wire::decode_actor_cooldowns(domain.payload).map_err(debug)?))
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
