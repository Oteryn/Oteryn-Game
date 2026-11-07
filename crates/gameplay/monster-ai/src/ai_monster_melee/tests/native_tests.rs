//! Owner integration fixtures: source closure -> shared think -> real bite commit/replay.
//! Cadence/magnitude mutations are explicit fixture inputs, not production parity claims.
use super::*;
use crate::ai_think::behaviour_profile::CreatureBehaviourProfile;
use crate::ai_think::targeting::TargetCandidate;
use crate::ai_think::{CreatureAiState, CreatureThinkFacts};
use crate::content::{ProjectV2AuthoringProfileData as Data, ProjectV2FormulaAuthoring};

mod movement_tests;

struct NativeFixture {
    runtime: ChannelRuntimeV1,
    owner: MonsterMeleeOwner,
    issuer: ExactActorRef,
    target: ExactActorRef,
    session: GameSessionId,
    sources: Vec<MeleeSource>,
    definitions: Vec<MeleeDefinition>,
    vitals: Vitals,
}

impl NativeFixture {
    fn new(interval_ms: u64, count: usize, magnitude: u32) -> Result<Self, String> {
        Self::with_position_context(interval_ms, count, magnitude, true, true)
    }

    fn with_position_context(
        interval_ms: u64,
        count: usize,
        magnitude: u32,
        issuer_pinned: bool,
        target_pinned: bool,
    ) -> Result<Self, String> {
        let world = WorldId::decode(&uuid(1)).map_err(|error| format!("{error:?}"))?;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&uuid(2)).map_err(|error| format!("{error:?}"))?,
            NodeId::decode(&uuid(3)).map_err(|error| format!("{error:?}"))?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            ChannelContentPin::test(world),
        )
        .map_err(|error| format!("{error:?}"))?;
        let session = GameSessionId::decode(&uuid(4)).map_err(|error| format!("{error:?}"))?;
        let reservation = runtime
            .reserve_fresh_session(session)
            .map_err(|error| format!("{error:?}"))?;
        let target = runtime
            .commit_fresh_session(reservation)
            .map_err(|error| format!("{error:?}"))?;
        if target_pinned {
            runtime.initialize_pinned_test_position(target, position(1))
        } else {
            runtime.initialize_movement_test_position(target, position(1))
        }
        .map_err(|error| format!("{error:?}"))?;
        let document: crate::content::native_gameplay::CreatureProfilesDocument =
            serde_json::from_slice(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/creatures/definitions/spell-native-profiles.json"
            )))
            .map_err(|error| error.to_string())?;
        let row = document
            .records
            .into_iter()
            .find(|row| row.profile.target.key == "oteryn:creature.rat")
            .ok_or("native rat row")?;
        let creature = row.profile.target;
        let mut closure = row.attack_definitions.ok_or("native rat source closure")?;
        let mut behavior = None;
        for profile in &mut closure.profiles {
            match &mut profile.data {
                Data::Behavior(value) => {
                    let mut entry = value.attacks.first().cloned().ok_or("rat melee")?;
                    entry.interval_ms = interval_ms;
                    entry.chance_ppm = 1_000_000;
                    value.attacks = vec![entry; count];
                    behavior = Some(value.clone());
                }
                Data::Formula(value) => {
                    *value = ProjectV2FormulaAuthoring::Range {
                        minimum: u64::from(magnitude),
                        maximum: u64::from(magnitude),
                    }
                }
                _ => {}
            }
        }
        let behavior = behavior.ok_or("source behavior")?;
        let abilities = closure
            .profiles
            .iter()
            .filter_map(|profile| {
                if let Data::Ability(ability) = &profile.data {
                    Some((profile.target.clone(), ability.clone()))
                } else {
                    None
                }
            })
            .collect();
        let profile = CreatureBehaviourProfile::project_native(Some(&behavior), &abilities)
            .map_err(|error| format!("{error:?}"))?;
        let issuer = if issuer_pinned {
            runtime.admit_pinned_monster_lab_creature(position(0), &creature.key, 20)
        } else {
            runtime.admit_monster_lab_creature(position(0), &creature.key, 20)
        }
        .map_err(|error| format!("{error:?}"))?;
        for (actor, pinned) in [(issuer, issuer_pinned), (target, target_pinned)] {
            let context = runtime
                .read_actor_position(actor)
                .map_err(|error| format!("{error:?}"))?
                .context();
            assert_eq!(context == runtime.pinned_movement_context(), pinned);
        }
        let mut owner = MonsterMeleeOwner::default();
        owner
            .creature_ai
            .admit(CreatureAiState::new(issuer, profile, position(0)), 0);
        let mut sources = Vec::new();
        let mut definitions = Vec::new();
        for index in 0..count {
            sources.push(
                MeleeSource::from_native(
                    &creature,
                    index,
                    &closure.records,
                    &closure.profiles,
                    runtime.content_pin().server_artifact_digest(),
                )
                .map_err(|error| format!("{error:?}"))?,
            );
            definitions.push(
                MeleeDefinition::new_native(
                    runtime.content_pin().server_artifact_digest(),
                    interval_ms,
                    1_000_000,
                    magnitude,
                    magnitude,
                )
                .ok_or("source definition")?
                .with_entry_index(u16::try_from(index).map_err(|error| error.to_string())?),
            );
        }
        Ok(Self {
            runtime,
            owner,
            issuer,
            target,
            session,
            sources,
            definitions,
            vitals: Vitals {
                health: 100,
                revision: 0,
                armor: 0,
                defence: None,
                protection: None,
            },
        })
    }

    fn think(&mut self, now_us: u64, visible: bool) -> Result<(), String> {
        let issuer_position = self
            .runtime
            .read_actor_position(self.issuer)
            .map_err(|error| format!("{error:?}"))?
            .position();
        let target_position = self
            .runtime
            .read_actor_position(self.target)
            .map_err(|error| format!("{error:?}"))?
            .position();
        let players = if visible {
            vec![TargetCandidate {
                actor: self.target,
                position: target_position,
                eligible: true,
                invisible: false,
                health: 100,
                damage: 0,
            }]
        } else {
            Vec::new()
        };
        let root = GameplayDecisionRoot::from_bytes([1; 32]);
        let revisions = revisions()?;
        let report = self
            .owner
            .creature_ai
            .think(
                self.issuer,
                &CreatureThinkFacts {
                    position: issuer_position,
                    health: 20,
                    has_active_condition: false,
                    players: &players,
                    now_us,
                    root: &root,
                    revisions: &revisions,
                },
            )
            .ok_or("admitted creature")?
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(report.refusal, None);
        self.owner
            .creature_ai
            .record_native_attack_positions(self.issuer, &self.runtime);
        Ok(())
    }

    fn swing(&mut self, index: usize, pass_sequence: u64, now_us: u64) -> Result<Dispatch, String> {
        let target = self.target;
        let session = self.session;
        let mut current_input = input(1);
        current_input.position = self
            .runtime
            .read_actor_position(self.issuer)
            .map_err(|error| format!("{error:?}"))?
            .position();
        let mut current_perceived = perceived();
        current_perceived[0].position = self
            .runtime
            .read_actor_position(self.target)
            .map_err(|error| format!("{error:?}"))?
            .position();
        self.owner
            .think_with_native_source(
                &mut self.runtime,
                &mut self.vitals,
                self.issuer,
                pass_sequence,
                self.definitions[index],
                current_input,
                &current_perceived,
                |_| {
                    Some((
                        target,
                        session,
                        ReentryProtection {
                            protected_until: None,
                        },
                    ))
                },
                revisions()?,
                SemanticTimeMicros::from_micros(now_us),
                Some(&self.sources[index]),
            )
            .map_err(|error| format!("{error:?}"))
    }
}

#[test]
fn native_short_interval_uses_the_shared_think_without_a_second_1500ms_cooldown()
-> Result<(), String> {
    let mut fixture = NativeFixture::new(500, 1, 8)?;
    for (index, now_us) in [0, 1_000_000, 2_000_000].into_iter().enumerate() {
        fixture.think(now_us, true)?;
        let result = fixture.swing(0, 90 + index as u64, now_us)?;
        assert!(matches!(result, Dispatch::Bite(Ok(_))));
        assert_eq!(fixture.vitals.health, 100 - 8 * (index as u32 + 1));
        assert_eq!(
            fixture.swing(0, 9999, now_us + 1)?,
            result,
            "global pass sequence cannot create another hit without a new committed AI think"
        );
        assert_eq!(fixture.vitals.health, 100 - 8 * (index as u32 + 1));
    }
    Ok(())
}

#[test]
fn disappearance_extra_reaches_vitals_before_the_previous_authored_interval_expires()
-> Result<(), String> {
    let mut fixture = NativeFixture::new(4000, 1, 8)?;
    for now_us in [0, 1_000_000, 2_000_000, 3_000_000] {
        fixture.think(now_us, true)?;
        fixture.swing(0, 100 + now_us, now_us)?;
    }
    assert_eq!(fixture.vitals.health, 92);
    fixture.think(4_000_000, false)?;
    fixture.think(5_000_000, true)?;
    assert!(matches!(
        fixture.swing(0, 200, 5_000_000)?,
        Dispatch::Bite(Ok(_))
    ));
    assert_eq!(
        fixture.vitals.health, 84,
        "extra bypasses the previous bite's 7s deadline"
    );
    fixture.think(6_000_000, true)?;
    assert_eq!(
        fixture.swing(0, 201, 6_000_000)?,
        Dispatch::Decision(ThinkOutcome::Idle)
    );
    assert_eq!(
        fixture.vitals.health, 84,
        "the 1500ms extra guard remains in the shared scheduler"
    );
    Ok(())
}

#[test]
fn native_melee_entries_have_independent_receipts_in_one_committed_think() -> Result<(), String> {
    let mut fixture = NativeFixture::new(1000, 2, 8)?;
    fixture.think(0, true)?;
    let first = fixture.swing(0, 70, 0)?;
    let second = fixture.swing(1, 70, 0)?;
    assert!(matches!(first, Dispatch::Bite(Ok(_))));
    assert!(matches!(second, Dispatch::Bite(Ok(_))));
    assert_eq!((fixture.vitals.health, fixture.vitals.revision), (84, 2));
    assert_eq!(fixture.swing(0, 71, 1)?, first);
    assert_eq!(fixture.swing(1, 71, 1)?, second);
    assert_eq!((fixture.vitals.health, fixture.vitals.revision), (84, 2));
    fixture.think(1_000_000, true)?;
    fixture.swing(0, 72, 1_000_000)?;
    fixture.swing(1, 72, 1_000_000)?;
    assert_eq!((fixture.vitals.health, fixture.vitals.revision), (68, 4));
    assert_eq!(
        fixture.owner.entries.len(),
        1,
        "the existing actor bound is unchanged"
    );
    assert_eq!(fixture.owner.bites.len(), 1);
    Ok(())
}

#[test]
fn native_zero_melee_is_retained_without_damage_or_a_second_replay() -> Result<(), String> {
    let mut fixture = NativeFixture::new(1000, 2, 0)?;
    fixture.think(0, true)?;
    assert_eq!(fixture.swing(0, 80, 0)?, Dispatch::ZeroDamage);
    assert_eq!(fixture.swing(1, 80, 0)?, Dispatch::ZeroDamage);
    assert_eq!(fixture.swing(0, 81, 1)?, Dispatch::ZeroDamage);
    assert_eq!((fixture.vitals.health, fixture.vitals.revision), (100, 0));
    Ok(())
}

#[test]
fn native_source_proof_refuses_modified_public_source_fields_and_definition() -> Result<(), String>
{
    let mut fixture = NativeFixture::new(1000, 1, 8)?;
    fixture.think(0, true)?;
    fixture.sources[0].maximum = 99;
    fixture.definitions[0].maximum = 99;
    assert_eq!(
        fixture.swing(0, 1, 0),
        Err(format!("{:?}", BiteRejection::InvalidPlan))
    );
    assert!(fixture.owner.entries.is_empty());
    assert_eq!(fixture.vitals.health, 100);
    Ok(())
}

#[test]
fn scheduled_bite_definition_is_bound_to_its_exact_occurrence_and_target_session()
-> Result<(), String> {
    let mut fixture = NativeFixture::new(4000, 1, 8)?;
    let intent = AiAbilityAdapter::bite(fixture.issuer, 1, fixture.target, fixture.session, 0, 0);
    let definition = CreatureBiteDefinition::from_native_source(
        &fixture.sources[0],
        &fixture.runtime,
        intent,
        fixture.definitions[0],
        8,
    )
    .ok_or("qualified native bite")?;
    let conflict = AiAbilityAdapter::bite(fixture.issuer, 2, fixture.target, fixture.session, 0, 0);
    let mut ledger = CreatureBiteLedger::default();
    assert_eq!(
        commit_ai_bite(
            &mut ledger,
            &mut fixture.runtime,
            &mut fixture.vitals,
            conflict,
            definition,
            revisions()?,
            ReentryProtection {
                protected_until: None
            },
            SemanticTimeMicros::from_micros(0)
        ),
        Err(BiteRejection::InvalidPlan)
    );
    assert_eq!(ledger.len(), 0, "invalid binding allocates no receipt");
    assert_eq!(
        commit_ai_bite(
            &mut ledger,
            &mut fixture.runtime,
            &mut fixture.vitals,
            intent,
            definition,
            revisions()?,
            ReentryProtection {
                protected_until: Some(SemanticTimeMicros::from_micros(1))
            },
            SemanticTimeMicros::from_micros(0)
        ),
        Err(BiteRejection::TargetProtected)
    );
    assert_eq!(
        commit_ai_bite(
            &mut ledger,
            &mut fixture.runtime,
            &mut fixture.vitals,
            intent,
            definition,
            revisions()?,
            ReentryProtection {
                protected_until: None
            },
            SemanticTimeMicros::from_micros(2)
        ),
        Err(BiteRejection::TargetProtected),
        "a protected attempt cannot become damage on retry"
    );
    assert_eq!(fixture.vitals.health, 100);
    Ok(())
}

#[test]
#[ignore = "requires actual OTERYN_FULL_SPELL_TEST_MANIFEST from the explicit source producer"]
fn actual_full_admitted_melee_sources_resolve_every_authored_index() -> Result<(), String> {
    use crate::content::ProjectV2AbilityKind;
    use crate::content::native_gameplay::NativeGameplayInput;
    let path = std::env::var_os("OTERYN_FULL_SPELL_TEST_MANIFEST")
        .ok_or("explicit actual source artifact manifest")?;
    let input = NativeGameplayInput::from_manifest(std::path::Path::new(&path))
        .map_err(|error| format!("{error:?}"))?;
    let world =
        crate::content::native_gameplay_test_world_id().map_err(|error| format!("{error:?}"))?;
    let room = crate::content::qualify_selected_native_gameplay_room(world, &input)
        .map_err(|error| format!("{error:?}"))?;
    let native = crate::content::qualified_native_gameplay_test_state(room.compiled())
        .map_err(|error| format!("{error:?}"))?;
    let mut source_indices = BTreeMap::new();
    let mut total = 0;
    let mut zero = 0;
    for record in &native.creature_profiles().records {
        // The seven retained legacy/familiar rows need no invented native closure. The
        // exact native source census below refuses a missing or silently dropped entry.
        let Some(closure) = &record.attack_definitions else {
            continue;
        };
        let Some(Data::Behavior(behavior)) = record.behavior.as_ref().map(|profile| &profile.data)
        else {
            return Err(format!(
                "missing native behavior: {}",
                record.profile.target.key
            ));
        };
        let mut indices = Vec::new();
        for (index, entry) in behavior.attacks.iter().enumerate() {
            let ability = closure
                .profiles
                .iter()
                .chain(record.source_item_abilities.iter())
                .find(|profile| profile.target == entry.ability)
                .and_then(|profile| match &profile.data {
                    Data::Ability(value) => value.details.as_deref(),
                    _ => None,
                });
            if !ability.is_some_and(|details| details.kind == ProjectV2AbilityKind::Melee) {
                continue;
            }
            let source = MeleeSource::from_native(
                &record.profile.target,
                index,
                &closure.records,
                &closure.profiles,
                native.source_digest(),
            )
            .map_err(|error| {
                format!(
                    "{} melee[{index}] refused: {error:?}",
                    record.profile.target.key
                )
            })?;
            assert_eq!(source.entry_index, index);
            assert_eq!(source.ability, entry.ability);
            assert!(
                MeleeDefinition::new_native(
                    native.source_digest(),
                    entry.interval_ms,
                    entry.chance_ppm,
                    source.minimum,
                    source.maximum
                )
                .is_some()
            );
            zero += usize::from(source.maximum == 0);
            total += 1;
            indices.push(index);
        }
        if !indices.is_empty() {
            assert!(
                source_indices
                    .insert(record.profile.target.key.clone(), indices)
                    .is_none()
            );
        }
    }
    let multiple = source_indices
        .values()
        .filter(|indices| indices.len() > 1)
        .count();
    assert_eq!(
        total, 1689,
        "every admitted native melee index must resolve; no filter-on-error"
    );
    assert_eq!(
        multiple, 24,
        "multi-melee profiles cannot be reduced to their first slot"
    );
    assert!(
        zero > 0,
        "actual zero-damage source melee must remain admitted"
    );
    eprintln!(
        "qualified full native melee census: entries={total}, multi_entry_creatures={multiple}, zero_damage_entries={zero}"
    );
    Ok(())
}
