//! FIELD-RUNES-1: real USE -> authored geometry -> atomic charge/field writer.
use super::death_ml_loss_composed::{current_fence, production_generation_with_pin};
use super::fixture::{Harness, debug, id, join_two, run};
use super::item_look::{large_stack, read_frame, seed_main_backpack};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{
    ChannelContentPin, CharacterLease, MessageType, MovementLocalPosition, decode_wire_envelope,
};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, test_command,
};
use crate::gameplay_transport::connection::{
    ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, LivenessPolicy, serve_admitted,
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
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};
use oteryn_protocol_oteryn::{CommandStatus, decode_command_result, decode_snapshot_chunk};
use tokio::io::AsyncWriteExt;

#[test]
fn field_runes_all_profiles_use_the_existing_caster() -> TestResult {
    large_stack(|| {
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, _) = production_generation_with_pin(world, "content-1")?;
        let active = controller.active().ok_or("active generation")?;
        let native = active.native_gameplay().ok_or("native gameplay")?;
        for id in [
            3148, 3149, 3156, 3164, 3166, 3172, 3173, 3176, 3180, 3188, 3190, 3192,
        ] {
            let spell = native.spell_book().rune(id).ok_or("activated field rune")?;
            assert!(
                oteryn_gameplay_spell_rules::dispatch_plans::rune_use::direct_rune(spell),
                "{id}"
            );
        }
        for (id, flags) in [
            (2128, (true, true)),
            (2130, (true, false)),
            (10181, (false, true)),
            (10182, (false, false)),
        ] {
            let policy = native
                .item_policy_for_source_id(id)
                .ok_or("barrier policy")?;
            let key = &policy.record().production_definition;
            assert!(
                native
                    .item_index()
                    .ok_or("item index")?
                    .definition_ref(&key.family, &key.production_key, &key.revision_ref)
                    .is_some(),
                "barrier {id} {:?} indexed revision {:?}",
                key,
                native
                    .item_index()
                    .and_then(|index| index.revision(&key.production_key))
            );
            assert_eq!(
                oteryn_gameplay_items::native_world_items::source_barrier_collision(
                    native
                        .item_policy_for_source_id(id)
                        .ok_or("barrier policy")?
                ),
                Some(flags)
            );
        }
        Ok(())
    })
}

#[test]
fn field_runes_postgres_player_contact_ticks_and_incidental_skull() -> TestResult {
    large_stack(|| {
        run(async |admin| {
            let mut reached = crate::gameplay_transport::field_removal::faults::arm(
                crate::gameplay_transport::field_removal::faults::PLAYER,
                false,
            );
            field_case(admin, 3188, 1, 2118, true, false, false).await?;
            assert_eq!(
                reached.try_recv(),
                Ok(()),
                "the real field writer reached the requested COMMIT interruption"
            );
            Ok(())
        })
    })
}

#[test]
fn field_runes_postgres_player_contact_recovers_committed_damage() -> TestResult {
    large_stack(|| {
        run(async |admin| {
            let mut reached = crate::gameplay_transport::field_removal::faults::arm(
                crate::gameplay_transport::field_removal::faults::PLAYER,
                true,
            );
            field_case(admin, 3188, 1, 2118, true, false, false).await?;
            assert_eq!(reached.try_recv(), Ok(()));
            Ok(())
        })
    })
}

#[test]
fn field_runes_postgres_safe_wall_unknown_commit_never_repeats_movement() -> TestResult {
    large_stack(|| {
        run(async |admin| {
            for after in [false, true] {
                let mut reached = crate::gameplay_transport::field_removal::faults::arm(
                    crate::gameplay_transport::field_removal::faults::REMOVAL,
                    after,
                );
                field_case(admin.clone(), 3180, 1, 10181, true, false, false).await?;
                assert_eq!(reached.try_recv(), Ok(()));
            }
            Ok(())
        })
    })
}

#[test]
fn field_runes_postgres_safe_fire_stays_safe_after_creator_loss_through_decay() -> TestResult {
    large_stack(|| run(async |admin| field_case(admin, 3188, 1, 21465, true, false, true).await))
}

#[test]
fn field_runes_postgres_real_dispatch_casts_fields_walls_and_bombs() -> TestResult {
    large_stack(|| {
        run(async |admin| {
            for (rune, count, item) in [
                (3188, 1, 21465),
                (3164, 1, 2135),
                (3172, 1, 2134),
                (3190, 5, 21465),
                (3166, 7, 2135),
                (3176, 5, 2134),
                (3192, 9, 21465),
                (3149, 9, 2135),
                (3173, 9, 2134),
            ] {
                field_case(admin.clone(), rune, count, item, true, false, false).await?;
            }
            Ok(())
        })
    })
}

#[test]
fn field_runes_postgres_real_dispatch_casts_blocking_and_safe_barriers() -> TestResult {
    large_stack(|| {
        run(async |admin| {
            for (rune, count, item) in [
                (3180, 1, 10181),
                (3156, 1, 10182),
                (3180, 1, 2128),
                (3156, 1, 2130),
            ] {
                field_case(admin.clone(), rune, count, item, true, false, false).await?;
            }
            Ok(())
        })
    })
}

#[test]
fn field_runes_postgres_secret_agent_tool_removes_wild_growth() -> TestResult {
    large_stack(|| {
        run(async |admin| {
            field_scenario(admin, 3156, 1, 2130, true, false, false, Scenario::SecretMachete)
                .await
        })
    })
}

#[test]
fn field_runes_postgres_missing_extension_cannot_pay_or_create() -> TestResult {
    large_stack(|| run(async |admin| field_case(admin, 3164, 0, 2135, false, false, false).await))
}

#[test]
fn field_runes_postgres_protection_zone_refuses_fields_and_barriers_without_payment() -> TestResult
{
    large_stack(|| {
        run(async |admin| {
            for rune in [3188, 3164, 3172, 3180, 3156] {
                field_case(admin.clone(), rune, 0, 0, true, true, false).await?;
            }
            Ok(())
        })
    })
}

#[test]
fn field_runes_postgres_destroy_field_removes_listed_field_and_refuses_without_payment() -> TestResult {
    large_stack(|| run(async |admin| {
        // Crystal 201189f8 destroy_field_rune.lua:2 includes 2135 but not Magic Wall 2128.
        for (rune, output) in [(3164, 2135), (3180, 2128)] {
            field_scenario(admin.clone(), rune, 1, output, true, false, false, Scenario::DestroyField).await?;
        }
        field_case(admin, 3148, 0, 0, true, true, false).await?;
        Ok(())
    }))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scenario {
    Ordinary,
    DestroyField,
    Aged,
    ProtectedTarget,
    ProtectedCaster,
    Hardcore,
    SelfHardcore,
    SelfOpen,
    SecretMachete,
    NoPvp,
    PzTick,
    NoPvpTick,
    Death,
    RevokedCaster,
    DepartedCaster,
    PeriodicRollback,
    PeriodicCommit,
    PeriodicPermissionChange,
    Creature,
    ElementPlayer,
    ElementCreature,
    Equipped,
    EquippedPair,
    EquippedMantra,
    EquippedMantraAmmo,
    EquippedLastCharge,
    EquippedWrongSlot,
    EquippedRollback,
    EquippedCommit,
}

fn equipped(scenario: Scenario) -> bool {
    matches!(scenario, Scenario::Equipped | Scenario::EquippedPair | Scenario::EquippedMantra | Scenario::EquippedMantraAmmo | Scenario::EquippedLastCharge | Scenario::EquippedWrongSlot | Scenario::EquippedRollback | Scenario::EquippedCommit)
}

fn equipped_hit(scenario: Scenario, output: u32, ordinal: u32) -> u32 {
    if scenario==Scenario::EquippedPair {return 0;}
    let damage = field_player_hit(output);
    if scenario==Scenario::EquippedMantraAmmo {return damage.saturating_sub(if ordinal==0 {3}else{0});}
    if !equipped(scenario) || scenario == Scenario::EquippedWrongSlot || scenario == Scenario::EquippedLastCharge && ordinal > 0 { return damage; }
    // The admitted fixture profile combines 30% elemental + 20% field absorb,
    // after (for the initial Mantra hit) three flat points. It exercises the real owners
    // without claiming these altered values are a new production Item definition.
    let damage = damage.saturating_sub(if scenario == Scenario::EquippedMantra && ordinal==0 {3} else {0});
    damage - damage/2
}

#[test]
fn field_runes_postgres_equipped_step_tick_charges_and_mantra() -> TestResult {
    large_stack(|| run(async |admin| {
        for scenario in [Scenario::Equipped, Scenario::EquippedPair, Scenario::EquippedMantra, Scenario::EquippedLastCharge, Scenario::EquippedWrongSlot, Scenario::EquippedRollback, Scenario::EquippedCommit] {
            field_scenario(admin.clone(),3188,1,2118,true,false,false,scenario).await?;
        }
        for (rune,output) in [(3164,2122),(3172,105)] {
            field_scenario(admin.clone(),rune,1,output,true,false,false,Scenario::Equipped).await?;
        }
        Ok(())
    }))
}

#[test]
fn field_runes_postgres_equipped_step_unknown_commit() -> TestResult {
    large_stack(||run(async |admin| {
        for after in [false,true] {
            let mut reached=crate::gameplay_transport::field_removal::faults::arm(crate::gameplay_transport::field_removal::faults::PLAYER,after);
            field_scenario(admin.clone(),3188,1,2118,true,false,false,Scenario::Equipped).await?;
            assert_eq!(reached.try_recv(),Ok(()),"equipped STEP reaches its exact unknown COMMIT boundary");
        }
        field_scenario(admin,3188,1,2118,true,false,false,Scenario::EquippedMantraAmmo).await?;
        Ok(())
    }))
}

fn equipped_generation(world: crate::foundation::WorldId, scenario: Scenario) -> TestResult<(crate::content::ContentActivationController, crate::content::NativeEntryContentPin)> {
    use crate::content::{ContentActivationController, NativeEntryActivationIssuance, NodeBootQuiescence, activate_native_entry_room_with_gameplay};
    use sha2::{Digest,Sha256};
    let mut input = crate::content::native_gameplay::NativeGameplayInput::from_manifest(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../content/spells.manifest.json"))?;
    input.source_world = None;
    input.build_training.as_mut().ok_or("training")?.content_revision = "content-1".into();
    let pin = input.item_profiles.as_mut().ok_or("profiles")?;
    let mut profiles: serde_json::Value = serde_json::from_slice(&pin.bytes)?;
    let record = profiles["records"].as_array_mut().ok_or("records")?.iter_mut().find(|record|record["production_definition"]["production_key"]=="oteryn:item.tibia.i3081").ok_or("amulet")?;
    let mut resistances = Vec::new();
    for kind in ["EARTH","ENERGY","FIRE"] {
        resistances.push(serde_json::json!({"kind":kind,"percent":{"state":"KNOWN","value":{"numerator":if kind=="FIRE" {30}else{50},"denominator":1}}}));
    }
    // FIRE_FIELD is the typed schema's field-only protection; energy/earth
    // receive 50% elemental protection in the per-element scenarios below.
    resistances.push(serde_json::json!({"kind":"FIRE_FIELD","percent":{"state":"KNOWN","value":{"numerator":20,"denominator":1}}}));
    record["semantics"]["protection"] = serde_json::json!({"state":"KNOWN","value":{"armor":{"state":"UNKNOWN"},"resistances":{"state":"KNOWN","value":resistances}}});
    record["semantics"]["charges"]["value"]["count"]["value"] = serde_json::json!(if scenario==Scenario::EquippedLastCharge {1} else {5});
    record["semantics"]["skill_modifiers"] = serde_json::json!({"state":"KNOWN","value":{"modifiers":{"state":"KNOWN","value":if matches!(scenario,Scenario::EquippedMantra|Scenario::EquippedMantraAmmo) {vec![serde_json::json!({"kind":"MANTRA","parameter":{"state":"KNOWN","value":{"kind":"SIGNED_POINTS","value":3}},"evaluation_phase":{"state":"UNKNOWN"},"priority":{"state":"UNKNOWN"},"target_domain":{"state":"UNKNOWN"}})]}else{vec![]}}}});
    if scenario==Scenario::EquippedPair {
        let patterns=record["semantics"]["equipment"]["value"]["patterns"]["value"].as_array_mut().ok_or("patterns")?;
        let mut ring=patterns.first().ok_or("pattern")?.clone();
        ring["pattern_id"]=serde_json::json!(2);
        ring["primary_slot"]["value"]=serde_json::json!("RING");
        patterns.push(ring);
    }
    pin.bytes = serde_json::to_vec(&profiles)?;
    pin.sha256 = Sha256::digest(&pin.bytes).iter().map(|b|format!("{b:02x}")).collect();
    let room = crate::content::qualify_selected_native_gameplay_room(world,&input).map_err(debug)?;
    let issuance = NativeEntryActivationIssuance {world_id:world,activation_sequence:1,server_artifact_digest:room.compiled().server_digest(),client_artifact_digest:room.compiled().client_digest(),frame_binding_digest:room.frame_binding().digest()};
    let mut controller = ContentActivationController::new();
    let pin = activate_native_entry_room_with_gameplay(&mut controller,&NodeBootQuiescence::before_channel_runtime(),world,&issuance,&input).map_err(debug)?;
    Ok((controller,pin))
}

#[test]
fn field_runes_postgres_player_legality_and_death() -> TestResult {
    large_stack(|| {
        run(async |admin| {
            for scenario in [
                Scenario::Aged,
                Scenario::ProtectedTarget,
                Scenario::ProtectedCaster,
                Scenario::Hardcore,
                Scenario::NoPvp,
            ] {
                field_scenario(
                    admin.clone(),
                    3188,
                    1,
                    if scenario == Scenario::NoPvp {
                        21465
                    } else {
                        2118
                    },
                    true,
                    false,
                    false,
                    scenario,
                )
                .await?;
            }
            Ok(())
        })
    })
}

#[test]
fn field_runes_postgres_zone_tick_suppression() -> TestResult {
    large_stack(|| {
        run(async |admin| {
            for scenario in [Scenario::PzTick, Scenario::NoPvpTick] {
                field_scenario(admin.clone(), 3188, 1, 2118, true, false, false, scenario).await?;
            }
            Ok(())
        })
    })
}

#[test]
fn field_runes_postgres_player_death_and_source_loss() -> TestResult {
    large_stack(|| {
        run(async |admin| {
            for scenario in [Scenario::Death, Scenario::RevokedCaster, Scenario::DepartedCaster] {
                field_scenario(admin.clone(), 3188, 1, 2118, true, false, false, scenario).await?;
            }
            Ok(())
        })
    })
}

#[test]
fn field_runes_postgres_periodic_unknown_commit() -> TestResult {
    large_stack(|| {
        run(async |admin| {
            for scenario in [
                Scenario::PeriodicRollback,
                Scenario::PeriodicCommit,
                Scenario::PeriodicPermissionChange,
            ] {
                field_scenario(admin.clone(), 3188, 1, 2118, true, false, false, scenario).await?;
            }
            Ok(())
        })
    })
}

#[test]
fn field_runes_postgres_caster_entry_respects_world_type() -> TestResult {
    large_stack(|| {
        run(async |admin| {
            for scenario in [Scenario::SelfOpen, Scenario::SelfHardcore] {
                field_scenario(admin.clone(), 3188, 1, 2118, true, false, false, scenario).await?;
            }
            Ok(())
        })
    })
}

#[test]
fn field_runes_postgres_creature_entry_and_residual_tick() -> TestResult {
    large_stack(|| {
        run(async |admin| {
            field_scenario(
                admin,
                3188,
                1,
                21465,
                true,
                false,
                false,
                Scenario::Creature,
            )
            .await
        })
    })
}

#[test]
fn field_runes_postgres_energy_and_poison_hit_players_and_creatures() -> TestResult {
    large_stack(|| {
        run(async |admin| {
            for (rune, output) in [(3164, 2122), (3172, 105)] {
                for scenario in [Scenario::ElementPlayer, Scenario::ElementCreature] {
                    field_scenario(admin.clone(), rune, 1, output, true, false, false, scenario)
                        .await?;
                }
            }
            Ok(())
        })
    })
}

fn field_player_hit(output: u32) -> u32 {
    // Authored entry pulses (20 fire, 25 energy, 5 poison), then the
    // accepted 50% PvP factor, rounded down before target mitigation.
    match output {
        2122 => 12,
        105 => 2,
        _ => 10,
    }
}

fn field_creature_hit(output: u32) -> i64 {
    // The production cyclops has 0.97% mitigation, 25% energy resistance
    // and -10% earth resistance. Mitigation truncates before resistance.
    match output {
        2122 => 18,
        105 => 4,
        _ => 9,
    }
}

fn field_residual_ticks(output: u32) -> u32 {
    match output {
        // The source energy recipe contains only its immediate pulse.
        2122 => 0,
        // Poison has two due five-second segments in the eleven-second turn.
        105 => 2,
        _ => 1,
    }
}

async fn read_field_setup_fence(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    session: crate::foundation::GameSessionId,
) -> TestResult<crate::durability::character_progression::CurrentCharacterGameplayFence> {
    // The two independent reads share the root's bounded holder with its
    // asynchronous pool cleanup. A retry obtains a completely fresh fence;
    // an unavailable read never substitutes persisted setup evidence.
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            if let Ok(fence) = current_fence(owner, session).await {
                return fence;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .map_err(debug)
}

/// These scenarios run long owner-side checks (SQL, map observation, fixture moves) while
/// the test client is not reading the wire. Under CI load that exceeds the idle cadence and
/// ends the session mid-scenario. Liveness is not under test here (`idle_kick` and the
/// connection tests own it), so the probe interval outlasts the 180-second scenario timeout.
const FIELD_SCENARIO_LIVENESS: LivenessPolicy = LivenessPolicy {
    interval: std::time::Duration::from_secs(600),
    missed_limit: 3,
};

async fn field_case(
    admin: String,
    rune: u32,
    count: i64,
    output: u32,
    enabled: bool,
    protection: bool,
    owner_loss: bool,
) -> TestResult {
    field_scenario(
        admin,
        rune,
        count,
        output,
        enabled,
        protection,
        owner_loss,
        Scenario::Ordinary,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn field_scenario(
    admin: String,
    rune: u32,
    count: i64,
    output: u32,
    enabled: bool,
    protection: bool,
    owner_loss: bool,
    scenario: Scenario,
) -> TestResult {
    tokio::time::timeout(
        std::time::Duration::from_secs(180),
        field_scenario_inner(
            admin, rune, count, output, enabled, protection, owner_loss, scenario,
        ),
    )
    .await
    .map_err(debug)?
}

#[allow(clippy::too_many_arguments)]
async fn field_scenario_inner(
    admin: String,
    rune: u32,
    count: i64,
    output: u32,
    enabled: bool,
    protection: bool,
    owner_loss: bool,
    scenario: Scenario,
) -> TestResult {
    eprintln!("FIELD-RUNES start rune={rune} scenario={scenario:?}");
    let committed = enabled && !protection;
    let pvp_barrier = matches!(output, 2128 | 2130);
    let player_field = (matches!(output, 2118 | 2122 | 105)
        && !matches!(
            scenario,
            Scenario::SelfHardcore | Scenario::SelfOpen | Scenario::ElementCreature
        ))
        || scenario == Scenario::NoPvp;
    let harmful = player_field
        && !matches!(
            scenario,
            Scenario::Aged
                | Scenario::NoPvp
                | Scenario::ProtectedTarget
                | Scenario::ProtectedCaster
        );
    let mut creature = None;
    let harness =
        Harness::create_with_admission_seed(admin, "field_runes_wire", true, false).await?;
    prepare_current_session(&harness).await?;
    if player_field || owner_loss {
        harness.seed_pvp_target().await?;
        super::appearance_change_postgres::observer_visibility_tests::admit(&harness).await?;
    }
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) = if equipped(scenario) {equipped_generation(world,scenario)?} else {production_generation_with_pin(world, "content-1")?};
    let active = controller.active().ok_or("active generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let entry_room = pin.qualified_room();
    let map = if matches!(scenario, Scenario::PzTick | Scenario::NoPvpTick) {
        field_world(world, if scenario == Scenario::PzTick { 1 } else { 4 })?
    } else if protection {
        crate::map::spell_tiles::tests::world()
    } else {
        crate::map::spell_tiles::tests::movement_arena::world()
    };
    let cells = map
        .movement_cells(entry_room.movement_cells())
        .map_err(debug)?;
    // Runtime floors are the negation of the bundle's native floors.
    let start = MovementLocalPosition {
        x: if protection { 1 } else { 100 },
        y: if protection { 0 } else { 100 },
        floor: 7,
    };
    let room = entry_room
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
        .ok_or("bundle field room")?;
    let channel_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let dependencies =
        Dependencies::with_content(channel_pin, entry_room.clone(), native.spell_book().clone())?;
    let (actor, session) = (dependencies.actor, dependencies.admitted.game_session_id);
    {
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
    }
    let mut owner = dependencies.owner(&harness, &authority);
    owner.emit = |_, line| {
        eprintln!(
            "FIELD-RUNES owner module={} event={} code={:?}",
            line.module, line.event, line.code
        )
    };
    owner.active_generation = Some(active);
    owner.rooms = SeamRooms::new(None, Some(&room));
    owner.movement_cells = &cells;
    owner.required_capabilities = BoundRequiredCapabilities::for_channel_boot(
        &[11, 30],
        crate::foundation::RuntimeScopeRefV1::channel(world, dependencies.admitted.channel_id),
        1,
        "policy-1",
        BUNDLE_WORLD_OFFERED_CAPABILITIES,
    )
    .map_err(debug)?;
    let formula = native.training_formula().ok_or("training formula")?;
    let druid = convert_vocation(formula, &DurableBuildState::default(), "druid").map_err(debug)?;
    harness
        .root
        .commit_character_build(
            &authority,
            &harness.node,
            read_field_setup_fence(&owner, session).await?,
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
            read_field_setup_fence(&owner, session).await?,
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
    let target = if player_field || owner_loss {
        let target = super::appearance_change_postgres::observer_visibility_tests::initialize(
            &dependencies,
            &owner,
        )
        .await?;
        let mut runtime = dependencies.runtime.lock().await;
        for x in [start.x + 1, start.x + 2] {
            let before = runtime.read_actor_position(target).map_err(debug)?;
            runtime
                .borrow_movement_position()
                .commit_cardinal(
                    before,
                    MovementLocalPosition {
                        x,
                        y: start.y,
                        floor: start.floor,
                    },
                )
                .map_err(debug)?;
        }
        drop(runtime);
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(101).as_slice()).execute(&harness.pool).await?;
        {
            let runtime = dependencies.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let target_session =
                crate::foundation::GameSessionId::decode(&id(110)).map_err(debug)?;
            let state = states
                .get_mut(&runtime, target, target_session)
                .ok_or("target state")?;
            // These assertions isolate field damage. Unfed actors now regenerate too;
            // keep this fresh fixture's regeneration deadline beyond the 180-second
            // scenario timeout and its 11-second clock advance. Preserve Monk state.
            let monk = state.monk.take();
            crate::spell::actor_conditions::clear_on_lifecycle(state);
            state
                .make_playable(owner.owner_now().checked_add(3_600_000_000).map_err(debug)?)
                .map_err(debug)?;
            state.monk = monk;
            if scenario == Scenario::Death {
                state.health = 8;
            }
        }
        Some(target)
    } else {
        None
    };
    if equipped(scenario) {
        let policy = native.item_policy_for_source_id(3081).ok_or("equipment policy")?;
        let definition = &policy.record().production_definition;
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($5,'hex')::uuid)")
            .bind(id(180).as_slice()).bind(id(42).as_slice()).bind(&definition.production_key).bind(&definition.revision_ref).bind(id(181).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid)")
            .bind(id(101).as_slice()).bind(if matches!(scenario,Scenario::EquippedWrongSlot|Scenario::EquippedMantraAmmo) {10_i16}else{7_i16}).bind(id(180).as_slice()).bind(id(42).as_slice()).bind(id(182).as_slice()).execute(&mut *tx).await?;
        if scenario==Scenario::EquippedPair {
            sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($5,'hex')::uuid)")
                .bind(id(183).as_slice()).bind(id(42).as_slice()).bind(&definition.production_key).bind(&definition.revision_ref).bind(id(184).as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,8,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
                .bind(id(101).as_slice()).bind(id(183).as_slice()).bind(id(42).as_slice()).bind(id(185).as_slice()).execute(&mut *tx).await?;
        }
        tx.commit().await?;
    }
    let mut inventory = vec![(format!("oteryn:item.tibia.i{rune}"), 3)];
    if matches!(output, 2128 | 2130) {
        inventory.push((
            format!("oteryn:item.tibia.i{}", if scenario == Scenario::SecretMachete { 9594 } else { 3308 }),
            1,
        ));
    }
    if scenario == Scenario::DestroyField {
        inventory.push(("oteryn:item.tibia.i3148".into(), 3));
    }
    seed_main_backpack(
        &harness,
        &inventory
            .iter()
            .map(|(key, count)| (key.as_str(), *count))
            .collect::<Vec<_>>(),
    )
    .await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice())
        .bind(pin.identity().client_artifact_digest().as_slice()).bind(room.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
    sqlx::query("INSERT INTO game_control_field_policy_grants VALUES(session_user,encode($1,'hex')::uuid,'policy-1')")
        .bind(id(42).as_slice()).execute(&harness.pool).await?;
    sqlx::query("INSERT INTO game_spell_field_world_policies(world_id,world_policy_revision,world_type,protection_level,in_fight_ms,source_pin,control_role,decision_identity) VALUES(encode($1,'hex')::uuid,'policy-1',$2,$3,60000,'99902524e052f37574194466c2949c576e4ab269',session_user,'field-runes-fixture')")
        .bind(id(42).as_slice()).bind(if matches!(scenario,Scenario::Hardcore|Scenario::SelfHardcore) {3_i16} else if pvp_barrier||scenario==Scenario::SelfOpen||scenario==Scenario::ElementCreature||player_field&&scenario!=Scenario::NoPvp { 2_i16 } else { 1_i16 }).bind(if scenario==Scenario::ProtectedCaster {51_i32} else if scenario==Scenario::ProtectedTarget {20_i32} else if player_field {8_i32}else{20_i32}).execute(&harness.pool).await?;
    let mut admitted = dependencies.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let (current, _) = FreshAdmissionStore::from_root(harness.root.clone())
        .current_session_at(session)
        .await?;
    admitted.item_fence = item_fence_of(current);
    admitted.continuity.selected_capabilities = owner
        .required_capabilities
        .authenticate_support(if enabled {
            &[4, 6, 11, 15, 18, 22, 30, 91]
        } else {
            &[4, 6, 11, 15, 18, 22, 30]
        })
        .map_err(debug)?
        .selected();
    let before = {
        let runtime = dependencies.runtime.lock().await;
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
        let end = serve_admitted(&mut server, admitted, &owner, FIELD_SCENARIO_LIVENESS).await;
        drop(server);
        match end {
            ConnectionEnd::AdmittedThenDisconnected(_) => {}
            other => return Err(format!("unexpected connection end: {other:?}").into()),
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let mut map_view = crate::gameplay_transport::world_map::SessionMapView::default();
    let mut map_items = crate::gameplay_transport::item_view::SessionItemView::resume(
        crate::gameplay_transport::connection::SessionContinuity::FRESH.item_view,
    )
    .with_map_view();
    let baseline = owner
        .observe_world_map(actor, &mut map_view, &mut map_items)
        .await
        .ok_or("map baseline")?
        .map_err(debug)?
        .ok_or("map baseline")?;
    let crate::gameplay_transport::world_map::MapUpdate::Snapshot(baseline) = baseline else {
        return Err("map baseline snapshot".into());
    };
    let map_header =
        oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(&baseline.payload)
            .map_err(debug)?
            .header;
    let inventory_definition = |id| -> TestResult<std::num::NonZeroU32> {
        let key = format!("oteryn:item.tibia.i{id}");
        let index = native.item_index().ok_or("inventory index")?;
        let revision = index.revision(&key).ok_or("inventory revision")?;
        index
            .definition_ref("Item", &key, revision)
            .ok_or_else(|| "inventory definition".into())
    };
    let rune_definition = inventory_definition(rune)?;
    let machete_id = if scenario == Scenario::SecretMachete { 9594 } else { 3308 };
    let machete_definition = inventory_definition(machete_id)?;
    let destroy_definition = inventory_definition(3148)?;
    let casting = async {
        let mut handle = None;
        let mut field_handle = None;
        let mut machete = None;
        let mut destroy = None;
        let mut saw_map = false;
        loop {
            let bytes = read_frame(&mut client).await.map_err(|error| {
                format!("field rune={rune} enabled={enabled} protection={protection}: {error}")
            })?;
            let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
            field_handle = field_handle.or(field_handle_from_wire(&envelope, &map_header, start)?);
            if envelope.message_type() == MessageType::SnapshotChunk {
                for domain in decode_snapshot_chunk(envelope.payload()).map_err(debug)?.1 {
                    saw_map |= domain.domain_id
                        == oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW;
                    if domain.domain_id == STATE_DOMAIN_CHARACTER_INVENTORY {
                        let inventory =
                            decode_character_inventory(domain.payload).map_err(debug)?;
                        handle = inventory
                            .entries
                            .iter()
                            .find(|item| item.item_definition_ref == rune_definition)
                            .map(|item| item.handle);
                        destroy = inventory.entries.iter().find(|item| item.item_definition_ref == destroy_definition).map(|item| item.handle);
                        machete = inventory
                            .entries
                            .iter()
                            .find(|item| item.item_definition_ref == machete_definition)
                            .map(|item| item.handle);
                    }
                }
            }
            if envelope.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        assert!(
            saw_map,
            "the required map capability has a real bundle baseline"
        );
        if player_field || matches!(scenario, Scenario::SelfHardcore | Scenario::SelfOpen) {
            wait_field_client(&mut client, std::time::Duration::from_secs(10)).await?;
        }
        let intent = UseIntent {
            target: UseTarget::Item(handle.ok_or("rune handle")?),
            use_with: None,
            use_with_map_item: None,
            use_with_position: Some(SpellTargetPosition {
                x: start.x + 1,
                y: start.y,
                floor: start.floor,
            }),
        };
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        let bytes = test_command(
            admitted.continuity.next_command_id,
            COMMAND_TYPE_USE_INTENT,
            &payload,
        );
        client.write_u32(u32::try_from(bytes.len())?).await?;
        client.write_all(&bytes).await?;
        loop {
            let bytes = read_frame(&mut client).await.map_err(|error| {
                format!("field rune={rune} enabled={enabled} protection={protection}: {error}")
            })?;
            let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
            field_handle = field_handle.or(field_handle_from_wire(&envelope, &map_header, start)?);
            if envelope.message_type() == MessageType::CommandResult {
                let result = decode_command_result(envelope.payload()).map_err(debug)?;
                let disposition = decode_use_result(result.payload).map_err(debug)?;
                assert_eq!(
                    disposition,
                    if protection {
                        UseDisposition::TargetIllegal
                    } else if enabled {
                        UseDisposition::Committed
                    } else {
                        UseDisposition::Rejected
                    },
                    "rune {rune} enabled={enabled} protection={protection}"
                );
                break;
            }
        }
        if committed {
            // Map publication has its own cadence: a command result does not
            // guarantee that the client has received the field's session handle.
            // Await that dependency before cooldown readers can discard its delta.
            if matches!(output, 2128 | 2130) {
                while field_handle.is_none() {
                    let frame = read_frame(&mut client).await?;
                    let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                    // The wait can span idle probes; an unanswered probe ends the session.
                    if envelope.message_type() == MessageType::LivenessProbe {
                        let probe = oteryn_protocol_oteryn::decode_liveness_probe(envelope.payload())
                            .map_err(debug)?;
                        let ack = oteryn_protocol_oteryn::encode_liveness_ack(1, probe, 0)
                            .map_err(debug)?;
                        client.write_u32(u32::try_from(ack.len())?).await?;
                        client.write_all(&ack).await?;
                        continue;
                    }
                    field_handle = field_handle_from_wire(&envelope, &map_header, start)?;
                }
            }
            let delta = owner
                .observe_world_map(actor, &mut map_view, &mut map_items)
                .await
                .ok_or("map appearance")?
                .map_err(debug)?
                .ok_or("field appearance delta")?;
            let crate::gameplay_transport::world_map::MapUpdate::Delta(delta) = delta else {
                return Err("field appearance must be a delta".into());
            };
            let delta = oteryn_protocol_oteryn::world_map::decode_world_map_delta(
                &map_header,
                &delta.payload,
            )
            .map_err(debug)?;
            assert!(
                delta.tiles.iter().any(|tile| tile.position.x == start.x + 1
                    && tile.position.y == start.y
                    && tile.position.floor == -7
                    && tile.items.len() > 1),
                "appearance reaches the persistent real map consumer"
            );
            if matches!(scenario, Scenario::Creature | Scenario::ElementCreature) {
                creature = Some(qualify_creature_entry(&owner, start, output).await?);
            }
            if matches!(output, 2128 | 2130 | 10181 | 10182) {
                qualify_barrier_consumers(&owner, start, output).await?;
            }
            let projected = projected_field_count(&owner, actor, output).await?;
            assert_eq!(
                projected, count,
                "the existing map consumer shows every field on the converted bundle floor"
            );
            let lock_before: (String, String) = sqlx::query_as(
                "SELECT pz_block_until::text,revision::text FROM game_character_pvp_state WHERE character_id=encode($1,'hex')::uuid",
            )
            .bind(id(41).as_slice())
            .fetch_one(&harness.pool)
            .await?;
            assert!(crate::gameplay_transport::pvp_live::pz_blocked(
                &*owner.states.spell_states.lock().await,
                actor,
                session,
                owner.owner_now(),
            ));
            client.write_u32(u32::try_from(bytes.len())?).await?;
            client.write_all(&bytes).await?;
            loop {
                let bytes = read_frame(&mut client).await.map_err(|error| {
                    format!("field rune={rune} enabled={enabled} protection={protection}: {error}")
                })?;
                let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
                field_handle =
                    field_handle.or(field_handle_from_wire(&envelope, &map_header, start)?);
                assert_ne!(envelope.message_type(), MessageType::ProtocolError);
                if envelope.message_type() == MessageType::CommandResult {
                    // FND-02 §13.2: the retained outcome replays; the family is not called
                    // again, which the single receipt and quantity below prove durably.
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(result.status, CommandStatus::DuplicateReplay);
                    assert_eq!(
                        decode_use_result(result.payload).map_err(debug)?,
                        UseDisposition::Committed
                    );
                    break;
                }
            }
            let lock_after: (String, String) = sqlx::query_as(
                "SELECT pz_block_until::text,revision::text FROM game_character_pvp_state WHERE character_id=encode($1,'hex')::uuid",
            )
            .bind(id(41).as_slice())
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(
                lock_after, lock_before,
                "a replay never refreshes the cast block"
            );
        }
        if scenario == Scenario::DestroyField {
            wait_field_client(&mut client, std::time::Duration::from_secs(2)).await?;
            for (offset, (dx, floor, accepted)) in [(2, start.floor, false), (6, start.floor, false), (1, start.floor + 1, false), (1, start.floor, output == 2135)].into_iter().enumerate() {
                let command_id = admitted.continuity.next_command_id + 1 + u64::try_from(offset)?;
                let destroy_intent = UseIntent {
                    target: UseTarget::Item(destroy.ok_or("Destroy Field handle")?),
                    use_with: None,
                    use_with_map_item: None,
                    use_with_position: Some(SpellTargetPosition { x: start.x + dx, y: start.y, floor }),
                };
                let command = test_command(command_id, COMMAND_TYPE_USE_INTENT, &encode_item_use_intent(&destroy_intent).map_err(debug)?);
                client.write_u32(u32::try_from(command.len())?).await?;
                client.write_all(&command).await?;
                loop {
                    let frame = read_frame(&mut client).await?;
                    let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                    if envelope.message_type() == MessageType::CommandResult {
                        let result = decode_command_result(envelope.payload()).map_err(debug)?;
                        assert_eq!(result.command_id, command_id);
                        assert_eq!(decode_use_result(result.payload).map_err(debug)?, if accepted { UseDisposition::Committed } else if dx == 6 { UseDisposition::TooFar } else { UseDisposition::TargetIllegal }, "Destroy Field dx={dx} floor={floor} output={output}");
                        break;
                    }
                }
                let quantity: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3148'").fetch_one(&harness.pool).await?;
                assert_eq!(quantity, if accepted { 2 } else { 3 }, "only a successful removal consumes one charge");
            }
            assert_eq!(projected_field_count(&owner, actor, output).await?, if output == 2135 { 0 } else { 1 });
            let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_ground_locations").fetch_one(&harness.pool).await?;
            assert_eq!(remaining, if output == 2135 { 0 } else { 1 }, "removal changes durable Ground custody");
            client.shutdown().await?;
            return Ok::<_, Box<dyn std::error::Error>>(());
        }
        if committed && !player_field && !owner_loss {
            let projectile_probe = matches!(output, 2128 | 10181);
            if projectile_probe {
                wait_field_client(&mut client, std::time::Duration::from_secs(2)).await?;
                let behind_wall = UseIntent {
                    use_with_position: Some(SpellTargetPosition {
                        x: start.x + 2,
                        y: start.y,
                        floor: start.floor,
                    }),
                    ..intent.clone()
                };
                let probe = test_command(
                    admitted.continuity.next_command_id + 1,
                    COMMAND_TYPE_USE_INTENT,
                    &encode_item_use_intent(&behind_wall).map_err(debug)?,
                );
                client.write_u32(u32::try_from(probe.len())?).await?;
                client.write_all(&probe).await?;
                loop {
                    let frame = read_frame(&mut client).await?;
                    let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                    if envelope.message_type() == MessageType::CommandResult {
                        let result = decode_command_result(envelope.payload()).map_err(debug)?;
                        assert_eq!(
                            decode_use_result(result.payload).map_err(debug)?,
                            UseDisposition::TargetIllegal,
                            "the real cast ray cannot cross a live Magic Wall"
                        );
                        break;
                    }
                }
            }
            let step_command =
                admitted.continuity.next_command_id + 1 + u64::from(projectile_probe);
            if matches!(output, 2128 | 10181 | 2130 | 10182) {
                let flags: (bool, bool, bool) = sqlx::query_as(
                    "SELECT blocks_movement,blocks_projectile,immovable_block_solid FROM game_spell_item_lines WHERE operation_kind=1",
                )
                .fetch_one(&harness.pool)
                .await?;
                assert_eq!(
                    flags,
                    (pvp_barrier, matches!(output, 2128 | 10181), pvp_barrier),
                    "rune {rune} retains its effective barrier flags"
                );
                let duplicate_deadline: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules)",
                )
                .fetch_one(&harness.pool)
                .await?;
                assert!(
                    !duplicate_deadline,
                    "rune {rune} has only its callback's per-instance deadline"
                );
            }
            let before = dependencies
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?;
            let mut self_interrupted = if scenario == Scenario::SelfHardcore {
                Some(crate::gameplay_transport::field_removal::faults::arm(
                    crate::gameplay_transport::field_removal::faults::PLAYER,
                    false,
                ))
            } else {
                None
            };
            let step = test_command(
                step_command,
                COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                &encode_step_intent(StepDirection::East),
            );
            client.write_u32(u32::try_from(step.len())?).await?;
            client.write_all(&step).await?;
            loop {
                let bytes = read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
                field_handle =
                    field_handle.or(field_handle_from_wire(&envelope, &map_header, start)?);
                assert_ne!(envelope.message_type(), MessageType::ProtocolError);
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(result.command_id, step_command);
                    assert_eq!(
                        decode_step_result(result.payload).map_err(debug)?,
                        if pvp_barrier {
                            StepDisposition::Blocked
                        } else {
                            StepDisposition::Moved
                        },
                        "rune {rune}"
                    );
                    break;
                }
            }
            let after = dependencies
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?;
            drop(
                owner
                    .spell_lane_permit()
                    .await
                    .ok_or("field removal reconciliation")?,
            );
            if scenario == Scenario::SelfHardcore {
                assert_eq!(
                    self_interrupted
                        .as_mut()
                        .ok_or("self hit fault")?
                        .try_recv(),
                    Ok(()),
                    "the self hit also reconciles a rolled back COMMIT"
                );
                assert_eq!(
                    player_health(&owner, actor, session).await?,
                    crate::gameplay_transport::actor_spell::tests::FACTS.max_health - 10,
                    "Hardcore fields hurt their caster through the native HP owner"
                );
            }
            if scenario == Scenario::SelfOpen {
                assert_eq!(
                    player_health(&owner, actor, session).await?,
                    crate::gameplay_transport::actor_spell::tests::FACTS.max_health,
                    "Open PvP fields do not harm their caster"
                );
            }
            if pvp_barrier {
                assert_eq!(after, before);
            } else {
                assert_eq!(after.position().x, start.x + 1);
                assert_eq!(after.position().y, start.y);
            }
            if matches!(output, 2128 | 2130) {
                let use_tool = UseIntent {
                    target: UseTarget::Item(machete.ok_or("machete handle")?),
                    use_with: None,
                    use_with_map_item: Some(
                        oteryn_protocol_oteryn::world_object::MapUseTarget::Item(
                            field_handle.ok_or("wire field handle")?,
                        ),
                    ),
                    use_with_position: None,
                };
                move_field_caster(&mut client, step_command + 1, StepDirection::West).await?;
                move_field_caster(&mut client, step_command + 2, StepDirection::West).await?;
                let far = use_tool.clone();
                let refused = test_command(
                    step_command + 3,
                    COMMAND_TYPE_USE_INTENT,
                    &encode_item_use_intent(&far).map_err(debug)?,
                );
                client.write_u32(u32::try_from(refused.len())?).await?;
                client.write_all(&refused).await?;
                loop {
                    let frame = read_frame(&mut client).await?;
                    let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                    if envelope.message_type() == MessageType::CommandResult {
                        let result = decode_command_result(envelope.payload()).map_err(debug)?;
                        assert_eq!(
                            decode_use_result(result.payload).map_err(debug)?,
                            UseDisposition::TargetIllegal,
                            "machete reach is checked before any Item removal"
                        );
                        break;
                    }
                }
                move_field_caster(&mut client, step_command + 4, StepDirection::East).await?;
                move_field_caster(&mut client, step_command + 5, StepDirection::East).await?;
                let bytes = test_command(
                    step_command + 6,
                    COMMAND_TYPE_USE_INTENT,
                    &encode_item_use_intent(&use_tool).map_err(debug)?,
                );
                for replay in [false, true] {
                    client.write_u32(u32::try_from(bytes.len())?).await?;
                    client.write_all(&bytes).await?;
                    loop {
                        let frame = read_frame(&mut client).await?;
                        let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                        if envelope.message_type() == MessageType::CommandResult {
                            let result =
                                decode_command_result(envelope.payload()).map_err(debug)?;
                            assert_eq!(
                                decode_use_result(result.payload).map_err(debug)?,
                                if output == 2130 {
                                    UseDisposition::Committed
                                } else {
                                    UseDisposition::TargetIllegal
                                }
                            );
                            if replay {
                                assert_eq!(result.status, CommandStatus::DuplicateReplay);
                            }
                            break;
                        }
                    }
                }
            }
            if matches!(output, 2128 | 2130) {
                let tool:(i16,i64)=sqlx::query_as("SELECT lifecycle,quantity::bigint FROM game_item_instances WHERE definition_production_key=$1").bind(format!("oteryn:item.tibia.i{machete_id}")).fetch_one(&harness.pool).await?;
                assert_eq!(
                    tool,
                    (1, 1),
                    "a machete removes growth without being consumed"
                );
            }
            let removals: i64 = sqlx::query_scalar("SELECT count(*) FROM game_field_removals")
                .fetch_one(&harness.pool)
                .await?;
            assert_eq!(
                removals,
                i64::from(matches!(output, 10181 | 10182 | 2130)),
                "safe entry and machete retire the exact barrier once"
            );
        }
        if matches!(output, 10181 | 10182 | 2130) {
            let delta = owner
                .observe_world_map(actor, &mut map_view, &mut map_items)
                .await
                .ok_or("map retirement")?
                .map_err(debug)?
                .ok_or("field leave delta")?;
            let crate::gameplay_transport::world_map::MapUpdate::Delta(delta) = delta else {
                return Err("field leave must be a delta".into());
            };
            let delta = oteryn_protocol_oteryn::world_map::decode_world_map_delta(
                &map_header,
                &delta.payload,
            )
            .map_err(debug)?;
            assert!(
                delta.tiles.iter().any(|tile| tile.position.x == start.x + 1
                    && tile.position.y == start.y
                    && tile.items.len() == 1),
                "durable removal emits the field leave through the existing map delta consumer"
            );
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    tokio::time::timeout(
        std::time::Duration::from_secs(if player_field { 90 } else { 30 }),
        async {
            let (served, result) = join_two(serving, casting).await;
            result?;
            served?;
            // The fixture root has one SQL holder. Finish the caster's wire
            // phase before driving the victim: the caster's vitals cadence
            // reads equipment outside the spell lane and can otherwise make
            // STEP's fresh PvP read unavailable. Ending this wire driver does
            // not end the Game session or its field ownership.
            if player_field && let Some(target) = target {
                if equipped(scenario) {
                    assert!(
                        owner.runtime.lock().await
                            .player_control_facts(actor, session)
                            .is_ok_and(|facts| facts.control_loss.is_none()),
                        "the field caster remains controlled after its wire phase"
                    );
                }
                drive_player_field(
                    &owner, &harness, admitted, target, start, scenario, harmful, output,
                )
                .await?;
            }
            Ok::<_, Box<dyn std::error::Error>>(())
        },
    )
    .await??;
    eprintln!("FIELD-RUNES transport complete rune={rune} scenario={scenario:?}");
    if scenario == Scenario::DestroyField {
        drop(owner);
        drop(authority);
        drop(seal);
        return harness.cleanup().await;
    }
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(id(50).as_slice()).fetch_one(&harness.pool).await?;
    let quantity: i64 = sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(71).as_slice())
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(receipts, i64::from(committed));
    assert_eq!(quantity, if committed { 2 } else { 3 });
    let locks: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_pvp_state WHERE character_id=encode($1,'hex')::uuid AND logout_block_until>clock_timestamp() AND pz_block_until>clock_timestamp() AND skull=0 AND white_skull_until IS NULL",
    )
    .bind(id(41).as_slice())
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(
        locks,
        i64::from(committed && (!harmful || matches!(scenario,Scenario::Hardcore|Scenario::DepartedCaster))),
        "a cast holds PZ without granting a skull"
    );
    if !committed {
        let runtime = dependencies.runtime.lock().await;
        assert_eq!(
            owner
                .states
                .spell_states
                .lock()
                .await
                .get(&runtime, actor, session)
                .ok_or("caster")?
                .vitals(),
            before
        );
    }
    // Reload durable Ground custody independently of the runtime's physical installation.
    let ground: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_ground_locations g JOIN game_spell_item_lines l USING(item_instance_id) WHERE l.transaction_id IN(SELECT transaction_id FROM game_spell_item_receipts WHERE game_session_id=encode($1,'hex')::uuid)")
        .bind(id(50).as_slice()).fetch_one(&harness.pool).await?;
    let remaining_count = if committed && matches!(output, 10181 | 10182 | 2130) {
        0
    } else {
        count
    };
    assert_eq!(ground, remaining_count, "rune {rune}");
    if committed {
        let outputs: Vec<String> = sqlx::query_scalar("SELECT i.definition_production_key FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) ORDER BY g.stack_ordinal LIMIT 20")
            .fetch_all(&harness.pool).await?;
        assert_eq!(outputs.len(), usize::try_from(remaining_count)?);
        assert!(
            outputs
                .iter()
                .all(|key| key == &format!("oteryn:item.tibia.i{output}")),
            "rune {rune}: {outputs:?}"
        );
        let origins: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id) WHERE r.character_id=encode($1,'hex')::uuid AND l.operation_kind=1 AND r.caster_lease_generation=2")
            .bind(id(41).as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(origins, count, "every field retains its caster");
        let timed: i64 = sqlx::query_scalar("SELECT count(DISTINCT l.item_instance_id) FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id) LEFT JOIN game_spell_field_temporal_schedules s ON s.item_instance_id=l.item_instance_id AND s.source_transaction_id=l.transaction_id WHERE l.operation_kind=1 AND (l.expires_at_unix_ms>r.occurred_at_unix_ms OR s.expires_at_unix_ms>r.occurred_at_unix_ms)")
            .fetch_one(&harness.pool).await?;
        assert_eq!(timed, count, "every field has a bounded lifetime");
        if matches!(output, 2128 | 10181 | 2130 | 10182) {
            let expected = if matches!(output, 2128 | 10181) {
                (16_000_i64, 24_000_i64)
            } else {
                // Global's variable 30–60 s duration is the accepted projection
                // (WORLDINT0-RL-11), despite the pinned OTS Lua's fixed 30 s.
                (30_000_i64, 60_000_i64)
            };
            let duration: i64 = sqlx::query_scalar(
                "SELECT l.expires_at_unix_ms-r.occurred_at_unix_ms FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id) WHERE l.operation_kind=1 AND r.character_id=encode($1,'hex')::uuid",
            )
            .bind(id(41).as_slice())
            .fetch_one(&harness.pool)
            .await?;
            assert!(
                (expected.0..=expected.1).contains(&duration),
                "rune {rune} duration={duration}"
            );
            assert_eq!(duration % 1_000, 0);
        }
        if owner_loss {
            qualify_safe_field_after_owner_loss(
                &owner,
                &harness,
                admitted,
                actor,
                target.ok_or("recovery peer")?,
            )
            .await?;
        }
        let projection_actor = if owner_loss {
            qualify_field_restart(&mut owner, &harness).await?
        } else {
            actor
        };
        // Test-only clock provisioning. The production owner still performs every
        // transform, terminal removal and receipt write using its original schedule.
        let mut aging = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *aging)
            .await?;
        sqlx::query("UPDATE game_spell_item_lines SET expires_at_unix_ms=1 WHERE operation_kind=1 AND expires_at_unix_ms IS NOT NULL")
            .execute(&mut *aging).await?;
        sqlx::query(
            "UPDATE game_spell_field_temporal_schedules SET expires_at_unix_ms=source_stage+1",
        )
        .execute(&mut *aging)
        .await?;
        aging.commit().await?;
        // Aging is fixture time provisioning, so make the next real owner pass
        // due as well; the serving connection may just have polled the clock.
        owner
            .states
            .spell_states
            .lock()
            .await
            .next_item_deadline_pass_us = 0;
        owner.drain_source_item_deadlines().await;
        let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_ground_locations g JOIN game_spell_item_lines l USING(item_instance_id) WHERE l.operation_kind=1")
            .fetch_one(&harness.pool).await?;
        assert_eq!(remaining, 0, "rune {rune} decays to nothing");
        let retired: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances i JOIN game_spell_item_lines l USING(item_instance_id) WHERE l.operation_kind=1 AND i.lifecycle=2 AND i.quantity=0")
            .fetch_one(&harness.pool).await?;
        assert_eq!(retired, count, "rune {rune} expires through the Item owner");
        assert_eq!(
            projected_field_count(&owner, projection_actor, output).await?,
            0,
            "the cached map drops expired or removed fields"
        );
        if remaining_count > 0 && !owner_loss {
            let delta = owner
                .observe_world_map(actor, &mut map_view, &mut map_items)
                .await
                .ok_or("expiry map consumer")?
                .map_err(debug)?
                .ok_or("expiry leave delta")?;
            let crate::gameplay_transport::world_map::MapUpdate::Delta(delta) = delta else {
                return Err("expiry leave must be a delta".into());
            };
            let delta = oteryn_protocol_oteryn::world_map::decode_world_map_delta(
                &map_header,
                &delta.payload,
            )
            .map_err(debug)?;
            assert!(
                delta.tiles.iter().any(|tile| tile.position.x == start.x + 1
                    && tile.position.y == start.y
                    && tile.items.len() == 1),
                "expiry invalidates the cache and sends a leave on the converted floor"
            );
        }
        if let Some(creature) = creature {
            qualify_creature_residual(&mut owner, creature, output).await?;
        }
        if player_field
            && scenario != Scenario::Death
            && let Some(target) = target
        {
            let target_session =
                crate::foundation::GameSessionId::decode(&id(110)).map_err(debug)?;
            let before = player_health(&owner, target, target_session).await?;
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(11))
                .ok_or("clock provisioning")?;
            let mut interrupted = if matches!(
                scenario,
                Scenario::PeriodicRollback
                    | Scenario::PeriodicCommit
                    | Scenario::PeriodicPermissionChange
                    | Scenario::EquippedRollback | Scenario::EquippedCommit
            ) {
                Some(crate::gameplay_transport::field_removal::faults::arm(
                    crate::gameplay_transport::field_removal::faults::PLAYER,
                    matches!(scenario, Scenario::PeriodicCommit | Scenario::EquippedCommit),
                ))
            } else {
                None
            };
            eprintln!("FIELD-RUNES periodic begin {scenario:?}");
            let tick = owner.tick_spell_periodic(target, target_session).await;
            eprintln!("FIELD-RUNES periodic returned {scenario:?}");
            if scenario == Scenario::PeriodicPermissionChange {
                // Change exactly the current source liveness at the retained
                // successor boundary. The original transaction rolled back;
                // its historical lineage must neither grant damage nor keep
                // the channel lane occupied after reconciliation.
                let source = owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .actors
                    .iter()
                    .find(|(_, held, _)| *held == session)
                    .map(|(actor, _, _)| *actor)
                    .ok_or("source actor")?;
                owner
                    .runtime
                    .lock()
                    .await
                    .record_control_loss(
                        source,
                        session,
                        oteryn_gameplay_entities::ControlLossMark {
                            epoch: 1,
                            grace_deadline: i64::MAX,
                        },
                    )
                    .map_err(debug)?;
            }
            drop(
                owner
                    .spell_lane_permit()
                    .await
                    .ok_or("periodic reconciliation")?,
            );
            eprintln!("FIELD-RUNES reconciliation returned {scenario:?}");
            if let Some(reached) = interrupted.as_mut() {
                assert_eq!(reached.try_recv(), Ok(()));
            }
            let residual = harmful
                && scenario!=Scenario::EquippedPair
                && field_residual_ticks(output) > 0
                && !matches!(
                    scenario,
                    Scenario::PzTick
                        | Scenario::NoPvpTick
                        | Scenario::PeriodicPermissionChange
                );
            if residual && interrupted.is_none() {
                assert!(
                    tick.is_some(),
                    "the real periodic owner consumes the residual condition after decay"
                );
            }
            assert_eq!(
                player_health(&owner, target, target_session).await?,
                before
                    - if matches!(scenario,Scenario::Aged|Scenario::RevokedCaster|Scenario::DepartedCaster) {
                        20
                    } else if residual {
                        (1..=field_residual_ticks(output)).map(|ordinal| equipped_hit(scenario,output,ordinal)).sum()
                    } else {
                        0
                    }
            );
            let harm:(i64,i64)=sqlx::query_as("SELECT count(*),COALESCE(sum(damage),0)::bigint FROM game_character_pvp_harm WHERE octet_length(field_occurrence)=32").fetch_one(&harness.pool).await?;
            assert_eq!(
                harm,
                if scenario==Scenario::DepartedCaster {(0,0)} else if scenario==Scenario::RevokedCaster {(1,i64::from(equipped_hit(scenario,output,0)))} else if residual {
                    let ticks = i64::from(field_residual_ticks(output));
                    (1 + ticks, (0..=field_residual_ticks(output)).map(|ordinal| i64::from(equipped_hit(scenario,output,ordinal))).sum())
                } else if harmful {
                    (1, i64::from(equipped_hit(scenario,output,0)))
                } else {
                    (0, 0)
                },
                "only legal entry and residual ticks have distinct durable occurrences"
            );
        }
    }
    if equipped(scenario) {
        let (lifecycle, charges, writes): (i16, Option<i32>, i64) = sqlx::query_as("SELECT i.lifecycle,t.charges,(SELECT count(*) FROM game_item_timed_state_writes w WHERE w.item_instance_id=i.item_instance_id) FROM game_item_instances i LEFT JOIN game_item_timed_states t USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid").bind(id(180).as_slice()).fetch_one(&harness.pool).await?;
        let spent = if matches!(scenario,Scenario::EquippedWrongSlot|Scenario::EquippedMantraAmmo) {0}else if matches!(scenario,Scenario::EquippedLastCharge|Scenario::EquippedPair) {1}else{1+field_residual_ticks(output)};
        assert_eq!(writes,i64::from(spent),"each applied field pulse consumes exactly one durable charge, including after unknown COMMIT");
        if scenario==Scenario::EquippedLastCharge { assert_eq!(lifecycle,2,"the last charge retires its real Item"); }
        else { assert_eq!(lifecycle,1); assert_eq!(charges,if spent==0 {None}else{Some(5-i32::try_from(spent)?)}); }
    }
    if scenario==Scenario::EquippedPair {
        let charges: i32=sqlx::query_scalar("SELECT charges FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(183).as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(charges,4,"both charged protectors spend once in the same zero-HP hit");
    }
    if scenario==Scenario::Equipped && output==2118 {
        // A genuine held protective Item and current victim fence are insufficient:
        // the charge also needs this transaction's own cycle, not a past receipt.
        let victim_session=crate::foundation::GameSessionId::decode(&id(110)).map_err(debug)?;
        let fence=read_field_setup_fence(&owner,victim_session).await?.item_fence();
        let permit=owner.spell_lane_permit().await.ok_or("proof permit")?;
        let target_actor=target.ok_or("proof victim")?;
        let position=owner.runtime.lock().await.read_actor_position(target_actor).map_err(debug)?.position();
        let address={let runtime=owner.runtime.lock().await;crate::durability::spell_items_abi::SpellGroundTarget::for_native_tile_read(&room,&runtime,position).map_err(debug)?};
        let stale:Vec<u8>=sqlx::query_scalar("SELECT occurrence FROM game_player_field_cycles WHERE victim=encode($1,'hex')::uuid ORDER BY occurred_at LIMIT 1").bind(id(101).as_slice()).fetch_one(&harness.pool).await?;
        let stale:[u8;32]=stale.try_into().map_err(|_|"cycle bytes")?;
        let charges_before:i32=sqlx::query_scalar("SELECT charges FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(180).as_slice()).fetch_one(&harness.pool).await?;
        for cycle in [stale,[0;32]] {
            let mut tx=crate::durability::item_mint_audit::Type2Transaction::open(harness.pool.begin().await?).await?;
            let read=crate::durability::spell_item_transaction::read_standing_player_tile_in_transaction(&mut tx,&permit,owner.root,owner.character,owner.holder,&fence,native.source_digest(),&address).await.map_err(debug)?;
            assert_eq!(owner.equipped_field_damage(&mut tx,&read,oteryn_gameplay_domain::domain::condition::DotElement::Fire,10,false,cycle).await?,5);
            let error=tx.into_inner().commit().await.expect_err("a charge cannot reuse a stale or absent field cycle");
            assert!(error.as_database_error().is_some_and(|error|matches!(error.code().as_deref(),Some("23514"|"23503"))));
            let after:i32=sqlx::query_scalar("SELECT charges FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(180).as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(after,charges_before,"rejected proof rolls back the timed Item write");
        }
    }
    drop(owner);
    drop(authority);
    drop(seal);
    harness.cleanup().await?;
    Ok(())
}

fn field_handle_from_wire(
    envelope: &oteryn_protocol_oteryn::WireEnvelopeView<'_>,
    header: &oteryn_protocol_oteryn::world_map::MapViewHeader,
    start: MovementLocalPosition,
) -> TestResult<Option<std::num::NonZeroU64>> {
    if envelope.message_type() != MessageType::StateDelta {
        return Ok(None);
    }
    let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload()).map_err(debug)?;
    if delta.domain_id != oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW {
        return Ok(None);
    }
    let delta = oteryn_protocol_oteryn::world_map::decode_world_map_delta(header, delta.payload)
        .map_err(debug)?;
    Ok(delta
        .tiles
        .iter()
        .find(|tile| tile.position.x == start.x + 1 && tile.position.y == start.y)
        .and_then(|tile| {
            tile.items.iter().find_map(|item| match item.origin {
                oteryn_protocol_oteryn::world_map::MapOrigin::Handle(handle) => Some(handle),
                _ => None,
            })
        }))
}

// Move through dispatch so each step publishes its map cursor. Directly committing two
// fixture moves can race the visibility cadence: a two-tile jump needs a map snapshot,
// which the live connection cannot send, so it disconnects before the machete command.
async fn move_field_caster(
    client: &mut tokio::io::DuplexStream,
    command_id: u64,
    direction: StepDirection,
) -> TestResult {
    let command = test_command(
        command_id,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        &encode_step_intent(direction),
    );
    client.write_u32(u32::try_from(command.len())?).await?;
    client.write_all(&command).await?;
    let mut moved = false;
    loop {
        let frame = read_frame(client).await?;
        let envelope = decode_wire_envelope(&frame).map_err(debug)?;
        assert_ne!(envelope.message_type(), MessageType::ProtocolError);
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(result.command_id, command_id);
            assert_eq!(
                decode_step_result(result.payload).map_err(debug)?,
                StepDisposition::Moved
            );
            moved = true;
        }
        // The result precedes moved_step_frames. Require its real map delta before
        // sending another move, independently of the periodic visibility timer.
        if moved && envelope.message_type() == MessageType::StateDelta {
            let delta =
                oteryn_protocol_oteryn::decode_state_delta(envelope.payload()).map_err(debug)?;
            if delta.domain_id == oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW {
                return Ok(());
            }
        }
    }
}

fn field_world(
    world: crate::foundation::WorldId,
    zone: u32,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{
        sector::{Attrs, Item, Tile},
        writer::{
            self, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector, Terrain,
            TerrainKind,
        },
    };
    let manifest = Manifest {
        subareas: Default::default(),
        format: writer::FORMAT.into(),
        min_reader_version: writer::VERSION,
        projection_class: "server".into(),
        compiler_version: "field-runes-test".into(),
        build_class: BuildClass::Production,
        identity: Identity {
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "rev-1".into(),
            ..Default::default()
        },
        world: Extent {
            min_x: 96,
            min_y: 96,
            max_x: 112,
            max_y: 112,
            floors: vec![-7],
        },
        palette: vec![PaletteEntry {
            family: Family::Terrain,
            key: "oteryn:item.tibia.i100".into(),
            id: 0,
            terrain: Some(Terrain {
                kind: TerrainKind::Ground,
                walkable: Some(true),
                ground_speed: Some(150),
            }),
        }],
        draft_areas: vec![],
        skipped_provisional_keys: vec![],
        dropped_teleports: vec![],
        spawns: Default::default(),
        npcs: Default::default(),
    };
    let mut tiles = Vec::new();
    for y in 96..112 {
        for x in 96..112 {
            tiles.push(Tile {
                x,
                y,
                flags: if (x, y) == (101, 98) { zone } else { 0 },
                house: 0,
                zones: vec![],
                items: vec![Item {
                    palette: 0,
                    depth: 0,
                    attrs: Attrs::default(),
                }],
            });
        }
    }
    let bytes = writer::write(
        &manifest,
        &[Sector {
            floor: -7,
            sx: 3,
            sy: 3,
            tiles,
        }],
        &Default::default(),
    )
    .map_err(debug)?;
    let pins = crate::map::BundlePins {
        digest: oteryn_world_bundle::bundle::read(&bytes)
            .map_err(debug)?
            .digest,
        project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
        world_schema_version: "world-schema-1".into(),
        content_revision: "rev-1".into(),
        production: false,
    };
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            bundle: pins,
            map_revision: crate::map::overlay::map_revision(&base),
            start: crate::map::overlay::TilePos {
                x: 100,
                y: 100,
                floor: -7,
            },
        },
        world,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}

async fn qualify_creature_entry(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    start: MovementLocalPosition,
    output: u32,
) -> TestResult<crate::foundation::ExactActorRef> {
    let creature = {
        let mut runtime = owner.runtime.lock().await;
        let native = owner
            .active_generation
            .and_then(|active| active.native_gameplay())
            .ok_or("native content")?;
        // Keep damage/replay observations deterministic across the Forge population tick.
        let mut policy = native
            .policies()
            .policies()
            .iter()
            .find(|policy| policy.definition_key == "oteryn:creature.cyclops")
            .ok_or("target creature policy")?
            .as_ref()
            .clone();
        policy.forge_difficulty = None;
        let policies = crate::content::creature_policy::CompiledCreaturePolicies::from_active_artifact(
            native.source_digest(),
            vec![policy],
        )
        .map_err(debug)?;
        runtime.install_companion_policies(policies).map_err(debug)?;
        let pin = runtime.content_pin().clone();
        runtime
            .realize_native_qualification_spawn(
                pin,
                "oteryn:creature.cyclops",
                MovementLocalPosition {
                    x: start.x + 1,
                    y: start.y + 1,
                    ..start
                },
            )
            .map_err(debug)?
            .actor
    };
    step_creature_on_field(owner, creature, crate::movement::CardinalStep::North).await?;
    owner
        .states
        .spell_states
        .lock()
        .await
        .next_monster_ai_pass_us = 0;
    owner.drain_monster_melee().await;
    let snapshot = owner
        .runtime
        .lock()
        .await
        .companion_snapshot(creature)
        .map_err(debug)?;
    assert_eq!(
        snapshot.health,
        snapshot.maximum_health - field_creature_hit(output),
        "the real creature entry owner applies its source mitigation and elemental resistance"
    );
    assert_eq!(
        snapshot.state.conditions.instances().is_empty(),
        output == 2122,
        "the source energy recipe is exhausted by its immediate pulse; fire and poison retain residual segments"
    );
    step_creature_on_field(owner, creature, crate::movement::CardinalStep::South).await?;
    Ok(creature)
}

async fn step_creature_on_field(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    creature: crate::foundation::ExactActorRef,
    direction: crate::movement::CardinalStep,
) -> TestResult {
    use crate::durability::spell_item_transaction::{
        assert_spell_item_scope_in_transaction, begin_spell_owner_transaction,
    };
    let permit = owner
        .spell_lane_permit()
        .await
        .ok_or("creature field lane")?;
    let mut runtime = owner.runtime.lock().await;
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native content")?;
    let room = owner.rooms.gate_field_step_ingress().ok_or("field room")?;
    let pass = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            match owner.root.try_issue_semantic_pass() {
                Ok(pass) => break Ok(pass),
                Err(crate::durability::DurabilityError::RootUnavailable) => {
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await
                }
                Err(error) => break Err(debug(error)),
            }
        }
    })
    .await
    .map_err(debug)??;
    let mut context = (owner, &mut *runtime, &permit);
    pass.run_with_context(&mut context, |holder, deadline, context| {
        Box::pin(async move {
            let (owner, runtime, permit) = context;
            let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
            let binding = runtime.binding();
            let authority = assert_spell_item_scope_in_transaction(
                &mut tx,
                permit,
                owner.root,
                owner.character,
                owner.holder,
                crate::foundation::RuntimeScopeRefV1::channel(
                    binding.world_id(),
                    binding.channel_id(),
                ),
                binding.scope_generation().get(),
            )
            .await
            .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
            let objects = owner.door.lock().await;
            let mut map = crate::movement::monster_map::CurrentMonsterMap::new(
                &mut tx, &authority, runtime, room, &objects, native,
            );
            let before = runtime
                .read_actor_position(creature)
                .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
            let (dx, dy) = crate::domain::direction::CARDINAL_DIRECTION_DELTAS[direction as usize];
            let at = before.position();
            let destination = MovementLocalPosition {
                x: at.x + dx,
                y: at.y + dy,
                ..at
            };
            assert_eq!(map.read(runtime, destination).await, Some(true));
            assert!(map.validate_transaction(runtime).await);
            if direction == crate::movement::CardinalStep::North {
                assert_eq!(
                    map.view().field_path_cost(destination, [false; 3]),
                    Some(180),
                    "the current path consumer observes the real harmful field surcharge"
                );
                assert_eq!(
                    map.view().field_path_cost(destination, [true; 3]),
                    Some(0),
                    "a profile that walks on fire pays no field surcharge"
                );
            }

            map.view()
                .step(runtime, room.movement_cells(), creature, before, direction)
                .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
            drop(map);
            tx.rollback().await?;
            Ok(())
        })
    })
    .await
    .map_err(debug)
}

async fn qualify_creature_residual(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    creature: crate::foundation::ExactActorRef,
    output: u32,
) -> TestResult {
    let before = owner
        .runtime
        .lock()
        .await
        .companion_snapshot(creature)
        .map_err(debug)?
        .health;
    owner.clock_origin -= std::time::Duration::from_secs(11);
    owner
        .states
        .spell_states
        .lock()
        .await
        .next_monster_ai_pass_us = 0;
    owner.drain_monster_melee().await;
    let after = owner
        .runtime
        .lock()
        .await
        .companion_snapshot(creature)
        .map_err(debug)?
        .health;
    assert_eq!(
        after,
        before - field_creature_hit(output) * i64::from(field_residual_ticks(output)),
        "the real periodic creature owner consumes residual damage after the durable field decays"
    );
    Ok(())
}

async fn projected_field_count(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    source_id: u32,
) -> TestResult<i64> {
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native gameplay")?;
    let policy = native
        .item_policy_for_source_id(source_id)
        .ok_or("field policy")?;
    let index = native.item_index().ok_or("item index")?;
    let reference = index
        .definition_ref(
            "Item",
            &policy.record().production_definition.production_key,
            &policy.record().production_definition.revision_ref,
        )
        .ok_or("field definition")?;
    let mut map = crate::gameplay_transport::world_map::SessionMapView::default();
    let mut items = crate::gameplay_transport::item_view::SessionItemView::resume(
        crate::gameplay_transport::connection::SessionContinuity::FRESH.item_view,
    )
    .with_map_view();
    let update = owner
        .observe_world_map(actor, &mut map, &mut items)
        .await
        .ok_or("map consumer unavailable")?
        .map_err(debug)?
        .ok_or("map snapshot")?;
    let crate::gameplay_transport::world_map::MapUpdate::Snapshot(snapshot) = update else {
        return Err("expected fresh snapshot".into());
    };
    let snapshot = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(&snapshot.payload)
        .map_err(debug)?;
    let mut count = 0;
    for tile in snapshot.tiles {
        for item in tile.items {
            if item.definition == oteryn_protocol_oteryn::world_map::MapDefinition::Item(reference)
            {
                assert_eq!(
                    tile.position.floor, -7,
                    "runtime floor converts to the bundle floor once"
                );
                count += 1;
            }
        }
    }
    Ok(count)
}

async fn qualify_safe_field_after_owner_loss(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    creator: crate::foundation::ExactActorRef,
    peer: crate::foundation::ExactActorRef,
) -> TestResult {
    assert_eq!(
        owner
            .commit_control_loss(
                admitted.game_session_id,
                creator,
                admitted.controller.ok_or("creator controller")?
            )
            .await,
        crate::gameplay_transport::connection::ControlLossResult::Recorded
    );
    let session = crate::foundation::GameSessionId::decode(&id(110)).map_err(debug)?;
    let health = player_health(owner, peer, session).await?;
    for (stage, definition, direction) in [
        (None, 21465, StepDirection::West),
        (Some(1), 2132, StepDirection::South),
        (Some(2), 2133, StepDirection::South),
    ] {
        if let Some(stage) = stage {
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE game_spell_field_temporal_schedules SET expires_at_unix_ms=source_stage+1 WHERE source_stage=$1").bind(stage).execute(&mut *tx).await?;
            tx.commit().await?;
            owner
                .states
                .spell_states
                .lock()
                .await
                .next_item_deadline_pass_us = 0;
            owner.drain_source_item_deadlines().await;
        }
        assert_eq!(
            projected_field_count(owner, peer, definition).await?,
            1,
            "the real map consumer follows the original field's transform chain"
        );
        let (entry, _) = owner
            .ordinary_paced_step(peer, session, direction, None, None)
            .await;
        assert_eq!(
            entry.disposition,
            StepDisposition::Moved,
            "safe field phase {definition} remains traversable without its creator"
        );
        assert_eq!(player_health(owner, peer, session).await?, health);
        let (leave, _) = owner
            .ordinary_paced_step(peer, session, StepDirection::North, None, None)
            .await;
        assert_eq!(leave.disposition, StepDisposition::Moved);
    }
    let harm: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_pvp_harm")
        .fetch_one(&harness.pool)
        .await?;
    assert_eq!(
        harm, 0,
        "safe decay phases never become player damage or attribution"
    );
    Ok(())
}

async fn wait_field_client(
    client: &mut tokio::io::DuplexStream,
    duration: std::time::Duration,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + duration;
    loop {
        let frame = match tokio::time::timeout_at(deadline, read_frame(client)).await {
            Ok(frame) => frame?,
            Err(_) => return Ok(()),
        };
        let envelope = decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == MessageType::LivenessProbe {
            let probe =
                oteryn_protocol_oteryn::decode_liveness_probe(envelope.payload()).map_err(debug)?;
            let ack = oteryn_protocol_oteryn::encode_liveness_ack(1, probe, 0).map_err(debug)?;
            client.write_u32(u32::try_from(ack.len())?).await?;
            client.write_all(&ack).await?;
        }
    }
}

async fn player_health(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
) -> TestResult<u32> {
    let runtime = owner.runtime.lock().await;
    Ok(owner
        .states
        .spell_states
        .lock()
        .await
        .get(&runtime, actor, session)
        .ok_or("player state")?
        .vitals()
        .health)
}

#[allow(clippy::too_many_arguments)]
async fn drive_player_field(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    target: crate::foundation::ExactActorRef,
    start: MovementLocalPosition,
    scenario: Scenario,
    harmful: bool,
    output: u32,
) -> TestResult {
    let creator_session = admitted.game_session_id;
    let session = crate::foundation::GameSessionId::decode(&id(110)).map_err(debug)?;
    let mut admitted = admitted;
    admitted.game_session_id = session;
    admitted.runtime_actor = Some(target);
    let selected = admitted.continuity.selected_capabilities;
    admitted.continuity = crate::gameplay_transport::connection::SessionContinuity::FRESH;
    admitted.continuity.selected_capabilities = selected;
    let current = FreshAdmissionStore::from_root(harness.root.clone())
        .current_session_at(session)
        .await?
        .0;
    let account:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(account_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(current.commit().character_id().as_bytes().as_slice()).fetch_one(&harness.pool).await?;
    admitted.controller = Some(crate::gameplay_transport::connection::ControllerBinding {
        account_id: account.try_into().map_err(|_| "account bytes")?,
        transport: current
            .current_transport()
            .ok_or("current target transport")?,
    });
    admitted.continuity.connection_generation = current.current_connection_generation().get();
    admitted.item_fence = item_fence_of(current);
    if scenario == Scenario::Death {
        owner.admit_progression_session(&mut admitted).await;
        assert!(
            owner.progression_binding(session).is_some(),
            "the actual death owner requires its admitted progression binding"
        );
    }
    if scenario == Scenario::DepartedCaster {
        let creator = owner.states.spell_states.lock().await.actors.iter().find(|(_,held,_)|*held==creator_session).map(|(actor,_,_)|*actor).ok_or("creator")?;
        owner.runtime.lock().await.record_control_loss(creator,creator_session,oteryn_gameplay_entities::ControlLossMark {epoch:1,grace_deadline:i64::MAX}).map_err(debug)?;
    }
    let original = player_health(owner, target, session).await?;
    let initial_damage = if matches!(scenario,Scenario::Aged|Scenario::DepartedCaster) {
        20
    } else if harmful {
        equipped_hit(scenario,output,0)
    } else {
        0
    };
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async { serve_admitted(&mut server, admitted, owner, FIELD_SCENARIO_LIVENESS).await };
    let driving = async {
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        if scenario == Scenario::Aged {
            wait_field_client(&mut client, std::time::Duration::from_secs(6)).await?;
        }
        if scenario == Scenario::EquippedRollback {
            // A concurrent field owner retains the lane and its sole ready SQL
            // holder. STEP must wait for that turn before any preflight read.
            let permit = owner
                .spell_lane_permit()
                .await
                .ok_or("contended field lane")?;
            let pass = owner.root.try_issue_semantic_pass().map_err(debug)?;
            {
                let step =
                    owner.ordinary_paced_step(target, session, StepDirection::West, None, None);
                tokio::pin!(step);
                let polled = std::future::poll_fn(|cx| {
                    std::task::Poll::Ready(std::future::Future::poll(step.as_mut(), cx))
                })
                .await;
                assert!(
                    polled.is_pending(),
                    "STEP must await the occupied field lane, got {polled:?}"
                );
            }
            pass.run(|_, _| Box::pin(async { Ok(()) }))
                .await
                .map_err(debug)?;
            drop(permit);
        }
        if scenario == Scenario::Death {
            let (outcome, _) = owner
                .ordinary_paced_step(target, session, StepDirection::West, None, None)
                .await;
            assert_eq!(outcome.disposition, StepDisposition::Moved);
            let death = {
                let runtime = owner.runtime.lock().await;
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .player_death(&runtime, target, session)
                    .ok_or("field lethal receipt")?
            };
            assert_eq!(player_health(owner, target, session).await?, 0);
            let creator = owner
                .states
                .spell_states
                .lock()
                .await
                .actors
                .iter()
                .find(|(_, held, _)| *held == creator_session)
                .map(|(actor, _, _)| *actor)
                .ok_or("creator actor")?;
            let (moved, _) = owner
                .ordinary_paced_step(creator, creator_session, StepDirection::West, None, None)
                .await;
            assert_eq!(
                moved.disposition,
                StepDisposition::Moved,
                "the real respawn landing must be free"
            );
            // Respawn is retryable, and the admitted connection's cadence can
            // settle the same occurrence concurrently. Require the durable
            // receipt and actual refill, not one caller's transient return.
            tokio::time::timeout(std::time::Duration::from_secs(10), async {
                loop {
                    let _ = owner.respawn_after_death(target, session, death).await;
                    if player_health(owner, target, session).await? > 0 {
                        return Ok::<_, Box<dyn std::error::Error>>(());
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                }
            })
            .await??;
            {
                let runtime = owner.runtime.lock().await;
                assert_eq!(
                    runtime
                        .read_actor_position(target)
                        .map_err(debug)?
                        .position(),
                    runtime.respawn_position()
                );
                assert!(
                    owner
                        .states
                        .spell_states
                        .lock()
                        .await
                        .player_death(&runtime, target, session)
                        .is_none()
                );
            }
            let deaths:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_death_receipts WHERE character_id=encode($1,'hex')::uuid").bind(id(101).as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(deaths, 1, "the real field death is settled durably once");
            let points:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_unjustified_points WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(
                points, 1,
                "field harm receipts supply the real death ledger"
            );
            client.shutdown().await?;
            return Ok(());
        }
        for (id, direction, replay) in [
            (1, StepDirection::West, false),
            (1, StepDirection::West, true),
            (2, StepDirection::North, false),
        ] {
            if id == 2 {
                // Drive the actor-owned pacing clock instead of guessing its deadline. If the
                // fixed sleep wakes just before the deadline, the connection buffers this step
                // and its cadence can race the field owner for the next semantic pass.
                if let Some(due) = owner.actor_step_deadline_now(target).await {
                    tokio::time::sleep_until(due).await;
                }
            }
            let frame = test_command(
                id,
                COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                &encode_step_intent(direction),
            );
            client.write_u32(u32::try_from(frame.len())?).await?;
            client.write_all(&frame).await?;
            loop {
                let frame = read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(
                        decode_step_result(result.payload).map_err(debug)?,
                        StepDisposition::Moved,
                        "scenario={scenario:?} step={id} replay={replay}"
                    );
                    if replay {
                        assert_eq!(result.status, CommandStatus::DuplicateReplay);
                    }
                    break;
                }
            }
            drop(
                owner
                    .spell_lane_permit()
                    .await
                    .ok_or("player field reconciliation")?,
            );
            assert_eq!(
                player_health(owner, target, session).await?,
                original - initial_damage,
                "the actual player entry owner enforces protection and World policy; replay does no extra damage"
            );
        }
        let harm:(i64,i64)=sqlx::query_as("SELECT count(*),COALESCE(sum(damage),0)::bigint FROM game_character_pvp_harm WHERE octet_length(field_occurrence)=32").fetch_one(&harness.pool).await?;
        assert_eq!(
            harm,
            if harmful && scenario!=Scenario::DepartedCaster {
                (1, i64::from(equipped_hit(scenario,output,0)))
            } else {
                (0, 0)
            }
        );
        if matches!(scenario, Scenario::PzTick | Scenario::NoPvpTick) {
            let (outcome, _) = owner
                .ordinary_paced_step(target, session, StepDirection::North, None, None)
                .await;
            assert_eq!(outcome.disposition, StepDisposition::Moved);
            assert_eq!(
                owner
                    .runtime
                    .lock()
                    .await
                    .read_actor_position(target)
                    .map_err(debug)?
                    .position(),
                MovementLocalPosition {
                    x: start.x + 1,
                    y: start.y - 2,
                    floor: start.floor
                }
            );
        }
        if scenario == Scenario::RevokedCaster {
            let creator = owner
                .states
                .spell_states
                .lock()
                .await
                .actors
                .iter()
                .find(|(_, held, _)| *held == creator_session)
                .map(|(actor, _, _)| *actor)
                .ok_or("creator")?;
            let current = FreshAdmissionStore::from_root(harness.root.clone())
                .current_session_at(
                    crate::foundation::GameSessionId::decode(&id(50)).map_err(debug)?,
                )
                .await?
                .0;
            let controller = crate::gameplay_transport::connection::ControllerBinding {
                account_id: id(40),
                transport: current.current_transport().ok_or("creator transport")?,
            };
            assert_eq!(
                owner
                    .commit_control_loss(
                        crate::foundation::GameSessionId::decode(&id(50)).map_err(debug)?,
                        creator,
                        controller
                    )
                    .await,
                crate::gameplay_transport::connection::ControlLossResult::Recorded
            );
        }
        let skull:bool=sqlx::query_scalar("SELECT COALESCE((SELECT white_skull_until>clock_timestamp() FROM game_character_pvp_state WHERE character_id=encode($1,'hex')::uuid),false)").bind(id(41).as_slice()).fetch_one(&harness.pool).await?;
        assert!(
            skull == (harmful && !matches!(scenario,Scenario::Hardcore|Scenario::DepartedCaster)),
            "only incidental field harm gives a skull, including with secure mode on"
        );
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (end, result) = join_two(serving, driving).await;
    result?;
    assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
    Ok(())
}

// These are the actual map readers used by monster movement/pathing and sight,
// in the same sealed scope transaction used by the production owner cycle.
async fn qualify_barrier_consumers(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    start: MovementLocalPosition,
    definition: u32,
) -> TestResult {
    use crate::durability::spell_item_transaction::{
        assert_spell_item_scope_in_transaction, begin_spell_owner_transaction,
    };
    let permit = owner
        .spell_lane_permit()
        .await
        .ok_or("barrier consumer lane")?;
    let runtime = owner.runtime.lock().await;
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native content")?;
    let room = owner.rooms.gate_field_step_ingress().ok_or("field room")?;
    let pass = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            match owner.root.try_issue_semantic_pass() {
                Ok(pass) => break Ok(pass),
                Err(crate::durability::DurabilityError::RootUnavailable) => {
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await
                }
                Err(error) => break Err(debug(error)),
            }
        }
    })
    .await
    .map_err(debug)??;
    let mut context = (owner, &*runtime, &permit);
    pass.run_with_context(&mut context, |holder, deadline, context| {
        Box::pin(async move {
            let (owner, runtime, permit) = context;
            let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
            let binding = runtime.binding();
            let scope = crate::foundation::RuntimeScopeRefV1::channel(
                binding.world_id(),
                binding.channel_id(),
            );
            let authority = assert_spell_item_scope_in_transaction(
                &mut tx,
                permit,
                owner.root,
                owner.character,
                owner.holder,
                scope,
                binding.scope_generation().get(),
            )
            .await
            .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
            let objects = owner.door.lock().await;
            let mut map = crate::movement::monster_map::CurrentMonsterMap::new(
                &mut tx, &authority, runtime, room, &objects, native,
            );
            let target = MovementLocalPosition {
                x: start.x + 1,
                ..start
            };
            let behind = MovementLocalPosition {
                x: start.x + 2,
                ..start
            };
            assert_eq!(
                map.read(runtime, target).await,
                Some(matches!(definition, 10181 | 10182)),
                "monster pathing observes the effective barrier collision"
            );
            map.read_sight(runtime, start, behind).await;
            assert!(map.validate_transaction(runtime).await);
            assert_eq!(
                map.view().sight(start, behind),
                !matches!(definition, 2128 | 10181),
                "Magic Wall blocks the real sight/projectile ray; Wild Growth does not"
            );
            tx.rollback().await?;
            Ok(())
        })
    })
    .await
    .map_err(debug)?;
    Ok(())
}

// A real assignment replacement fences the predecessor. Rebuild its volatile
// runtime and publish readiness through the owning API before the expiry pass.
async fn qualify_field_restart(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
) -> TestResult<crate::foundation::ExactActorRef> {
    use crate::durability::runtime_scope_assignment::{
        AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
        RuntimeScopeAssignmentWriter,
    };
    use crate::foundation::admission_authority_publication::*;
    let scope = crate::foundation::RuntimeScopeRefV1::channel(owner.world_id, owner.channel_id);
    let predecessor = owner
        .root
        .read_runtime_scope_predecessor(scope)
        .await
        .map_err(debug)?
        .ok_or("scope predecessor")?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,2)").bind(owner.world_id.as_bytes().as_slice()).bind(owner.channel_id.as_bytes().as_slice()).execute(&harness.pool).await?;
    let writer = RuntimeScopeAssignmentWriter::open(owner.root.clone(), "sequencer-writer")
        .await
        .map_err(debug)?;
    let AssignmentOutcome::Committed(replacement) = writer
        .submit(&AssignmentRequest {
            operation_key: OperationKey::from_bytes([99; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            command: AssignmentCommand::Replace {
                scope,
                predecessor,
                target: harness.node.fact(),
            },
        })
        .await
        .map_err(debug)?
    else {
        return Err("scope replacement refused".into());
    };
    let generation = replacement.assignment.ownership_generation;
    assert_eq!(generation, predecessor.ownership_generation + 1);
    let publication_revision = replacement
        .fenced_publication_revision
        .ok_or("replacement runtime fence")?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint")
            .fetch_one(&harness.pool)
            .await?;
    let guards = crate::durability::admission_authority_guards::AdmissionGuardStore::from_root(
        owner.root.clone(),
    );
    let mut change = guards
        .load(&[AdmissionAuthorityGuardKeyV1::Runtime(scope)])
        .await?
        .into_iter()
        .next()
        .flatten()
        .ok_or("fenced Runtime guard")?;
    assert_eq!(change.publication_revision, publication_revision);
    change.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
        expected_publication_revision: publication_revision,
    };
    change.publication_revision += 1;
    change.source.source_revision += 1;
    change.source.decision_identity = "field-runtime-ready-2".into();
    change.source.source_observed_at = now;
    let AdmissionAuthorityGuardStateV1::Runtime {
        ownership_generation,
        ready,
        ..
    } = &mut change.state
    else {
        return Err("Runtime guard shape".into());
    };
    assert_eq!(*ownership_generation, generation);
    *ready = true;
    let mut publisher = super::fresh_fixture::fresh::Source::new(now)?;
    publisher.rows = vec![change];
    let publication = AdmissionAuthorityPublicationV1::prepare(&publisher, now).map_err(debug)?;
    assert_eq!(
        owner
            .root
            .publish_runtime_readiness(&harness.node, &publication)
            .await
            .map_err(debug)?,
        crate::durability::admission_authority_guards::GuardPublicationDisposition::Applied
    );
    let fact = harness.node.fact();
    let mut runtime = owner.runtime.lock().await;
    let mut restarted = crate::foundation::ChannelRuntimeV1::from_committed_assignment(
        owner.world_id,
        owner.channel_id,
        fact.node_id(),
        fact.registration_revision(),
        generation,
        replacement.assignment.source_revision,
        &replacement.assignment.decision_identity,
        2,
        runtime.content_pin().clone(),
    )
    .map_err(debug)?;
    let session = crate::foundation::GameSessionId::decode(&id(110)).map_err(debug)?;
    let reserved = restarted.reserve_fresh_session(session).map_err(debug)?;
    let peer = restarted.commit_fresh_session(reserved).map_err(debug)?;
    restarted
        .initialize_first_entry_position(peer)
        .map_err(debug)?;
    *runtime = restarted;
    drop(runtime);
    owner.required_capabilities = BoundRequiredCapabilities::for_channel_boot(
        &[11, 30],
        scope,
        generation,
        "policy-1",
        BUNDLE_WORLD_OFFERED_CAPABILITIES,
    )
    .map_err(debug)?;
    assert_eq!(
        projected_field_count(owner, peer, 2133).await?,
        0,
        "an old-generation field is never projected into the new channel incarnation"
    );
    owner
        .states
        .spell_states
        .lock()
        .await
        .next_item_deadline_pass_us = 0;
    owner.drain_source_item_deadlines().await;
    let retired: i64 = sqlx::query_scalar("SELECT count(*) FROM game_field_restart_retirements")
        .fetch_one(&harness.pool)
        .await?;
    assert_eq!(
        retired, 1,
        "the current fenced owner retires a transformed field without adopting its lineage"
    );
    owner
        .states
        .spell_states
        .lock()
        .await
        .next_item_deadline_pass_us = 0;
    owner.drain_source_item_deadlines().await;
    let again: i64 = sqlx::query_scalar("SELECT count(*) FROM game_field_restart_retirements")
        .fetch_one(&harness.pool)
        .await?;
    assert_eq!(
        again, retired,
        "retrying the recovery pass never retires twice"
    );
    Ok(peer)
}
