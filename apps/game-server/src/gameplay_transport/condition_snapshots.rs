//! Read-only current condition projection on the same independently fenced source transaction.
//! It carries native coordinates and qualified generation pins, never invented VIS2 coordinates.
//! Numeric capability/domain admission and the client renderer remain separate registry owners.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]
use super::actor_spell::ChannelSpellStates;
use crate::ability::condition::{ConditionStore, TemporaryDisplayedAppearance};
use crate::content::QualifiedNativeEntryRoom;
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::spell_item_transaction::{SpellItemAuthority, check_transaction};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementPositionSnapshot, RuntimeScopeRefV1,
};
use crate::spell::cast::PlayerSpellState;
use crate::spell::owned_cast_facts::{CastFactsBinding, OwnedCastFacts};
use crate::world_runtime::LocalObjectRuntime;
use oteryn_protocol_oteryn::actor_condition_snapshot_candidate::{
    self as codec, Snapshot, TemporaryAppearance,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use sqlx::{Postgres, Transaction};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Error {
    Unqualified,
    Stale,
    Codec,
}
#[derive(Debug, Clone)]
pub(crate) struct PreparedConditionSnapshot {
    snapshot: Snapshot,
    bytes: Vec<u8>,
    state: PlayerSpellState,
    position: MovementPositionSnapshot,
    facts: CastFactsBinding,
    equipment: crate::durability::character_equipment::EquipmentSnapshot,
}
impl PreparedConditionSnapshot {
    pub(crate) fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// The publisher supplies another independently read owner projection while
    /// retaining runtime -> player locks. Historical payload bytes grant no authority.
    pub(crate) fn validate_current(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
        current: &OwnedCastFacts,
        now_us: u64,
    ) -> Result<(), Error> {
        let pin = runtime.content_pin();
        let b = current.binding();
        if b != &self.facts
            || current.equipment() != &self.equipment
            || states.get(runtime, b.actor, b.session) != Some(&self.state)
            || runtime
                .read_actor_position(b.actor)
                .map_err(|_| Error::Stale)?
                != self.position
            || pin.server_artifact_digest() != self.snapshot.content
            || pin.map_revision_digest() != self.snapshot.map
            || pin.frame_binding_digest() != self.snapshot.frame
        {
            return Err(Error::Stale);
        }
        let (invisible, light, appearance) = current_fields(self.state.owned_conditions(), now_us)?;
        if now_us < self.snapshot.owner_time_us
            || invisible != self.snapshot.invisible
            || light != self.snapshot.condition_light
            || appearance != self.snapshot.temporary_appearance
        {
            return Err(Error::Stale);
        }
        Ok(())
    }
}
fn current_fields<S: Clone>(
    store: &ConditionStore<S>,
    now: u64,
) -> Result<(bool, Option<codec::Light>, Option<TemporaryAppearance>), Error> {
    if !store.accepts_time(now) {
        return Err(Error::Stale);
    }
    let light = store
        .light_at(now)
        .map(|(level, color)| {
            color
                .map(|color| codec::Light { level, color })
                .ok_or(Error::Unqualified)
        })
        .transpose()?;
    let appearance = store
        .displayed_temporary_appearance_at(now)
        .map(|shown| match shown {
            TemporaryDisplayedAppearance::Item(item) => TemporaryAppearance::Item {
                key: item.definition_key().to_owned(),
                revision: item.revision_ref().to_owned(),
                artifact: item.artifact_digest(),
            },
            TemporaryDisplayedAppearance::Outfit(outfit) => TemporaryAppearance::Outfit {
                key: outfit.outfit_key.clone(),
                colours: outfit.colours,
                addons: outfit.addons,
                mount: outfit.mount_key.clone(),
            },
        });
    Ok((store.invisible_at(now), light, appearance))
}
#[allow(clippy::too_many_arguments)]
pub(crate) async fn prepare_current_player_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    objects: &LocalObjectRuntime,
    content: &NativeGameplayState,
    owned: &OwnedCastFacts,
    actor: ExactActorRef,
    session: GameSessionId,
    now_us: u64,
) -> Result<PreparedConditionSnapshot, Error> {
    check_transaction(tx, authority)
        .await
        .map_err(|_| Error::Unqualified)?;
    let b = owned.binding();
    let scope = runtime.binding();
    let pin = runtime.content_pin();
    let state = states.get(runtime, actor, session).ok_or(Error::Stale)?;
    if b.actor != actor
        || b.session != session
        || b.player_revision != state.revision()
        || authority.command().game_session_id() != session
        || authority.character_id_bytes() != b.character
        || authority.runtime_scope()
            != RuntimeScopeRefV1::channel(scope.world_id(), scope.channel_id())
        || authority.scope_generation() != scope.scope_generation().get()
        || content.source_digest() != b.content_digest
        || authority.compatible_content_digest() != b.content_digest
        || pin.server_artifact_digest() != b.content_digest
    {
        return Err(Error::Unqualified);
    }
    let position = runtime
        .read_actor_position(actor)
        .map_err(|_| Error::Stale)?;
    // This is the actual signed source cell/frame qualification used by combat,
    // including mutable Item placements read in this same physical SQL transaction.
    let tile = crate::spell::world_execution::qualified_combat_tile_in_transaction(
        tx,
        authority,
        room,
        runtime,
        objects,
        position.position(),
    )
    .await
    .map_err(|_| Error::Unqualified)?;
    if !tile.ground_present() {
        return Err(Error::Unqualified);
    }
    let (invisible, condition_light, temporary_appearance) =
        current_fields(state.owned_conditions(), now_us)?;
    match &temporary_appearance {
        Some(TemporaryAppearance::Item {
            key,
            revision,
            artifact,
        }) => {
            if *artifact != content.source_digest()
                || (content.item_policy(key, revision).is_none()
                    && content
                        .spell_appearances()
                        .and_then(|r| r.for_item(key, revision))
                        .is_none())
            {
                return Err(Error::Unqualified);
            }
        }
        Some(TemporaryAppearance::Outfit {
            key,
            colours,
            addons,
            mount,
        }) => {
            let look = state
                .owned_conditions()
                .outfit_at(now_us)
                .ok_or(Error::Unqualified)?;
            let qualified = content
                .spell_appearances()
                .and_then(|p| {
                    p.for_selection(
                        look,
                        &crate::domain::appearance::AppearanceSelection {
                            outfit_key: key.clone(),
                            colours: *colours,
                            addons: *addons,
                            mount_key: mount.clone(),
                        },
                    )
                })
                .ok_or(Error::Unqualified)?;
            let selected = qualified.selection();
            if qualified.source_digest() != content.source_digest()
                || selected.outfit_key != *key
                || selected.colours != *colours
                || selected.addons != *addons
                || selected.mount_key != *mount
            {
                return Err(Error::Unqualified);
            }
        }
        None => {}
    }
    let p = position.position();
    let snapshot = Snapshot {
        entity: EntityRef {
            identity: actor.placement_identity(),
            generation: actor.actor_local_generation(),
        },
        content: pin.server_artifact_digest(),
        map: pin.map_revision_digest(),
        frame: pin.frame_binding_digest(),
        x: p.x,
        y: p.y,
        floor: p.floor,
        owner_time_us: now_us,
        player_revision: state.revision(),
        invisible,
        condition_light,
        temporary_appearance,
    };
    let bytes = codec::encode(&snapshot).map_err(|_| Error::Codec)?;
    Ok(PreparedConditionSnapshot {
        snapshot,
        bytes,
        state: state.clone(),
        position,
        facts: b.clone(),
        equipment: owned.equipment().clone(),
    })
}
#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::ability::condition::{
        ApplicationFacts, ConditionDefinition, ConditionSourceKind, ConditionValues,
    };
    use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
    #[test]
    fn source_current_light_invisible_item_expiry_never_overwrites_persistent_selection() {
        let root = GameplayDecisionRoot::from_bytes([1; 32]);
        let facts = ApplicationFacts {
            now: 1000,
            base_speed: 220,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([2; 16]),
        };
        let mut store = ConditionStore::<String>::new();
        let catalog: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../../tools/content-schema/spell-authoring/samples/executable-spell-catalog.json"
        ))
        .expect("source catalog");
        let row = catalog["bundles"]
            .as_array()
            .expect("bundles")
            .iter()
            .find(|r| r["bundle"]["spell"]["identity"]["key"] == "candidate:spell/light")
            .expect("actual Light");
        let definition =
            crate::spell::authoring::spell_from_bundle(&row["bundle"], &row["dependencies"])
                .expect("source Light definition");
        let source = definition.authored.as_ref().expect("authored source");
        let light = crate::spell::actor_conditions::condition_definition(
            &source.dependencies.effects[0],
            &source.dependencies,
        )
        .expect("qualified source Light converter");
        let invisible = ConditionDefinition::new(
            "source.invisible",
            1,
            ConditionValues::Invisible { duration_ms: 6000 },
        )
        .expect("source invisible");
        let item = ConditionDefinition::new(
            "source.chameleon",
            1,
            ConditionValues::ItemOutfit { duration_ms: 6000 },
        )
        .expect("source item lifetime")
        .with_item_appearance("oteryn:item.tibia.i3264", "source-r1", [3; 32])
        .expect("qualified item");
        for definition in [&light, &invisible, &item] {
            store
                .apply(
                    definition,
                    Some("actual-source".into()),
                    ConditionSourceKind::SelfUse,
                    &[],
                    &facts,
                )
                .expect("real store application");
        }
        let (_, full, shown) = current_fields(&store, 1001).expect("actual current projection");
        assert_eq!(
            full,
            Some(codec::Light {
                level: 6,
                color: 215
            })
        );
        assert!(matches!(
            shown,
            Some(TemporaryAppearance::Item {
                artifact,
                ..
            }) if artifact == [3;32]
        ));
        assert_eq!(
            current_fields(&store, 61_667_000)
                .expect("source first whole-ms decay")
                .1,
            Some(codec::Light {
                level: 5,
                color: 215
            })
        );
        assert_eq!(
            current_fields(&store, 6_001_000).expect("exact temporary expiry"),
            (
                false,
                Some(codec::Light {
                    level: 6,
                    color: 215
                }),
                None
            )
        );
        assert_eq!(
            current_fields(&store, 370_001_000).expect("actual Light exact expiry"),
            (false, None, None)
        );
        assert_eq!(store.instances().len(), 3); // Observation does not mutate or discard owner conditions.
    }
}

#[cfg(test)]
mod source_player_owner_regressions {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::foundation::{
        self as source, ChannelRuntimeV1, ExactActorRef, GameSessionId, RuntimeScopeRefV1,
        RuntimeWorkStamp, ScopeOwnershipGeneration, ScopeRuntimeFence,
    };
    use crate::gameplay_transport::actor_spell::ChannelSpellStates;
    use crate::player_lethal::PlayerLethalVitals;
    use oteryn_simulation_determinism::{
        DecisionOccurrenceId, GameplayDecisionRoot, SemanticTimeMicros,
    };

    fn ready(
        tag: u8,
    ) -> (
        ChannelRuntimeV1,
        ChannelSpellStates,
        ExactActorRef,
        GameSessionId,
        ExactActorRef,
        ScopeRuntimeFence,
        RuntimeWorkStamp,
    ) {
        let (mut runtime, actor, session) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(tag);
        runtime.initialize_first_entry_position(actor).unwrap();
        let p = runtime.read_actor_position(actor).unwrap().position();
        let caster = runtime
            .admit_source_pinned_lab_creature(
                source::MovementLocalPosition {
                    x: p.x + 1,
                    y: p.y,
                    floor: p.floor,
                },
                "test:creature",
                20,
            )
            .unwrap();
        let mut states = ChannelSpellStates::default();
        let mut character = crate::gameplay_transport::actor_spell::tests::FACTS;
        character.magic_level = 10;
        states
            .initialize(
                &runtime,
                actor,
                session,
                character,
                (0, 0),
                SemanticTimeMicros::from_micros(0),
            )
            .unwrap();
        let b = runtime.binding();
        let (fence, stamp) = source::crystal_timer_fixture(
            RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        )
        .unwrap();
        (runtime, states, actor, session, caster, fence, stamp)
    }
    fn facts(root: &GameplayDecisionRoot, id: u8) -> source::ApplicationFacts<'_> {
        source::ApplicationFacts {
            now: 0,
            base_speed: 110,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: root,
            occurrence: DecisionOccurrenceId::from_bytes([id; 16]),
        }
    }
    fn defs() -> Vec<source::ConditionDefinition> {
        use source::ConditionValues as V;
        vec![
            source::ConditionDefinition::new(
                "source.native.invisible",
                1,
                V::TimedStatus {
                    kind: source::StatusKind::Invisible,
                    duration_ms: 1000,
                },
            )
            .unwrap(),
            source::ConditionDefinition::new(
                "source.native.paralysis",
                1,
                V::Speed {
                    paralysis: true,
                    range: source::SpeedRange {
                        a_min: 0,
                        b_min: 10,
                        a_max: 0,
                        b_max: 10,
                    },
                    duration_ms: 1000,
                },
            )
            .unwrap(),
            source::ConditionDefinition::new(
                "source.native.attributes",
                1,
                V::SourceAttributes {
                    modifiers: source::AttributeModifiers {
                        magic_points: Some(source::AttributeModifier::Add(-1)),
                        ..Default::default()
                    },
                    duration_ms: 1000,
                },
            )
            .unwrap(),
            source::ConditionDefinition::new(
                "source.native.poison",
                1,
                V::DamageOverTime {
                    element: source::DotElement::Poison,
                    total_min: 20,
                    total_max: 20,
                    per_tick: 10,
                    interval_ms: 1000,
                    delayed: true,
                },
            )
            .unwrap(),
        ]
    }

    #[test]
    fn source_conditions_feed_actual_snapshot_cure_and_native_player_store_only() {
        let (mut r, mut s, a, g, c, fence, stamp) = ready(0x91);
        let root = GameplayDecisionRoot::from_bytes([41; 32]);
        let definitions = defs();
        let original = s.get(&r, a, g).unwrap().clone();
        assert!(s.apply_attack_conditions(
            &mut r,
            a,
            g,
            "source.native:0",
            c,
            &definitions,
            &facts(&root, 41),
            &[],
            &fence,
            stamp
        ));
        let current = s.get(&r, a, g).unwrap();
        assert_eq!(current.vitals(), original.vitals());
        assert_eq!(current.character_facts(), original.character_facts());
        assert_eq!(current.owned_effective_magic_level(0).unwrap(), 9);
        assert_eq!(current.owned_effective_magic_level(1_000_000).unwrap(), 10);
        assert_eq!(current.owned_conditions().instances().len(), 4);
        // This is the actual snapshot owner's projection, not a parallel test reader.
        assert!(current_fields(current.owned_conditions(), 0).unwrap().0);
        // Accepted native paralysis clamps the target10 to floor40; base110 -> delta-70.
        assert_eq!(current.owned_conditions().active_speed_delta(0), -70);
        assert_eq!(
            110 + current.owned_conditions().active_speed_delta(0),
            crate::ability::condition::PARALYSIS_SPEED_FLOOR
        );
        assert_eq!(
            s.native_effective_attributes(&r, a, g, SemanticTimeMicros::from_micros(0))
                .unwrap()
                .magic_level(10),
            Some(9)
        );
        assert!(
            r.actor_conditions(a, Some(g))
                .unwrap()
                .instances()
                .is_empty()
        );
        assert!(r.actor_conditions(c, None).unwrap().instances().is_empty());
        let committed = current.clone();
        assert!(s.apply_attack_conditions(
            &mut r,
            a,
            g,
            "source.native:0",
            c,
            &definitions,
            &facts(&root, 41),
            &[],
            &fence,
            stamp
        ));
        assert_eq!(s.get(&r, a, g).unwrap(), &committed);
        // Existing native spell cure consumes exactly the same player store.
        assert!(
            crate::spell::actor_conditions::remove_condition(
                s.get_mut(&r, a, g).unwrap(),
                "paralysis"
            )
            .unwrap()
        );
        let cured = s.get(&r, a, g).unwrap();
        assert_eq!(cured.owned_conditions().active_speed_delta(0), 0);
        assert_eq!(cured.owned_conditions().instances().len(), 3);
        assert!(current_fields(cured.owned_conditions(), 0).unwrap().0);
        let cycle = crate::spell::actor_conditions::stage_source_owner_cycle(
            cured,
            SemanticTimeMicros::from_micros(1_000_000),
            Some(crate::ability::condition::TickFacts {
                in_protection_zone: false,
                standing_on_field: None,
            }),
            None,
        )
        .unwrap()
        .unwrap();
        assert_eq!(cycle.combat_ticks.len(), 1);
        assert!(matches!(
            cycle.combat_ticks[0].kind,
            crate::ability::condition::TickKind::Damage {
                amount: 10,
                refused: false,
                ..
            }
        ));
        assert_eq!(cured.owned_conditions().instances().len(), 3); // Projection alone publishes nothing.
    }

    #[test]
    fn source_player_root_status_uses_native_movement_and_stale_grant_preserves_both_owners() {
        let (mut r, mut s, a, g, c, mut fence, stamp) = ready(0x92);
        let root = GameplayDecisionRoot::from_bytes([42; 32]);
        let definitions = [source::ConditionDefinition::new(
            "source.native.root",
            1,
            source::ConditionValues::TimedStatus {
                kind: source::StatusKind::Rooted,
                duration_ms: 1000,
            },
        )
        .unwrap()];
        assert!(s.native_condition_movement_allowed(&r, a, g, SemanticTimeMicros::from_micros(0)));
        assert!(s.apply_attack_conditions(
            &mut r,
            a,
            g,
            "source.root:0",
            c,
            &definitions,
            &facts(&root, 42),
            &[],
            &fence,
            stamp
        ));
        assert!(!s.native_condition_movement_allowed(&r, a, g, SemanticTimeMicros::from_micros(0)));
        assert!(s.native_condition_movement_allowed(
            &r,
            a,
            g,
            SemanticTimeMicros::from_micros(1_000_000)
        ));
        let owned = s.get(&r, a, g).unwrap().clone();
        let slot = r.actor_conditions(a, Some(g)).unwrap().clone();
        fence
            .apply_external_grant(ScopeOwnershipGeneration::new(2).unwrap())
            .unwrap();
        assert!(!s.apply_attack_conditions(
            &mut r,
            a,
            g,
            "source.root:1",
            c,
            &definitions,
            &facts(&root, 43),
            &[],
            &fence,
            stamp
        ));
        assert_eq!(s.get(&r, a, g).unwrap(), &owned);
        assert_eq!(r.actor_conditions(a, Some(g)).unwrap(), &slot);
    }

    #[test]
    fn native_source_lethal_composite_clears_actual_player_store_once() {
        let (mut r, mut s, a, g, c, fence, stamp) = ready(0x93);
        let root = GameplayDecisionRoot::from_bytes([43; 32]);
        let definitions = defs();
        assert!(s.apply_attack_conditions(
            &mut r,
            a,
            g,
            "source.predeath:0",
            c,
            &definitions,
            &facts(&root, 44),
            &[],
            &fence,
            stamp
        ));
        assert_eq!(
            s.get(&r, a, g)
                .unwrap()
                .owned_conditions()
                .instances()
                .len(),
            4
        );
        let receipt = s
            .apply_composite_attack_damage(
                &mut r,
                a,
                g,
                185,
                "source.lethal:0",
                c,
                &definitions,
                &facts(&root, 45),
                &[],
                &fence,
                stamp,
            )
            .unwrap();
        assert_eq!(receipt.health_after, 0);
        assert!(receipt.death.is_some());
        assert!(s.is_dead(a));
        assert!(
            s.get(&r, a, g)
                .unwrap()
                .owned_conditions()
                .instances()
                .is_empty()
        );
        assert!(
            r.actor_conditions(a, Some(g))
                .unwrap()
                .instances()
                .is_empty()
        );
        assert_eq!(
            s.apply_composite_attack_damage(
                &mut r,
                a,
                g,
                185,
                "source.lethal:0",
                c,
                &definitions,
                &facts(&root, 45),
                &[],
                &fence,
                stamp
            ),
            Some(receipt)
        );
        assert!(
            s.get(&r, a, g)
                .unwrap()
                .owned_conditions()
                .instances()
                .is_empty()
        );
    }
}
