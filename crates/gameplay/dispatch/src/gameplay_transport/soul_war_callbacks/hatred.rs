//! Crystal201189f soul_war_mechanics.lua579-643: actual Burning onThink owner.
use super::*;
use oteryn_gameplay_encounters::bosses::goshnars_hatred::{BurningForm, Effect, HatredRoom};
use oteryn_gameplay_entities::carrier::soul_war_hatred::HatredEmpowerment;

const HATRED: &str = "oteryn:creature.goshnar_s_hatred";
#[derive(Debug)]
pub(super) struct HatredCallbacks {
    boss: ExactActorRef,
    burning: ExactActorRef,
    clock: HatredRoom,
    content: [u8; 32],
    entered: bool,
    removed: bool,
    removing: Option<[Option<ExactActorRef>; 4]>,
}
impl HatredCallbacks {
    fn current_actors(
        runtime: &ChannelRuntimeV1,
        native: &NativeGameplayState,
        boss: ExactActorRef,
        burning: ExactActorRef,
    ) -> Result<Self, ()> {
        runtime.owner_fence().map_err(|_| ())?;
        if !matches!(
            runtime.binding().runtime_scope(),
            crate::foundation::RuntimeScopeRefV1::Instance { .. }
        ) || !qualifies_source(runtime, native)
            || source_actor(runtime, native, boss).as_deref() != Some(HATRED)
            || source_actor(runtime, native, burning).as_deref() != Some(BurningForm::Ashes.key())
            || !runtime.contains_live_creature(boss)
            || !runtime.contains_live_creature(burning)
        {
            return Err(());
        }
        Ok(Self {
            boss,
            burning,
            clock: HatredRoom::default(),
            content: native.source_digest(),
            entered: false,
            removed: false,
            removing: None,
        })
    }
    pub(super) fn initialize(
        runtime: &mut ChannelRuntimeV1,
        native: &NativeGameplayState,
        room: &QualifiedNativeEntryRoom,
        boss: ExactActorRef,
    ) -> Result<Self, ()> {
        use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
        if room.pinned_activity_source_room() != Some(PinnedActivitySourceRoom::GoshnarsHatred)
            || runtime.content_pin().frame_binding_digest() != room.frame_binding().digest()
            || runtime.map_revision_digest() != room.map_revision_digest()
            || !runtime.read_actor_position(boss).is_ok_and(|position| {
                position.position()
                    == MovementLocalPosition {
                        x: 33744,
                        y: 31599,
                        floor: 14,
                    }
            })
        {
            return Err(());
        }
        let at = MovementLocalPosition {
            x: 33743,
            y: 31599,
            floor: 14,
        };
        // boss_lever.lua228-229 creates this literal child with force=true.
        // Read the genuinely qualified current ground, never an injected map grant.
        let burning = runtime
            .spawn_soul_war_child(boss, BurningForm::Ashes.key(), at, |_, position| {
                super::super::soul_war_scene::current_ground(room, position).map(|_| true)
            })
            .map_err(|_| ())?;
        Self::current_actors(runtime, native, boss, burning)
    }
    pub(super) fn committed_entry(&mut self) {
        self.clock.lever_pulled();
        self.entered = true;
    }
    /// A native think carries its original actor and sequence. No wall-clock
    /// catch-up or client interaction advances the source callback counter.
    fn think(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        native: &NativeGameplayState,
        occurrence: ThinkOccurrence,
        mut speech: impl FnMut(ExactActorRef, MovementLocalPosition, &str) -> bool,
    ) -> Result<bool, ()> {
        if occurrence.actor != self.burning
            || self.content != runtime.content_pin().server_artifact_digest()
            || !qualifies_source(runtime, native)
        {
            return Err(());
        }
        runtime.owner_fence().map_err(|_| ())?;
        if self
            .clock
            .last_think()
            .is_some_and(|sequence| occurrence.sequence <= sequence)
        {
            return Ok(false);
        }
        runtime
            .assert_actor_spell_unreserved(self.burning)
            .map_err(|_| ())?;
        let before = runtime.companion_snapshot(self.burning).map_err(|_| ())?;
        if before.health <= 0 {
            return Ok(false);
        }
        let current = [
            BurningForm::Ashes,
            BurningForm::Spark,
            BurningForm::Flame,
            BurningForm::Blaze,
        ]
        .into_iter()
        .find(|form| form.key() == before.state.policy.definition_key)
        .ok_or(())?;
        let boss = runtime
            .companion_snapshot(self.boss)
            .ok()
            .filter(|snapshot| {
                snapshot.health > 0 && snapshot.state.policy.definition_key == HATRED
            });
        let mut next = self.clock.clone();
        let effects = next.burning_think(occurrence.sequence, current, boss.is_some());
        let percent = if effects
            .iter()
            .any(|effect| matches!(effect, Effect::EmpowerBoss(_)))
        {
            runtime
                .assert_actor_spell_unreserved(self.boss)
                .map_err(|_| ())?;
            let value = runtime.soul_war_hatred_percent(self.boss).map_err(|_| ())?;
            value.checked_add(10).ok_or(())?;
            Some(value)
        } else {
            None
        };
        for effect in effects {
            match effect {
                Effect::SetForm {
                    form,
                    restore_health: true,
                } => {
                    runtime
                        .change_soul_war_burning_form(&before, form.key())
                        .map_err(|_| ())?;
                }
                Effect::AshesSpeech => {
                    let position = runtime
                        .read_actor_position(self.burning)
                        .map_err(|_| ())?
                        .position();
                    let _ = speech(
                        self.burning,
                        position,
                        "The fire of hatred fuels and empowers Goshnar's Hate!",
                    );
                }
                Effect::EmpowerBoss(amount) => {
                    runtime
                        .change_soul_war_hatred_percent(
                            boss.as_ref().ok_or(())?,
                            percent.ok_or(())?,
                            HatredEmpowerment::Add(amount),
                        )
                        .map_err(|_| ())?;
                }
                _ => return Err(()),
            }
        }
        self.clock = next;
        Ok(true)
    }
}

impl HatredCallbacks {
    fn remove_source_forms(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        native: &NativeGameplayState,
    ) -> Result<(), ()> {
        runtime.owner_fence().map_err(|_| ())?;
        if self.boss.runtime_scope() != runtime.binding().runtime_scope()
            || self.boss.scope_generation() != runtime.binding().scope_generation()
            || self.content != runtime.content_pin().server_artifact_digest()
            || !qualifies_source(runtime, native)
        {
            return Err(());
        }
        if self.removed {
            return Ok(());
        }
        // Copied death handlers require their occupied slots until source completion.
        // The native owner drains them before this onDisappear callback.
        if runtime.has_pending_source_creature_deaths() {
            return Err(());
        }
        let forms = [
            BurningForm::Ashes,
            BurningForm::Spark,
            BurningForm::Flame,
            BurningForm::Blaze,
        ];
        if self.removing.is_none() {
            let mut actors = [None; 4];
            for (actor, _) in runtime
                .source_registered_creature_census()
                .map_err(|_| ())?
            {
                let Some(key) = source_actor(runtime, native, actor) else {
                    continue;
                };
                let Some(index) = forms.iter().position(|form| form.key() == key) else {
                    continue;
                };
                // Creature(name) selects one actor. As with source_names, refuse
                // an ambiguous name instead of inventing registry order or removing all.
                if actors[index].replace(actor).is_some() {
                    return Err(());
                }
            }
            self.removing = Some(actors);
        }
        for retained in self.removing.as_mut().ok_or(())? {
            let Some(actor) = *retained else {
                continue;
            };
            match runtime.companion_snapshot_including_dead(actor) {
                Ok(_) => {
                    if !source_actor(runtime, native, actor)
                        .is_some_and(|key| forms.iter().any(|form| form.key() == key))
                    {
                        return Err(());
                    }
                    runtime.remove_encounter_actor(actor).map_err(|_| ())?;
                }
                Err(
                    oteryn_gameplay_domain::domain::carrier_values::CarrierError::StaleActorGeneration
                    | oteryn_gameplay_domain::domain::carrier_values::CarrierError::NotCreature,
                ) => {}
                Err(_) => return Err(()),
            }
            states.monster_melee.creature_ai.remove(actor);
            // Retain exact refs across a partial refusal; a retry never captures
            // another actor subsequently admitted under the same name or slot.
            *retained = None;
        }
        self.removed = true;
        Ok(())
    }
}

/// Same native-owner pass and original think batch as the real attack dispatcher.
pub(super) fn drive(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    occurrences: &[ThinkOccurrence],
) -> Result<(), ()> {
    let mut owner = states.soul_war_callbacks.hatred.take().ok_or(())?;
    let result = (|| {
        if !owner.entered {
            return Ok(());
        }
        if !qualifies_source(runtime, native)
            || owner.content != runtime.content_pin().server_artifact_digest()
        {
            return Err(());
        }
        let boss_present = runtime
            .companion_snapshot(owner.boss)
            .is_ok_and(|boss| boss.health > 0 && boss.state.policy.definition_key == HATRED);
        if !boss_present && !owner.removed {
            owner.remove_source_forms(runtime, states, native)?;
            return Ok(());
        }
        if owner.removed {
            return Ok(());
        }
        let captured_name = native
            .policies()
            .policies()
            .iter()
            .find(|policy| policy.definition_key == BurningForm::Ashes.key())
            .and_then(|policy| {
                oteryn_protocol_oteryn::entity_names::EntityDisplayName::new(&policy.display_name)
                    .ok()
            });
        let burning = owner.burning;
        for occurrence in occurrences
            .iter()
            .copied()
            .filter(|occurrence| occurrence.actor == burning)
        {
            owner.think(runtime, native, occurrence, |speaker, at, text| {
                let Some(map) = super::super::soul_war_scene::source_map(room) else {
                    return false;
                };
                let Some(position) = super::super::soul_war_scene::wire_position(at) else {
                    return false;
                };
                map.facts
                    .emit_movement_speech(crate::map::facts::MovementSpeech {
                        speaker,
                        position,
                        talk_type: 36,
                        text: text.into(),
                        captured_name,
                    })
            })?;
        }
        Ok(())
    })();
    states.soul_war_callbacks.hatred = Some(owner);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::native_gameplay::test_support::activated_with_loot_tables;
    use crate::foundation::{NodeId, RuntimeScopeRefV1, WorldId};
    fn id(tag: u8) -> [u8; 16] {
        let mut id = [tag; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id
    }
    #[test]
    fn srx3_hatred_real_native_think_transforms_and_empowers_once()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let world = WorldId::decode(&id(70)).map_err(|e| format!("{e:?}"))?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let native = controller
            .active()
            .and_then(|a| a.native_gameplay())
            .ok_or("actual native generation")?;
        let scope = RuntimeScopeRefV1::instance(world, id(71)).map_err(|e| format!("{e:?}"))?;
        let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
            scope,
            NodeId::decode(&id(72)).map_err(|e| format!("{e:?}"))?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            pin.into_channel_pin(),
        )?;
        runtime.install_companion_policies(native.policies().clone())?;
        let boss = runtime
            .spawn_arena_opponent(
                HATRED,
                MovementLocalPosition {
                    x: 33743,
                    y: 31596,
                    floor: 14,
                },
                |_, _| Some(true),
            )
            .map_err(|e| format!("boss {e:?}"))?;
        let burning = runtime
            .spawn_arena_opponent(
                BurningForm::Ashes.key(),
                MovementLocalPosition {
                    x: 33743,
                    y: 31599,
                    floor: 14,
                },
                |_, _| Some(true),
            )
            .map_err(|e| format!("burning {e:?}"))?;
        assert!(qualifies_source(&runtime, native));
        let mut owner = HatredCallbacks::current_actors(&runtime, native, boss, burning)
            .map_err(|_| "actual native Hatred context")?;
        owner.committed_entry();
        let mut speeches = Vec::new();
        for sequence in 1..=46 {
            owner
                .think(
                    &mut runtime,
                    native,
                    ThinkOccurrence {
                        actor: burning,
                        sequence,
                    },
                    |actor, at, text| {
                        speeches.push((actor, at, text.to_owned()));
                        true
                    },
                )
                .map_err(|_| "actual Burning think")?;
        }
        assert_eq!(
            runtime
                .companion_snapshot(burning)?
                .state
                .policy
                .definition_key,
            BurningForm::Spark.key(),
            "old remaining135 selects source Spark on the46th actual think"
        );
        for sequence in 47..=182 {
            owner
                .think(
                    &mut runtime,
                    native,
                    ThinkOccurrence {
                        actor: burning,
                        sequence,
                    },
                    |actor, at, text| {
                        speeches.push((actor, at, text.to_owned()));
                        true
                    },
                )
                .map_err(|_| "actual Burning cycle")?;
        }
        assert_eq!(
            runtime
                .companion_snapshot(burning)?
                .state
                .policy
                .definition_key,
            BurningForm::Ashes.key()
        );
        assert_eq!(runtime.companion_snapshot(burning)?.health, 15000);
        assert_eq!(runtime.soul_war_hatred_percent(boss)?, 10);
        assert_eq!(speeches.len(), 1);
        assert_eq!(speeches[0].0, burning);
        assert_eq!(
            speeches[0].2,
            "The fire of hatred fuels and empowers Goshnar's Hate!"
        );
        let snapshot = runtime.companion_snapshot(burning)?;
        let mut replayed_speech = false;
        owner
            .think(
                &mut runtime,
                native,
                ThinkOccurrence {
                    actor: burning,
                    sequence: 182,
                },
                |_, _, _| {
                    replayed_speech = true;
                    false
                },
            )
            .map_err(|_| "exact think replay")?;
        assert!(!replayed_speech, "replayed think speech");
        assert_eq!(runtime.companion_snapshot(burning)?, snapshot);
        assert_eq!(runtime.soul_war_hatred_percent(boss)?, 10);
        let mut forms = vec![burning];
        for (offset, form) in [BurningForm::Spark, BurningForm::Flame, BurningForm::Blaze]
            .into_iter()
            .enumerate()
        {
            forms.push(
                runtime
                    .spawn_arena_opponent(
                        form.key(),
                        MovementLocalPosition {
                            x: 33745 + offset as i32,
                            y: 31599,
                            floor: 14,
                        },
                        |_, _| Some(true),
                    )
                    .map_err(|e| format!("actual cleanup source form: {e:?}"))?,
            );
        }
        let mut states = ChannelSpellStates::default();
        let duplicate = runtime
            .spawn_arena_opponent(
                BurningForm::Spark.key(),
                MovementLocalPosition {
                    x: 33749,
                    y: 31599,
                    floor: 14,
                },
                |_, _| Some(true),
            )
            .map_err(|e| format!("duplicate current source name: {e:?}"))?;
        runtime.remove_encounter_actor(boss)?;
        assert!(
            owner
                .remove_source_forms(&mut runtime, &mut states, native)
                .is_err(),
            "ambiguous names cannot authorize an arbitrary actor or a bulk removal"
        );
        assert!(
            forms
                .iter()
                .all(|&actor| runtime.contains_live_creature(actor))
        );
        assert!(runtime.contains_live_creature(duplicate));
        runtime.remove_encounter_actor(duplicate)?;
        owner
            .remove_source_forms(&mut runtime, &mut states, native)
            .map_err(|_| "actual onDisappear cleanup")?;
        assert!(
            forms
                .iter()
                .all(|&actor| !runtime.contains_live_creature(actor)),
            "source onDisappear removes the four current named Burning forms through the physical owner"
        );
        let replacement = runtime
            .spawn_arena_opponent(
                BurningForm::Ashes.key(),
                MovementLocalPosition {
                    x: 33743,
                    y: 31599,
                    floor: 14,
                },
                |_, _| Some(true),
            )
            .map_err(|e| format!("later source form incarnation: {e:?}"))?;
        let previous = forms
            .iter()
            .find(|actor| actor.actor_local_id() == replacement.actor_local_id())
            .ok_or("replacement reuses a removed physical slot")?;
        assert_ne!(
            previous.actor_local_generation(),
            replacement.actor_local_generation()
        );
        owner
            .remove_source_forms(&mut runtime, &mut states, native)
            .map_err(|_| "completed cleanup replay")?;
        assert!(runtime.contains_live_creature(replacement));
        Ok(())
    }
}
