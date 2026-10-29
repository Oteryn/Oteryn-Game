//! PKG-P1/C16c: admitted USE must reach the durable companion owner.
// Global library text: docs/reference/spells/r21-local-candidate/bundles/rune-convince_creature_rune/spell.json:10
// Convince charges creature-dependent mana in addition to rune creation.
// docs/reference/spells/r21-local-candidate/bundles/rune-animate_dead_rune/spell.json:10
// Animate produces an owned undead from a body; :29 has no rune-use mana cost.
// Creation fees are separate: docs/reference/spells/r22-audit/tibiopedia/tibiopedia-spell-facts-2026-10-02.json:928
// (Convince) and :1988 (Animate). Source-qualified profiles retain the two-summon
// cap (both spell.json:75) and movable real-corpse rules (Animate spell.json:89).
use super::death_ml_loss_composed::{
    current_fence, production_generation_with_pin, production_generation_with_source,
};
use super::fixture::{Harness, debug, id, join_two, run};
use super::item_look::{large_stack, read_frame, seed_main_backpack};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildOccurrence, DurableBuildState,
    convert_vocation,
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{
    ChannelContentPin, CharacterLease, MessageType, MovementLocalPosition, decode_wire_envelope,
};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, RUNE_USE_OFFERED, SelectedCapabilities, test_command,
};
use crate::gameplay_transport::connection::{
    ConnectionEnd, FirstEntryOutcome, IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities;
use crate::gameplay_transport::{SeamRooms, item_fence_of};
use crate::spell::cast::CharacterCastFacts;
use oteryn_protocol_oteryn::actor_spell::SpellTargetPosition;
use oteryn_protocol_oteryn::item_view::{
    STATE_DOMAIN_CHARACTER_INVENTORY, decode_character_inventory,
};
use oteryn_protocol_oteryn::world_object::{
    COMMAND_TYPE_USE_INTENT, UseDisposition, UseIntent, UseTarget, decode_use_result,
    encode_item_use_intent,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use oteryn_protocol_oteryn::{CommandStatus, decode_command_result, decode_snapshot_chunk};
use tokio::io::AsyncWriteExt;

const CASTER: MovementLocalPosition = MovementLocalPosition {
    x: 100,
    y: 100,
    floor: 7,
};
const VICTIM: MovementLocalPosition = MovementLocalPosition {
    x: 101,
    y: 100,
    floor: 7,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Case {
    Convince,
    NotConvinceable,
    SummonLimit,
    Animate,
    AnimateBlackSkull,
    NonCorpse,
}

#[test]
fn utility_runes_convince_real_dispatch_assigns_master_and_charges_creature_mana() -> TestResult {
    scenario(Case::Convince)
}
#[test]
fn utility_runes_convince_real_dispatch_refuses_non_convinceable() -> TestResult {
    scenario(Case::NotConvinceable)
}
#[test]
fn utility_runes_convince_real_dispatch_refuses_summon_limit() -> TestResult {
    scenario(Case::SummonLimit)
}
#[test]
fn utility_runes_animate_real_dispatch_owns_skeleton_and_consumes_corpse() -> TestResult {
    scenario(Case::Animate)
}
#[test]
fn utility_runes_animate_real_dispatch_refuses_non_corpse() -> TestResult {
    scenario(Case::NonCorpse)
}

#[test]
fn utility_runes_animate_real_dispatch_refuses_black_skull() -> TestResult {
    scenario(Case::AnimateBlackSkull)
}

fn scenario(case: Case) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(move || {
            run(async move |admin| run_case(admin, case).await).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "utility rune scenario panicked")?
        .map_err(Into::into)
}

async fn run_case(admin: String, case: Case) -> TestResult {
    let animate = matches!(
        case,
        Case::Animate | Case::AnimateBlackSkull | Case::NonCorpse
    );
    let committed = matches!(case, Case::Convince | Case::Animate);
    let harness = Harness::create_with_admission_seed(admin, "utility_runes", true, false).await?;
    prepare_current_session(&harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
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
    let active = controller.active().ok_or("active generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let entry_room = pin.qualified_room();
    let map = crate::map::spell_tiles::tests::movement_arena::world();
    let cells = map
        .movement_cells(entry_room.movement_cells())
        .map_err(debug)?;
    let room = entry_room
        .bundle_view(
            cells.clone(),
            map.frame_binding_digest(world),
            map.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: CASTER.x,
                y: CASTER.y,
                floor: CASTER.floor,
            },
        )
        .ok_or("bundle utility rune room")?;
    let start = CASTER;
    let target_position = VICTIM;
    let channel_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let dependencies = Dependencies::with_content_and_capacity(
        channel_pin.clone(),
        entry_room.clone(),
        native.spell_book().clone(),
        8,
    )?;
    let (actor, session) = (dependencies.actor, dependencies.admitted.game_session_id);
    let victim;
    let target_actor;
    let convince_mana;
    {
        let mut runtime = dependencies.runtime.lock().await;
        // Keep the selected creature ordinary across the Forge population tick.
        let mut policy = native
            .policies()
            .policies()
            .iter()
            .find(|policy| policy.definition_key == "oteryn:creature.rat")
            .ok_or("target creature policy")?
            .as_ref()
            .clone();
        policy.forge_difficulty = None;

        if case == Case::NotConvinceable {
            policy = native
                .policies()
                .policies()
                .iter()
                .find(|p| p.definition_key == "oteryn:creature.dragon")
                .ok_or("non-convinceable Dragon")?
                .as_ref()
                .clone();
            policy.forge_difficulty = None;
            assert!(!policy.convinceable);
        }
        let target_key = policy.definition_key.clone();
        let skeleton = native
            .policies()
            .policies()
            .iter()
            .find(|p| p.definition_key == "oteryn:creature.skeleton")
            .ok_or("Skeleton policy")?
            .as_ref()
            .clone();
        let policies =
            crate::content::creature_policy::CompiledCreaturePolicies::from_active_artifact(
                native.source_digest(),
                vec![policy, skeleton],
            )
            .map_err(debug)?;
        runtime
            .install_companion_policies(policies)
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
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        runtime
            .realize_native_qualification_spawn(channel_pin, &target_key, target_position)
            .map_err(debug)?;
        let creature = runtime.visible_entities().creatures[0];
        target_actor = creature.actor;
        convince_mana = runtime
            .companion_snapshot(target_actor)
            .map_err(debug)?
            .state
            .policy
            .mana_cost
            .unwrap_or(0);
        if case == Case::SummonLimit {
            for x in [102, 103] {
                runtime
                    .create_companion(
                        actor,
                        session,
                        "oteryn:creature.rat",
                        MovementLocalPosition {
                            x,
                            y: 100,
                            floor: 7,
                        },
                        None,
                        None,
                        0,
                    )
                    .map_err(debug)?;
            }
        }
        victim = EntityRef {
            identity: creature.actor.placement_identity(),
            generation: creature.generation,
        };
    }
    let mut owner = dependencies.owner(&harness, &authority);
    owner.emit = |_, line| {
        eprintln!(
            "utility rune owner: {} {} {:?}",
            line.module, line.event, line.code
        )
    };
    owner.active_generation = Some(active);
    owner.required_capabilities = crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
        &[11, 30], crate::foundation::RuntimeScopeRefV1::channel(world, dependencies.admitted.channel_id),
        1, "policy-1", BUNDLE_WORLD_OFFERED_CAPABILITIES).map_err(debug)?;
    owner.rooms = SeamRooms::new(None, Some(&room));
    owner.movement_cells = &cells;
    let mut admitted = dependencies.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let (current, _) = FreshAdmissionStore::from_root(harness.root.clone())
        .current_session_at(session)
        .await?;
    admitted.item_fence = item_fence_of(current);
    let formula = native.training_formula().ok_or("training formula")?;
    let druid = convert_vocation(formula, &DurableBuildState::default(), "druid").map_err(debug)?;
    assert!(matches!(
        harness
            .root
            .commit_character_build(
                &authority,
                &harness.node,
                current_fence(&owner, session).await?,
                BuildChangeRequest {
                    occurrence: BuildOccurrence::from_bytes(id(90)).map_err(debug)?,
                    cause: BuildCause::VocationChoice,
                    before: DurableBuildState::default(),
                    after: druid.clone(),
                    pruned_stance: None,
                },
                formula
            )
            .await
            .map_err(debug)?,
        BuildCommitOutcome::Committed(_)
    ));
    let trained =
        DurableBuildState::new(druid.vocation(), (10, 0), druid.skills()).map_err(debug)?;
    harness
        .root
        .commit_character_build(
            &authority,
            &harness.node,
            current_fence(&owner, session).await?,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(91)).map_err(debug)?,
                cause: BuildCause::Training,
                before: druid,
                after: trained.clone(),
                pruned_stance: None,
            },
            formula,
        )
        .await
        .map_err(debug)?;
    {
        let runtime = dependencies.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        assert!(
            states
                .initialize(
                    &runtime,
                    actor,
                    session,
                    CharacterCastFacts {
                        level: 50,
                        max_mana: 2000,
                        magic_level: 10,
                        ..crate::gameplay_transport::actor_spell::tests::FACTS
                    },
                    (0, 0),
                    owner.owner_now()
                )
                .is_some()
        );
        assert!(states.load_owned_build(&runtime, actor, session, &trained));
        states
            .get_mut(&runtime, actor, session)
            .ok_or("present caster")?
            .enable_owned_training(&trained, formula)
            .map_err(debug)?;
    }

    admitted.continuity.selected_capabilities = owner
        .required_capabilities
        .authenticate_support(&[4, 6, 11, 15, 18, 22, 30, 91])
        .map_err(debug)?
        .selected();
    // Fixture data only. Admission, source policy, caster facts and payment below
    // are read by the real current owners, with all normal runtime fences.
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
        .bind(id(41).as_slice()).execute(&harness.pool).await?;
    let source_key = if animate {
        "oteryn:item.tibia.i3203"
    } else {
        "oteryn:item.tibia.i3177"
    };
    seed_main_backpack(&harness, &[(source_key, 3)]).await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(room.frame_binding().digest().as_slice()).execute(&harness.pool).await?;

    harness
        .root
        .initialize_familiar_group(
            &authority,
            &harness.node,
            current_fence(&owner, session).await?,
        )
        .await
        .map_err(debug)?;
    if animate {
        sqlx::query("INSERT INTO game_control_field_policy_grants VALUES(session_user,encode($1,'hex')::uuid,'policy-1')")
            .bind(id(42).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_spell_field_world_policies(world_id,world_policy_revision,world_type,protection_level,in_fight_ms,source_pin,control_role,decision_identity) VALUES(encode($1,'hex')::uuid,'policy-1',1,20,60000,'99902524e052f37574194466c2949c576e4ab269',session_user,'utility-runes-fixture')")
            .bind(id(42).as_slice()).execute(&harness.pool).await?;
        if case == Case::AnimateBlackSkull {
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO game_character_pvp_state(character_id,world_id,skull,skull_until) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,2,clock_timestamp()+interval '1 hour')")
                .bind(id(41).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
        }
    }
    if animate {
        // Free the tile used by the Convince fixture. Corpse custody is independently durable.
        let mut runtime = owner.runtime.lock().await;
        let before = runtime.read_actor_position(target_actor).map_err(debug)?;
        runtime
            .borrow_movement_position()
            .commit_cardinal(
                before,
                MovementLocalPosition {
                    x: 102,
                    y: 100,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        let address =
            crate::durability::native_world_items::SpellGroundTarget::for_native_tile_read(
                &room, &runtime, VICTIM,
            )
            .map_err(debug)?;
        drop(runtime);
        seed_target(&harness, native, &address, case != Case::NonCorpse).await?;
    }
    let before = {
        let runtime = owner.runtime.lock().await;
        owner
            .states
            .spell_states
            .lock()
            .await
            .get(&runtime, actor, session)
            .ok_or("caster")?
            .vitals()
    };
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS).await;
        drop(server);
        assert!(
            matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
            "utility rune connection ended before client shutdown"
        );
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let casting = async {
        let mut source = None;
        loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::SnapshotChunk {
                for domain in decode_snapshot_chunk(envelope.payload()).map_err(debug)?.1 {
                    if domain.domain_id == STATE_DOMAIN_CHARACTER_INVENTORY {
                        source = decode_character_inventory(domain.payload)
                            .map_err(debug)?
                            .entries
                            .first()
                            .map(|e| e.handle);
                    }
                }
            }
            if envelope.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        let intent = UseIntent {
            target: UseTarget::Item(source.ok_or("rune handle")?),
            use_with: if animate { None } else { Some(victim) },
            use_with_map_item: None,
            use_with_position: if animate {
                Some(SpellTargetPosition {
                    x: VICTIM.x,
                    y: VICTIM.y,
                    floor: VICTIM.floor,
                })
            } else {
                None
            },
        };
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        let frame = test_command(
            admitted.continuity.next_command_id,
            COMMAND_TYPE_USE_INTENT,
            &payload,
        );
        client.write_u32(u32::try_from(frame.len())?).await?;
        client.write_all(&frame).await?;
        loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() != MessageType::CommandResult {
                continue;
            }
            let result = decode_command_result(envelope.payload()).map_err(debug)?;
            let disposition = decode_use_result(result.payload).map_err(debug)?;
            let expected = if committed {
                UseDisposition::Committed
            } else if animate {
                // Refusal diagnostics reject an Item whose collision origin is
                // unavailable; neither outcome may pay or allocate a summon.
                UseDisposition::Rejected
            } else {
                UseDisposition::TargetIllegal
            };
            assert_eq!(disposition, expected, "{case:?}");
            assert_eq!(
                result.status,
                if expected == UseDisposition::Rejected {
                    CommandStatus::Rejected
                } else {
                    CommandStatus::Accepted
                }
            );
            break;
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let (served, cast) = join_two(serving, casting).await;
        served?;
        cast
    })
    .await??;
    let runtime = owner.runtime.lock().await;
    let after = owner
        .states
        .spell_states
        .lock()
        .await
        .get(&runtime, actor, session)
        .ok_or("caster")?
        .vitals();
    assert_eq!(
        after.mana,
        before.mana
            - if case == Case::Convince {
                convince_mana
            } else {
                0
            }
    );
    let owned = runtime
        .positioned_actor_census()
        .map_err(debug)?
        .iter()
        .filter(|(_, _, s)| s.is_none())
        .filter_map(|(a, _, _)| runtime.companion_snapshot(*a).ok())
        .filter(|s| {
            s.state
                .master
                .is_some_and(|m| m.actor == actor && m.session == session)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        owned.len(),
        if case == Case::SummonLimit {
            2
        } else {
            usize::from(committed)
        }
    );
    if committed {
        assert_eq!(
            owned[0].state.policy.definition_key,
            if animate {
                "oteryn:creature.skeleton"
            } else {
                "oteryn:creature.rat"
            }
        );
        if !animate {
            assert_eq!(owned[0].actor, target_actor);
        }
    }
    drop(runtime);
    let quantity: i64 = sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(71).as_slice())
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(quantity, if committed { 2 } else { 3 });
    if animate {
        let live: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(180).as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(live, i64::from(!committed));
        let body: (i16, i64) = sqlx::query_as("SELECT lifecycle,quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(id(180).as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(body, if committed { (2, 0) } else { (1, 1) });
    }
    drop(owner);
    drop(authority);
    drop(seal);
    harness.cleanup().await
}

async fn seed_target(
    h: &Harness,
    native: &crate::content::native_gameplay::NativeGameplayState,
    target: &crate::durability::native_world_items::SpellGroundTarget,
    corpse: bool,
) -> TestResult {
    let creature = native
        .creature_profiles()
        .records
        .iter()
        .find(|r| r.profile.target.key == "oteryn:creature.goldhanded_cultist")
        .ok_or("qualified corpse creature")?;
    let crate::content::ProjectV2AuthoringProfileData::Creature(profile) = &creature.profile.data
    else {
        return Err("creature profile".into());
    };
    let corpse_ref = profile
        .details
        .as_ref()
        .and_then(|d| d.corpse_item.as_ref())
        .ok_or("creature corpse")?;
    let policy = native
        .item_policy(&corpse_ref.key, &corpse_ref.revision)
        .ok_or("movable creature corpse policy")?;
    let definition = &policy.record().production_definition;
    let qualified =
        crate::durability::native_world_items::QualifiedItemDefinition::from_native_policy(policy)
            .map_err(debug)?;
    assert!(qualified.movable, "the actual creature corpse is movable");
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$4,$5,1,1,encode($3,'hex')::uuid)")
        .bind(id(180).as_slice()).bind(id(42).as_slice()).bind(id(181).as_slice()).bind(&definition.production_key).bind(&definition.revision_ref).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,$4,$5,$6,$7,$8,100)")
        .bind(id(180).as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice()).bind(&target.spatial_position).bind(id(182).as_slice()).bind(&target.map_revision).bind(&target.content_revision).bind(&target.placement_context).execute(&mut *tx).await?;
    if corpse {
        sqlx::query("INSERT INTO game_item_mint_receipts(death_world_id,death_channel_id,death_scope_ownership_generation,death_actor_local_id,death_actor_local_generation,loot_table_family,loot_table_production_key,loot_table_revision_ref,loot_purpose_key,draw_ordinal,intent_binding,transaction_id,event_id,item_instance_id,occurred_at,envelope_sha256,committed_at,corpse_top_damage_character_id,materialized_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,10,1,'LootTable','fixture:cultist','r1','CORPSE_MATERIALIZATION',0,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,1,$7,1,encode($8,'hex')::uuid,1)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(vec![1_u8;33]).bind(id(181).as_slice()).bind(id(183).as_slice()).bind(id(180).as_slice()).bind(vec![1_u8;32]).bind(id(41).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

// Reference parity: docs/reference/spells/utility-runes-2026-10-29.md:9 (200 s),
// :14 (movable items/PZ) and :17 (other players' houses). Global snapshots: docs/reference/spells/r22-audit/tibiopedia/mechanics-single-facts.json:864
// and :1124 explicitly leave duration/target exceptions unverified. Crystal 201189f8
// data/scripts/runes/chameleon.lua:4,19 supplies 200 s and movable-only targeting;
// desintegrate_rune.lua:13 supplies movable/tag/corpse exclusions. Global PZ evidence:
// https://tibia.fandom.com/wiki/Disintegrate_Rune (support rune; allowed in PZ).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorldItemCase {
    Chameleon,
    Creature,
    Empty,
    Immovable,
    Disintegrate,
    Pz,
    House,
}

#[test]
fn utility_runes_postgres_chameleon_item_outfit_lasts_200_seconds() -> TestResult {
    large_stack(|| run(async |admin| utility_case(admin, WorldItemCase::Chameleon).await))
}
#[test]
fn utility_runes_postgres_chameleon_refuses_creature_on_movable_item_tile() -> TestResult {
    large_stack(|| run(async |admin| utility_case(admin, WorldItemCase::Creature).await))
}
#[test]
fn utility_runes_postgres_chameleon_refuses_empty_and_immovable_targets() -> TestResult {
    large_stack(|| {
        run(async |admin| {
            for case in [WorldItemCase::Empty, WorldItemCase::Immovable] {
                utility_case(admin.clone(), case).await?;
            }
            Ok(())
        })
    })
}
#[test]
fn utility_runes_postgres_disintegrate_removes_movable_items_and_allows_pz() -> TestResult {
    large_stack(|| {
        run(async |admin| {
            for case in [WorldItemCase::Disintegrate, WorldItemCase::Pz] {
                utility_case(admin.clone(), case).await?;
            }
            Ok(())
        })
    })
}
#[test]
fn utility_runes_postgres_disintegrate_refuses_house_target_without_payment() -> TestResult {
    large_stack(|| run(async |admin| utility_case(admin, WorldItemCase::House).await))
}

async fn utility_case(admin: String, case: WorldItemCase) -> TestResult {
    tokio::time::timeout(
        std::time::Duration::from_secs(120),
        utility_inner(admin, case),
    )
    .await?
}
async fn utility_inner(admin: String, case: WorldItemCase) -> TestResult {
    let rune = if matches!(
        case,
        WorldItemCase::Disintegrate | WorldItemCase::Pz | WorldItemCase::House
    ) {
        3197
    } else {
        3178
    };
    let harness = Harness::create_with_admission_seed(admin, "utility_runes", true, false).await?;
    prepare_current_session(&harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let source = include_bytes!(
        "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
    );
    let (controller, pin) = if case == WorldItemCase::House {
        production_generation_with_pin(world, "content-1")?
    } else {
        production_generation_with_source(world, "content-1", Some(source))?
    };
    let active = controller.active().ok_or("active generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let entry_room = pin.qualified_room();
    let map = if case == WorldItemCase::House {
        Some(house_world(world, native)?)
    } else {
        None
    };
    let cells = if let Some(map) = &map {
        map.movement_cells(entry_room.movement_cells())
            .map_err(debug)?
    } else {
        entry_room.movement_cells().clone()
    };
    let (start, target) = match case {
        WorldItemCase::House => ((2, 0, 7), (3, 0, 7)),
        WorldItemCase::Empty => ((2, 11, -7), (3, 11, -7)),
        WorldItemCase::Pz => ((2, 0, -7), (1, 0, -7)),
        WorldItemCase::Immovable => ((2, 11, -7), (1, 11, -7)),
        _ => ((13, 11, -7), (12, 11, -7)),
    };
    let start = MovementLocalPosition {
        x: start.0,
        y: start.1,
        floor: start.2,
    };
    let target = SpellTargetPosition {
        x: target.0,
        y: target.1,
        floor: target.2,
    };
    let room = if let Some(map) = &map {
        entry_room
            .bundle_view(
                cells.clone(),
                map.frame_binding_digest(world),
                map.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: start.x,
                    y: start.y,
                    floor: start.floor,
                },
            )
            .ok_or("bundle room")?
    } else {
        entry_room.clone()
    };
    let channel_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let dependencies = Dependencies::with_content(
        channel_pin.clone(),
        entry_room.clone(),
        native.spell_book().clone(),
    )?;
    let (actor, session) = (dependencies.actor, dependencies.admitted.game_session_id);
    let creature_target = {
        let mut runtime = dependencies.runtime.lock().await;
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
        if case == WorldItemCase::Creature {
            runtime
                .install_companion_policies(native.policies().clone())
                .map_err(debug)?;
            runtime
                .realize_native_qualification_spawn(
                    channel_pin,
                    "oteryn:creature.rat",
                    MovementLocalPosition {
                        x: target.x,
                        y: target.y,
                        floor: target.floor,
                    },
                )
                .map_err(debug)?;
            let rat = runtime.visible_entities().creatures[0];
            Some(oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                identity: rat.actor.placement_identity(),
                generation: rat.generation,
            })
        } else {
            None
        }
    };
    let mut owner = dependencies.owner(&harness, &authority);
    owner.active_generation = Some(active);
    owner.rooms = SeamRooms::new(Some(&room), Some(&room));
    owner.movement_cells = &cells;
    if case == WorldItemCase::House {
        owner.required_capabilities = BoundRequiredCapabilities::for_channel_boot(
            &[11, 30],
            crate::foundation::RuntimeScopeRefV1::channel(world, dependencies.admitted.channel_id),
            1,
            "policy-1",
            BUNDLE_WORLD_OFFERED_CAPABILITIES,
        )
        .map_err(debug)?;
    }
    let formula = native.training_formula().ok_or("training formula")?;
    let druid = convert_vocation(formula, &DurableBuildState::default(), "druid").map_err(debug)?;
    harness
        .root
        .commit_character_build(
            &authority,
            &harness.node,
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
    // Use the real build writer and reload its durable magic level before USE.
    let trained =
        DurableBuildState::new(druid.vocation(), (10, 0), druid.skills()).map_err(debug)?;
    harness
        .root
        .commit_character_build(
            &authority,
            &harness.node,
            current_fence(&owner, session).await?,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(91)).map_err(debug)?,
                cause: BuildCause::Training,
                before: druid,
                after: trained.clone(),
                pruned_stance: None,
            },
            formula,
        )
        .await
        .map_err(debug)?;
    {
        let runtime = dependencies.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        assert!(
            states
                .initialize(
                    &runtime,
                    actor,
                    session,
                    CharacterCastFacts {
                        level: 50,
                        magic_level: 10,
                        ..crate::gameplay_transport::actor_spell::tests::FACTS
                    },
                    (0, 0),
                    owner.owner_now()
                )
                .is_some()
        );
        assert!(states.load_owned_build(&runtime, actor, session, &trained));
        states
            .get_mut(&runtime, actor, session)
            .ok_or("caster")?
            .enable_owned_training(&trained, formula)
            .map_err(debug)?;
    }
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
        .bind(id(41).as_slice()).execute(&harness.pool).await?;

    let rune_key = format!("oteryn:item.tibia.i{rune}");
    seed_main_backpack(&harness, &[(&rune_key, 3)]).await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice())
        .bind(pin.identity().client_artifact_digest().as_slice()).bind(room.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
    let mut admitted = dependencies.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let (current, _) = FreshAdmissionStore::from_root(harness.root.clone())
        .current_session_at(session)
        .await?;
    admitted.item_fence = item_fence_of(current);
    admitted.continuity.selected_capabilities = if case == WorldItemCase::House {
        owner
            .required_capabilities
            .authenticate_support(&[4, 6, 11, 15, 18, 22, 30])
            .map_err(debug)?
            .selected()
    } else {
        SelectedCapabilities::select(RUNE_USE_OFFERED, &[4, 6, 15, 22])
            .ok_or("rune capabilities")?
    };
    if case != WorldItemCase::House {
        assert!(
            owner.ensure_source_map_initialized().await,
            "captured source map is ready before dispatch"
        );
    }
    let ground_target = {
        let runtime = dependencies.runtime.lock().await;
        crate::durability::native_world_items::SpellGroundTarget::for_native_tile_read(
            &room,
            &runtime,
            MovementLocalPosition {
                x: target.x,
                y: target.y,
                floor: target.floor,
            },
        )
        .map_err(debug)?
    };
    if case == WorldItemCase::Disintegrate {
        let quantity:i64=sqlx::query_scalar("SELECT i.quantity FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE g.spatial_position=$1 AND i.definition_production_key='oteryn:item.tibia.i3446' AND i.lifecycle=1 LIMIT 1").bind(&ground_target.spatial_position).fetch_one(&harness.pool).await?;
        assert_eq!(
            quantity, 100,
            "captured target starts with 100 real movable arrows"
        );
    }
    let accepted = matches!(
        case,
        WorldItemCase::Chameleon | WorldItemCase::Disintegrate | WorldItemCase::Pz
    );
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(
            &mut server,
            admitted,
            &owner,
            crate::gameplay_transport::connection::IDLE_LIVENESS,
        )
        .await;
        assert!(
            matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
            "{end:?}"
        );
        drop(server);
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let casting = async {
        let mut handle = None;
        loop {
            let bytes = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
            if envelope.message_type() == MessageType::SnapshotChunk {
                for domain in decode_snapshot_chunk(envelope.payload()).map_err(debug)?.1 {
                    if domain.domain_id == STATE_DOMAIN_CHARACTER_INVENTORY {
                        handle = decode_character_inventory(domain.payload)
                            .map_err(debug)?
                            .entries
                            .first()
                            .map(|e| e.handle);
                    }
                }
            }
            if envelope.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        let payload = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(handle.ok_or("rune handle")?),
            use_with: creature_target,
            use_with_map_item: None,
            use_with_position: if creature_target.is_some() {
                None
            } else {
                Some(target)
            },
        })
        .map_err(debug)?;
        let bytes = test_command(
            admitted.continuity.next_command_id,
            COMMAND_TYPE_USE_INTENT,
            &payload,
        );
        client.write_u32(u32::try_from(bytes.len())?).await?;
        client.write_all(&bytes).await?;
        loop {
            let bytes = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
            if envelope.message_type() == MessageType::CommandResult {
                let result = decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(
                    decode_use_result(result.payload).map_err(debug)?,
                    if accepted {
                        UseDisposition::Committed
                    } else {
                        UseDisposition::TargetIllegal
                    },
                    "{case:?}"
                );
                break;
            }
        }
        let quantity:i64=sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(
            quantity,
            if accepted { 2 } else { 3 },
            "atomic rune debit {case:?}"
        );
        if case == WorldItemCase::Chameleon {
            use oteryn_gameplay_foundation::foundation::condition::TemporaryDisplayedAppearance;
            let runtime = dependencies.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let conditions = states
                .get(&runtime, actor, session)
                .ok_or("caster")?
                .owned_conditions();
            let instance = conditions
                .instances()
                .iter()
                .find(|c| c.definition().source_item_appearance().is_some())
                .ok_or("real rune installs item outfit")?;
            let start = instance.started_at();
            assert_eq!(instance.remaining_duration_ms(start), Some(200_000));
            for at in [start, start + 199_999_999] {
                let Some(TemporaryDisplayedAppearance::Item(item)) =
                    conditions.displayed_temporary_appearance_at(at)
                else {
                    return Err("item outfit ended early".into());
                };
                assert_eq!(item.definition_key(), "oteryn:item.tibia.i3446");
                assert_eq!(item.revision_ref(), "definition-r1");
                assert_eq!(
                    item.artifact_digest(),
                    pin.identity().server_artifact_digest()
                );
            }
            assert_eq!(
                conditions.displayed_temporary_appearance_at(start + 200_000_000),
                None
            );
        }
        if case == WorldItemCase::Disintegrate {
            let left:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE g.spatial_position=$1 AND i.lifecycle=1").bind(&ground_target.spatial_position).fetch_one(&harness.pool).await?;
            assert_eq!(left, 0, "the movable stack removed durably");
            let removed:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3446' AND lifecycle<>1").fetch_one(&harness.pool).await?;
            assert_eq!(
                removed, 1,
                "the real dispatch retired all 100 arrows in the target stack"
            );
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (served, cast) = join_two(serving, casting).await;
    served?;
    cast?;
    if case == WorldItemCase::Chameleon {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(201))
            .ok_or("owner clock")?;
        let runtime = dependencies.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        assert_eq!(
            states
                .get(&runtime, actor, session)
                .ok_or("caster after duration")?
                .owned_conditions()
                .displayed_temporary_appearance_at(owner.owner_now().get()),
            None
        );
    }
    drop(owner);
    drop(authority);
    drop(seal);
    harness.cleanup().await
}
fn house_world(
    world: crate::foundation::WorldId,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    let reference = |key: &str| {
        native
            .item_index()?
            .definition_ref("Item", key, "definition-r1")
    };
    let (bytes, mut pins) = crate::map::boot::tests::bundle_with_item_ids(
        (
            "oteryn:item.tibia.i2854",
            reference("oteryn:item.tibia.i2854")
                .ok_or("backpack")?
                .get()
                - 1,
        ),
        (
            "oteryn:item.tibia.i3607",
            reference("oteryn:item.tibia.i3607").ok_or("food")?.get() - 1,
        ),
    );
    let mut bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    // Combat reads require the same source-qualified ground as the field-rune fixtures.
    bundle.manifest.palette[0].key = "oteryn:item.tibia.i100".into();
    bundle.manifest.palette[0]
        .terrain
        .as_mut()
        .ok_or("ground")?
        .ground_speed = Some(150);
    let mut house = bundle.sectors[0]
        .tiles
        .iter()
        .find(|t| t.x == 7)
        .ok_or("item tile")?
        .clone();
    let sign_palette = u32::try_from(bundle.manifest.palette.len())?;
    bundle
        .manifest
        .palette
        .push(oteryn_world_bundle::bundle::PaletteEntry {
            family: oteryn_world_bundle::bundle::Family::Item,
            key: "oteryn:item.tibia.i2012".into(),
            id: reference("oteryn:item.tibia.i2012").ok_or("sign")?.get() - 1,
            terrain: None,
        });
    house.items.truncate(1);
    house.items.push(oteryn_world_bundle::sector::Item {
        palette: sign_palette,
        depth: 0,
        attrs: Default::default(),
    });
    house.x = 3;
    house.house = 1;
    house.flags = 1;
    bundle.sectors[0].tiles.retain(|t| t.x != 3);
    bundle.sectors[0].tiles.push(house);
    bundle.sectors[0].tiles.sort_by_key(|t| (t.y, t.x));
    let bytes = oteryn_world_bundle_compiler::bundle::write(
        &bundle.manifest,
        &bundle.sectors,
        &bundle.spawns,
    )?;
    pins.digest = oteryn_world_bundle::bundle::read(&bytes)
        .map_err(debug)?
        .digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(&bytes,&crate::map::boot::BootPins {
        bundle:pins,map_revision:crate::map::overlay::map_revision(&base),start:crate::map::overlay::TilePos{x:2,y:0,floor:-7},
    },world,crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,|key| {
        let profile=native.item_policy(key,"definition-r1")?.record();
        Some(crate::map::facts::ItemDefinition{
            reference:reference(key)?,solid:profile.attributes.blocks_movement,
            blocks_projectile:profile.attributes.blocks_projectile.unwrap_or(true),
            pickupable:matches!(&profile.semantics.physical,crate::content::ReferenceItemField::Known(p) if p.pickupable==crate::content::ReferenceItemField::Known(true)),door:false,
        })
    }).map_err(debug)
}
