//! PAR-M10: Crystal step-in hooks on committed creature steps (Crystal `201189f8`). The player
//! half of the trap table stays on the player StepIn owner (`special_movement.rs`).
//!
//! - `data/scripts/movements/trap.lua:12-45`: a Starving Wolf on the magic wolf trap 12368, on a
//!   tile outside a protection zone (`:23`) with at most one 3482 (`:27`), is removed and a
//!   captured wolf 12369 is created on the tile (`:39-42`).
//! - `data/scripts/movements/thornfire_wolf.lua:3-14`: a Thornfire Wolf on 12721 is removed and a
//!   Crystal Wolf is created on its cell.
//!
//! - `data/scripts/movements/trap.lua:23-44`: any other creature on a spike trap (2145, 2147,
//!   3482, 3944, 44529) outside a protection zone, with at most one 3482, closes it (`:35`) and
//!   takes `doTargetCombatHealth(0, ...)` (`:44`): physical, or earth for 3944, drawn in the
//!   trap's range with no effect (`CONST_ME_NONE`). The hit is one unsourced field-entry
//!   condition through the creature condition owner (`commit_creature_field_entry` and
//!   `tick_source_creature_conditions`, the field damage path), so creature damage immunities,
//!   mitigation, resistances and death apply as for a field.
//!
//! The capture's STUN (`trap.lua:40`) and the replacement's ICEATTACK (`thornfire_wolf.lua:10`)
//! go to the observers through capability 262 (`observer_presentation`).
use super::special_movement::{TileEntries, Trap, tile_entries, transform, trap};
use crate::content::ReferenceResistanceKind;
use crate::durability::item_mint::{
    GroundPlacement, ItemMintCause, ItemMintRequest, TypedDefinitionRef,
};
use crate::foundation::{
    ChannelRuntimeV1, CreatureDeathOccurrenceKey, ExactActorRef, MovementLocalPosition,
};
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use crate::map::boot::BundleMap;
use oteryn_gameplay_domain::domain::condition::{ConditionValues, DotElement};
use oteryn_gameplay_entities::carrier::runtime_actor_periodic::CreatureFieldEntry;
use oteryn_gameplay_foundation::foundation::condition::ConditionDefinition;
use oteryn_protocol_oteryn::observer_presentation::{MAGIC_EFFECT_ICEATTACK, MAGIC_EFFECT_STUN};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

/// `trap.lua:6`: the magic wolf trap.
const MAGIC_WOLF_TRAP: u16 = 12368;
/// `trap.lua:27`: a tile holding more than one 3482 runs no trap.
const STACKED_TRAP: u16 = 3482;
/// `thornfire_wolf.lua:17`.
const THORNFIRE_GROUND: u16 = 12721;
const STARVING_WOLF: &str = "oteryn:creature.starving_wolf";
const THORNFIRE_WOLF: &str = "oteryn:creature.thornfire_wolf";
const CRYSTAL_WOLF: &str = "oteryn:creature.crystal_wolf";
/// `trap.lua:42`: the captured wolf, one item.
pub(in crate::gameplay_transport) const CAPTURED_WOLF_KEY: &str = "oteryn:item.tibia.i12369";
pub(in crate::gameplay_transport) const CAPTURED_WOLF_REVISION: &str = "definition-r1";
const MAGIC_WOLF_TRAP_KEY: &str = "oteryn:item.tibia.i12368";
/// The MINT purpose of a capture; its cause is the removed wolf's end key and the trap.
const CAPTURE_PURPOSE: &str = "MAGIC_WOLF_TRAP_CAPTURE";
/// The Ground `corpse_ref` of a capture: it belongs to no corpse.
const CAPTURE_GROUND_REF: &[u8] = b"creature-step-capture-v1";

/// What the hooks changed on one owner turn: the creatures they removed (for the think table) and
/// the Ground MINTs to commit once the Channel guards are released.
#[derive(Debug, Default)]
pub(in crate::gameplay_transport) struct CreatureStepHooks {
    pub removed: Vec<ExactActorRef>,
    pub captures: Vec<ItemMintRequest>,
    /// The magic effects to present at each hook's cell (capability 262), in hook order.
    pub effects: Vec<(MovementLocalPosition, u32)>,
}

/// Whether the active content admits the captured wolf 12369 as a native item that may be minted
/// on the ground; `trap.lua:42` creates it on the trap's tile.
pub(in crate::gameplay_transport) fn capture_admitted(
    content: &crate::content::native_gameplay::NativeGameplayState,
) -> bool {
    content
        .item_policy(CAPTURED_WOLF_KEY, CAPTURED_WOLF_REVISION)
        .is_some_and(|policy| {
            let admission = &policy.record().admission;
            admission.materializable
                && admission
                    .legal_destinations
                    .contains(&crate::content::native_gameplay::NativeItemDestination::Ground)
        })
}

/// Applies the creature step-in hooks to the committed creature steps of one pass. A capture runs
/// only when `capture_admitted`, that is, while the active content admits the captured wolf as a
/// native item: without it the wolf is not removed, so no capture is lost. A creature that is not
/// removed then closes and takes the hit of each spike trap on its cell.
pub(in crate::gameplay_transport) fn apply_creature_step_hooks(
    runtime: &mut ChannelRuntimeV1,
    states: &ChannelSpellStates,
    map: &BundleMap,
    steps: Vec<(ExactActorRef, MovementLocalPosition)>,
    capture_admitted: bool,
    now: u64,
) -> CreatureStepHooks {
    let mut hooks = CreatureStepHooks::default();
    for (actor, at) in steps {
        let Some(TileEntries {
            protection_zone,
            entries,
        }) = tile_entries(map, at)
        else {
            continue;
        };
        let Ok(snapshot) = runtime.companion_snapshot(actor) else {
            continue;
        };
        let has = |id: u16| entries.iter().any(|(_, entry)| entry.appearance_id == id);
        let stacked = entries
            .iter()
            .filter(|(_, entry)| entry.appearance_id == STACKED_TRAP)
            .count();
        match snapshot.state.policy.definition_key.as_str() {
            STARVING_WOLF
                if capture_admitted && !protection_zone && stacked <= 1 && has(MAGIC_WOLF_TRAP) =>
            {
                let Ok((end, _)) = runtime.remove_stepped_creature(actor, at, STARVING_WOLF, None)
                else {
                    continue;
                };
                hooks.removed.push(actor);
                hooks.captures.push(capture_request(runtime, end, at));
                // trap.lua:40.
                hooks.effects.push((at, MAGIC_EFFECT_STUN));
            }
            THORNFIRE_WOLF if has(THORNFIRE_GROUND) => {
                let replaced =
                    runtime.remove_stepped_creature(actor, at, THORNFIRE_WOLF, Some(CRYSTAL_WOLF));
                if replaced.is_ok() {
                    hooks.removed.push(actor);
                    // thornfire_wolf.lua:10.
                    hooks.effects.push((at, MAGIC_EFFECT_ICEATTACK));
                }
            }
            _ => {}
        }
        if hooks.removed.contains(&actor) || protection_zone || stacked > 1 {
            continue;
        }
        for (key, entry) in &entries {
            // The magic wolf trap 12368 neither transforms nor damages (`trap.lua:6`).
            let Some(rule) = trap(entry.appearance_id).filter(|rule| rule.maximum_damage > 0)
            else {
                continue;
            };
            // trap.lua:35 transforms before the hit (:44).
            if transform(map, *key, rule.closed) {
                spike_trap_hit(runtime, states, actor, *key, rule, now);
            }
        }
    }
    hooks
}

/// `trap.lua:44` on a creature: one draw in the trap's range, dealt at once as an unsourced
/// field-entry damage condition on the creature's current cell.
fn spike_trap_hit(
    runtime: &mut ChannelRuntimeV1,
    states: &ChannelSpellStates,
    actor: ExactActorRef,
    key: u64,
    rule: Trap,
    now: u64,
) {
    let Ok(snapshot) = runtime.companion_snapshot(actor) else {
        return;
    };
    if !snapshot.state.conditions.accepts_time(now) {
        return;
    }
    let digest = Sha256::new()
        .chain_update(b"oteryn:creature-spike-trap:v1")
        .chain_update(actor.placement_identity())
        .chain_update(snapshot.position_revision.to_be_bytes())
        .chain_update(key.to_be_bytes())
        .finalize();
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&digest[..16]);
    let occurrence = DecisionOccurrenceId::from_bytes(bytes);
    let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
    let Ok(draw) = deterministic_decision_u64(&root, occurrence, "movement.trap.damage", 0) else {
        return;
    };
    let damage = crate::spell::uniform_draw(
        draw,
        i64::from(rule.minimum_damage),
        i64::from(rule.maximum_damage),
    );
    let Ok(damage) = u32::try_from(damage) else {
        return;
    };
    let element = match rule.element {
        ReferenceResistanceKind::Earth => DotElement::Poison,
        _ => DotElement::Bleeding,
    };
    let Some(definition) = ConditionDefinition::new(
        "environment.spike_trap",
        1,
        ConditionValues::DamageOverTime {
            element,
            total_min: damage,
            total_max: damage,
            per_tick: damage,
            interval_ms: 2_000,
            delayed: false,
        },
    ) else {
        return;
    };
    let entry = CreatureFieldEntry {
        definition,
        source: None,
        lineage: None,
        occurrence,
    };
    if runtime
        .commit_creature_field_entry(actor, now, Some(&entry))
        .is_err()
    {
        return;
    }
    let _ = states.tick_source_creature_conditions(
        runtime,
        actor,
        crate::foundation::TickFacts {
            in_protection_zone: false,
            standing_on_field: None,
        },
        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now),
    );
}

fn capture_request(
    runtime: &ChannelRuntimeV1,
    end: CreatureDeathOccurrenceKey,
    at: MovementLocalPosition,
) -> ItemMintRequest {
    let pin = runtime.content_pin();
    let hex = |bytes: [u8; 32]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let content_revision = format!("sha256:{}", hex(pin.server_artifact_digest()));
    let mut spatial_position = Vec::with_capacity(10);
    spatial_position.extend_from_slice(&at.x.to_be_bytes());
    spatial_position.extend_from_slice(&at.y.to_be_bytes());
    spatial_position.extend_from_slice(&at.floor.to_be_bytes());
    let definition = |key: &str| TypedDefinitionRef {
        family: "Item".to_owned(),
        production_key: key.to_owned(),
        revision_ref: CAPTURED_WOLF_REVISION.to_owned(),
    };
    ItemMintRequest {
        cause: ItemMintCause::from_creature_death(
            end,
            definition(MAGIC_WOLF_TRAP_KEY),
            CAPTURE_PURPOSE.to_owned(),
            0,
        ),
        item: definition(CAPTURED_WOLF_KEY),
        quantity: 1,
        ground: GroundPlacement {
            spatial_position,
            corpse_ref: CAPTURE_GROUND_REF.to_vec(),
            map_revision: format!("sha256:{}", hex(pin.map_revision_digest())),
            content_revision: content_revision.clone(),
            native_room_placement_context: pin.frame_binding_digest().to_vec(),
        },
        content_revision,
        ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION.to_owned(),
        sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.to_owned(),
    }
}

impl crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_> {
    /// Commits the captured-wolf Ground MINTs of one pass, after every Channel guard is released
    /// and under the pass's Spell lane permit (SPELL-LOCK-2 §1.2). A MINT is idempotent per cause,
    /// so a failed one changes nothing durable.
    pub(in crate::gameplay_transport) async fn commit_creature_step_captures(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        captures: Vec<ItemMintRequest>,
    ) {
        for request in captures {
            let Ok(mut candidate) = self
                .root
                .freeze_item_mint(self.character, self.holder, request)
                .await
            else {
                continue;
            };
            let _ = self
                .root
                .commit_item_mint(permit, self.character, self.holder, &mut candidate)
                .await;
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::content::creature_policy::{
        CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureExactRatio, CreatureFlags,
    };
    use crate::map::overlay::TilePos;
    use crate::map::view::MapFacts;

    const RAT: &str = "oteryn:creature.rat";
    const EARTH_IMMUNE: &str = "oteryn:creature.earth_immune";

    fn policy(key: &str, name: &str, health: i64) -> CompiledCreaturePolicy {
        CompiledCreaturePolicy {
            definition_key: key.to_owned(),
            definition_revision: "definition-r1".to_owned(),
            display_name: name.to_owned(),
            maximum_health: health,
            forge_difficulty: None,
            base_speed: 100,
            outfit_look_type: 27,
            object_look_type: None,
            summonable: false,
            convinceable: false,
            mana_cost: None,
            is_familiar: false,
            condition_immunities: Vec::new(),
            defense: None,
            armor: None,
            mitigation: None,
            resistances: Vec::new(),
            damage_immunities: Vec::new(),
            healing_from_damage: Vec::new(),
            flags: CreatureFlags {
                attackable: true,
                illusionable: true,
                health_hidden: false,
            },
            preferred_distance: None,
            reward_boss: None,
            blocked_by_nearby_players: false,
        }
    }

    const AT: MovementLocalPosition = MovementLocalPosition {
        x: 6,
        y: 0,
        floor: 7,
    };

    /// One bundle row whose tile at x 6 holds the item `id`, and a Channel with one wild creature
    /// `creature` standing on it.
    fn hook_case(
        id: u16,
        protection_zone: bool,
        creature: (&str, &str, i64),
        check: impl FnOnce(&mut ChannelRuntimeV1, &BundleMap, ExactActorRef),
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use crate::gameplay_transport::actor_spell::tests::runtime_with_player;
        use crate::map::boot::{self, BundleCollisionIndexExt};
        use std::num::NonZeroU32;
        let (mut runtime, actor, _) = runtime_with_player(0x72);
        runtime
            .install_companion_policies(CompiledCreaturePolicies::from_active_artifact(
                runtime.content_pin().server_artifact_digest(),
                vec![
                    policy(STARVING_WOLF, "Starving Wolf", 85),
                    policy(THORNFIRE_WOLF, "Thornfire Wolf", 600),
                    policy(CRYSTAL_WOLF, "Crystal Wolf", 750),
                    CompiledCreaturePolicy {
                        mitigation: Some(CreatureExactRatio {
                            numerator: 0,
                            denominator: 1,
                        }),
                        ..policy(RAT, "Rat", 200)
                    },
                    CompiledCreaturePolicy {
                        mitigation: Some(CreatureExactRatio {
                            numerator: 0,
                            denominator: 1,
                        }),
                        damage_immunities: vec!["earth".to_owned()],
                        ..policy(EARTH_IMMUNE, "Earth Immune", 200)
                    },
                ],
            )?)
            .map_err(|e| format!("{e:?}"))?;
        let wolf = runtime
            .admit_source_pinned_lab_creature(AT, creature.0, creature.2)
            .map_err(|e| format!("{e:?}"))?;
        runtime
            .install_creature_policy(wolf, creature.0)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            runtime
                .companion_snapshot(wolf)
                .map_err(|e| format!("{e:?}"))?
                .state
                .policy
                .display_name,
            creature.1
        );
        // A trap's closed form is in the palette, so its transform resolves.
        let closed = trap(id)
            .map(|rule| rule.closed)
            .filter(|closed| *closed != id);
        let keys = (
            format!("oteryn:item.tibia.i{id}"),
            format!("oteryn:item.tibia.i{}", closed.unwrap_or(100)),
        );
        let (bytes, mut pins) = boot::tests::bundle_with_item_ids((&keys.0, 0), (&keys.1, 1));
        let mut bundle = oteryn_world_bundle::writer::read(&bytes)?;
        for tile in &mut bundle.sectors[0].tiles {
            if tile.x == 6 && protection_zone {
                tile.flags = 1;
            }
        }
        let bytes = oteryn_world_bundle::writer::write(
            &bundle.manifest,
            &bundle.sectors,
            &Default::default(),
        )?;
        pins.digest = oteryn_world_bundle::writer::read(&bytes)?.digest;
        let pos = TilePos {
            x: 6,
            y: 0,
            floor: -7,
        };
        let base = crate::map::load(&bytes, &pins).map_err(|e| format!("{e:?}"))?;
        let booted = boot::boot(
            &bytes,
            &boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: pos,
            },
            actor.world_id(),
            actor.channel_id().ok_or("channel")?,
            |key| {
                Some(crate::map::facts::ItemDefinition {
                    reference: NonZeroU32::new(if key == keys.0 { 1 } else { 2 })?,
                    solid: Some(false),
                    blocks_projectile: false,
                    pickupable: false,
                    door: false,
                })
            },
        )
        .map_err(|e| format!("{e:?}"))?;
        let room = crate::content::qualify_native_entry_room(actor.world_id())
            .map_err(|e| format!("{e:?}"))?;
        let cells = booted
            .movement_cells(room.movement_cells())
            .map_err(|e| format!("{e:?}"))?;
        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(collision) =
            cells.index()
        else {
            return Err("bundle collision".into());
        };
        check(&mut runtime, collision.map(), wolf);
        Ok(())
    }

    fn on_stack(
        case: impl FnOnce() -> Result<(), Box<dyn std::error::Error + Send + Sync>> + Send + 'static,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(case)?
            .join()
            .map_err(|_| "creature step hook test thread")?
    }

    #[test]
    fn m10_starving_wolf_on_the_magic_wolf_trap_is_removed_and_a_captured_wolf_minted()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        on_stack(|| {
            let wolf = (STARVING_WOLF, "Starving Wolf", 85);
            hook_case(MAGIC_WOLF_TRAP, false, wolf, |runtime, map, actor| {
                let hooks = apply_creature_step_hooks(
                    runtime,
                    &ChannelSpellStates::default(),
                    map,
                    vec![(actor, AT)],
                    true,
                    1,
                );
                assert_eq!(hooks.removed, vec![actor]);
                assert!(!runtime.contains_live_creature(actor));
                assert_eq!(hooks.effects, vec![(AT, MAGIC_EFFECT_STUN)]);
                assert_eq!(hooks.captures.len(), 1);
                let capture = hooks.captures.first().expect("one capture");
                assert_eq!(capture.item.production_key, CAPTURED_WOLF_KEY);
                assert_eq!(capture.quantity, 1);
                assert_eq!(
                    capture.ground.spatial_position,
                    [0, 0, 0, 6, 0, 0, 0, 0, 0, 7]
                );
                let cause = format!("{:?}", capture.cause);
                assert!(cause.contains(MAGIC_WOLF_TRAP_KEY) && cause.contains(CAPTURE_PURPOSE));
                assert!(cause.contains(&format!(
                    "actor_local_id: {}, actor_local_generation: {}",
                    actor.0.actor_local_id.0, actor.0.actor_local_generation.0
                )));
            })?;
            // trap.lua:23: no capture inside a protection zone; without an admitted captured
            // wolf item the wolf stays, so nothing is lost.
            for (pz, admitted) in [(true, true), (false, false)] {
                hook_case(MAGIC_WOLF_TRAP, pz, wolf, |runtime, map, actor| {
                    let hooks = apply_creature_step_hooks(
                        runtime,
                        &ChannelSpellStates::default(),
                        map,
                        vec![(actor, AT)],
                        admitted,
                        1,
                    );
                    assert!(hooks.removed.is_empty() && hooks.captures.is_empty());
                    assert!(hooks.effects.is_empty());
                    assert!(runtime.contains_live_creature(actor));
                })?;
            }
            // Another creature on the trap, and the wolf on another item, are unchanged.
            hook_case(
                MAGIC_WOLF_TRAP,
                false,
                (THORNFIRE_WOLF, "Thornfire Wolf", 600),
                |runtime, map, actor| {
                    let hooks = apply_creature_step_hooks(
                        runtime,
                        &ChannelSpellStates::default(),
                        map,
                        vec![(actor, AT)],
                        true,
                        1,
                    );
                    assert!(hooks.removed.is_empty() && runtime.contains_live_creature(actor));
                },
            )?;
            hook_case(THORNFIRE_GROUND, false, wolf, |runtime, map, actor| {
                let hooks = apply_creature_step_hooks(
                    runtime,
                    &ChannelSpellStates::default(),
                    map,
                    vec![(actor, AT)],
                    true,
                    1,
                );
                assert!(hooks.removed.is_empty() && runtime.contains_live_creature(actor));
            })
        })
    }

    /// The production native content of the accepted entry room, as the owner activates it.
    fn production_content()
    -> Result<crate::content::native_gameplay::NativeGameplayState, Box<dyn std::error::Error>>
    {
        use crate::content::{
            ContentActivationController, NativeEntryActivationIssuance, NodeBootQuiescence,
            activate_native_entry_room_with_gameplay,
        };
        let mut input = crate::content::native_gameplay::NativeGameplayInput::from_manifest(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../content/spells.manifest.json"),
        )?;
        input.source_world = None;
        let world = crate::foundation::WorldId::decode(&[
            1, 0x90, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1,
        ])?;
        let room = crate::content::qualify_selected_native_gameplay_room(world, &input)?;
        let issuance = NativeEntryActivationIssuance {
            world_id: world,
            activation_sequence: 1,
            server_artifact_digest: room.compiled().server_digest(),
            client_artifact_digest: room.compiled().client_digest(),
            frame_binding_digest: room.frame_binding().digest(),
        };
        let mut controller = ContentActivationController::new();
        activate_native_entry_room_with_gameplay(
            &mut controller,
            &NodeBootQuiescence::before_channel_runtime(),
            world,
            &issuance,
            &input,
        )
        .map_err(|e| format!("{e:?}"))?;
        Ok(controller
            .active()
            .and_then(|active| active.native_gameplay())
            .ok_or("native state")?
            .clone())
    }

    #[test]
    fn m10_production_content_admits_the_captured_wolf_so_the_trap_captures()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        on_stack(|| {
            let admitted = capture_admitted(&production_content().map_err(|e| e.to_string())?);
            assert!(
                admitted,
                "production content must admit 12369 on the ground"
            );
            let wolf = (STARVING_WOLF, "Starving Wolf", 85);
            hook_case(MAGIC_WOLF_TRAP, false, wolf, |runtime, map, actor| {
                let hooks = apply_creature_step_hooks(
                    runtime,
                    &ChannelSpellStates::default(),
                    map,
                    vec![(actor, AT)],
                    admitted,
                    1,
                );
                assert_eq!(hooks.removed, vec![actor]);
                let capture = hooks.captures.first().expect("one capture");
                assert_eq!(capture.item.production_key, CAPTURED_WOLF_KEY);
            })
        })
    }

    #[test]
    fn m10_thornfire_wolf_on_12721_becomes_a_crystal_wolf_on_its_cell()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        on_stack(|| {
            hook_case(
                THORNFIRE_GROUND,
                false,
                (THORNFIRE_WOLF, "Thornfire Wolf", 600),
                |runtime, map, actor| {
                    let hooks = apply_creature_step_hooks(
                        runtime,
                        &ChannelSpellStates::default(),
                        map,
                        vec![(actor, AT)],
                        false,
                        1,
                    );
                    assert_eq!(hooks.removed, vec![actor]);
                    assert!(hooks.captures.is_empty());
                    assert_eq!(hooks.effects, vec![(AT, MAGIC_EFFECT_ICEATTACK)]);
                    assert!(!runtime.contains_live_creature(actor));
                    let crystal = runtime
                        .visible_entities()
                        .creatures
                        .into_iter()
                        .find_map(|visible| runtime.companion_snapshot(visible.actor).ok())
                        .expect("replacement");
                    assert_eq!(crystal.state.policy.definition_key, CRYSTAL_WOLF);
                    assert_eq!(crystal.position, AT);
                },
            )
        })
    }

    /// Crystal trap.lua:23-44 on a creature: a spike trap outside a protection zone closes and
    /// hits once in its range (2145: 50..=100 physical, 3944: 15..=30 earth); an earth-immune
    /// creature takes nothing from 3944, which still closes; inside a protection zone nothing
    /// happens.
    #[test]
    fn m10_a_creature_on_a_spike_trap_closes_it_and_takes_its_hit()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        on_stack(|| {
            let pos = TilePos {
                x: 6,
                y: 0,
                floor: -7,
            };
            let rat = (RAT, "Rat", 200);
            let step = move |runtime: &mut ChannelRuntimeV1, map: &BundleMap, actor| {
                let states = ChannelSpellStates::default();
                let hooks =
                    apply_creature_step_hooks(runtime, &states, map, vec![(actor, AT)], true, 1);
                assert!(hooks.removed.is_empty() && hooks.effects.is_empty());
                let health = runtime.companion_snapshot(actor).expect("creature").health;
                (
                    health,
                    map.facts.base_entry(pos, 1, 0).map(|e| e.appearance_id),
                )
            };
            hook_case(2145, false, rat, |runtime, map, actor| {
                let (health, item) = step(runtime, map, actor);
                assert!((100..=150).contains(&health), "{health}");
                assert_eq!(item, Some(2146));
            })?;
            hook_case(2145, true, rat, |runtime, map, actor| {
                assert_eq!(step(runtime, map, actor), (200, Some(2145)));
            })?;
            hook_case(
                3944,
                false,
                (EARTH_IMMUNE, "Earth Immune", 200),
                |runtime, map, actor| {
                    assert_eq!(step(runtime, map, actor), (200, Some(3945)));
                },
            )?;
            hook_case(3944, false, rat, |runtime, map, actor| {
                let (health, item) = step(runtime, map, actor);
                assert!((170..=185).contains(&health), "{health}");
                assert_eq!(item, Some(3945));
            })
        })
    }
}
