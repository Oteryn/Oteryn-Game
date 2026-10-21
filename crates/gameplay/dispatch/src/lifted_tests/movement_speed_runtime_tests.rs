mod player_speed_tests {
    #![allow(clippy::expect_used)]

    use crate::content::LogicalCell;
    use crate::movement::speed::*;
    use oteryn_simulation_determinism::SemanticTimeMicros;
    use std::time::Duration;

    /// A charm's speed condition from the authored rows, as the condition owner tests load it.
    fn charm(name: &str) -> crate::foundation::ConditionDefinition {
        use crate::foundation::{ConditionDefinition, ConditionValues, SpeedRange};
        let rows: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../tools/content-schema/condition-authoring/authored-conditions.json"
        ))
        .expect("authored conditions");
        let row = rows["rows"]
            .as_array()
            .and_then(|rows| {
                rows.iter()
                    .find(|row| row["key"] == format!("charm.{name}"))
            })
            .expect("charm row");
        let values = &row["speed"];
        let n = |key: &str| i32::try_from(values[key].as_i64().expect("i64")).expect("i32");
        ConditionDefinition::new(
            &format!("oteryn:condition.charm.{name}"),
            1,
            ConditionValues::Speed {
                paralysis: values["kind"] == "paralysis",
                range: SpeedRange {
                    a_min: n("a_min"),
                    b_min: n("b_min"),
                    a_max: n("a_max"),
                    b_max: n("b_max"),
                },
                duration_ms: u32::try_from(values["duration_ms"].as_u64().expect("u64"))
                    .expect("u32"),
            },
        )
        .expect("definition")
    }

    /// The step duration of the player after `name` (if any) was applied to them at time 0.
    fn duration_with(name: Option<&str>, tag: u8) -> Option<Duration> {
        use crate::foundation::{
            ActorConditionTransition, ApplicationFacts, ConditionSource, ConditionSourceKind,
        };
        use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
        let (mut runtime, actor, session) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(tag);
        let at = SemanticTimeMicros::from_micros;
        if let Some(name) = name {
            let definition = charm(name);
            let root = GameplayDecisionRoot::from_bytes([7; 32]);
            let plan = runtime
                .prepare_actor_condition(
                    actor,
                    Some(session),
                    ActorConditionTransition::Apply {
                        definition: &definition,
                        source: ConditionSource {
                            actor,
                            session: Some(session),
                            kind: ConditionSourceKind::SelfUse,
                        },
                        immunities: &[],
                        facts: ApplicationFacts {
                            now: 0,
                            base_speed: player_effective_speed(1, 0, 0),
                            mana_shield_capacity: 0,
                            target_reentry_protected: false,
                            source_reentry_protected: false,
                            target_is_player: true,
                            decision_root: &root,
                            occurrence: DecisionOccurrenceId::from_bytes([1; 16]),
                        },
                    },
                    at(0),
                )
                .expect("prepared");
            assert_eq!(runtime.commit_actor_condition(&plan, at(0)), Ok(true));
        }
        let speed = runtime_player_speed(&runtime, actor, session, 1, at(1))?;
        player_step_duration(
            &EngineeringGroundSpeed,
            LogicalCell { x: 0, y: 0, z: 7 },
            speed,
        )
    }

    #[test]
    fn haste_and_paralysis_on_the_runtime_actor_change_the_step_duration() {
        let plain = duration_with(None, 0x81).expect("plain");
        assert_eq!(plain, Duration::from_millis(550));
        let haste = duration_with(Some("adrenaline_burst"), 0x82).expect("haste");
        let paralysis = duration_with(Some("cripple"), 0x83).expect("paralysis");
        assert!(haste < plain, "{haste:?} < {plain:?}");
        assert!(paralysis > plain, "{paralysis:?} > {plain:?}");
    }
}

mod creature_native_speed_tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use crate::content::LogicalCell;
    use crate::foundation::{
        ActorConditionTransition, ApplicationFacts, ChannelRuntimeV1, ConditionDefinition,
        ConditionSource, ConditionSourceKind, ConditionValues, ExactActorRef,
        MovementLocalPosition, SpeedRange,
    };
    use crate::movement::speed::*;
    use oteryn_simulation_determinism::{
        DecisionOccurrenceId, GameplayDecisionRoot, SemanticTimeMicros,
    };
    use std::time::Duration;
    struct Native {
        actor: ExactActorRef,
        base: u16,
        pin: [u8; 32],
    }
    impl CurrentCreatureSpeedReader for Native {
        fn current_native_base_speed(&self, r: &ChannelRuntimeV1, a: ExactActorRef) -> Option<u16> {
            (a == self.actor
                && r.contains_live_creature(a)
                && r.content_pin().server_artifact_digest() == self.pin)
                .then_some(self.base)
        }
    }
    #[test]
    fn creature_cadence_reads_real_slot_haste_expiry_ground_and_stale_generation() {
        let (mut r, _, _) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(0x71);
        let position = MovementLocalPosition {
            x: 100,
            y: 100,
            floor: 7,
        };
        let actor = r.admit_test_creature(position).unwrap();
        let src = Native {
            actor,
            base: 220,
            pin: r.content_pin().server_artifact_digest(),
        };
        let onto = LogicalCell {
            x: 101,
            y: 100,
            z: 7,
        };
        let at = SemanticTimeMicros::from_micros;
        assert_eq!(
            creature_step_duration(&r, actor, &src, &EngineeringGroundSpeed, onto, at(0)),
            Some(Duration::from_millis(300))
        );
        let definition = ConditionDefinition::new(
            "speed.creature.fixture",
            1,
            ConditionValues::Speed {
                paralysis: false,
                range: SpeedRange {
                    a_min: 0,
                    b_min: 330,
                    a_max: 0,
                    b_max: 330,
                },
                duration_ms: 1000,
            },
        )
        .unwrap();
        let root = GameplayDecisionRoot::from_bytes([7; 32]);
        let plan = r
            .prepare_actor_condition(
                actor,
                None,
                ActorConditionTransition::Apply {
                    definition: &definition,
                    source: ConditionSource {
                        actor,
                        session: None,
                        kind: ConditionSourceKind::Creature,
                    },
                    immunities: &[],
                    facts: ApplicationFacts {
                        now: 100,
                        base_speed: 220,
                        mana_shield_capacity: 0,
                        target_reentry_protected: false,
                        source_reentry_protected: false,
                        target_is_player: false,
                        decision_root: &root,
                        occurrence: DecisionOccurrenceId::from_bytes([1; 16]),
                    },
                },
                at(100),
            )
            .unwrap();
        assert!(r.commit_actor_condition(&plan, at(100)).unwrap());
        assert_eq!(runtime_creature_speed(&r, actor, &src, at(99)), None);
        assert_eq!(runtime_creature_speed(&r, actor, &src, at(101)), Some(330));
        assert_eq!(
            creature_step_duration(&r, actor, &src, &EngineeringGroundSpeed, onto, at(101)),
            Some(Duration::from_millis(250))
        );
        assert_eq!(
            creature_step_duration(
                &r,
                actor,
                &src,
                &EngineeringGroundSpeed,
                onto,
                at(1_000_100)
            ),
            Some(Duration::from_millis(300))
        );
        struct Unknown;
        impl GroundSpeedSource for Unknown {
            fn ground_speed(&self, _: LogicalCell) -> u16 {
                0
            }
        }
        assert_eq!(
            creature_step_duration(&r, actor, &src, &Unknown, onto, at(1_000_100)),
            None
        );
        let wrong = Native {
            pin: [0; 32],
            ..src
        };
        assert_eq!(
            runtime_creature_speed(&r, actor, &wrong, at(1_000_100)),
            None
        );
        r.remove_test_actor(actor).unwrap();
        let replacement = r.admit_test_creature(position).unwrap();
        assert_ne!(replacement, actor);
        assert_eq!(runtime_creature_speed(&r, actor, &src, at(1_000_100)), None);
    }
}
