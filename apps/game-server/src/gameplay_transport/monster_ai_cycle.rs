//! First playable wild-monster dispatch: explicitly activated physical melee only.
//! Uses the actual positioned census, bound Creature policies and existing AI-4
//! owner. Summons, ranged/defensive/custom spells and movement remain disabled.
use super::ComposedFreshAdmission;
use crate::ability::creature_bite::ReentryProtection;
use crate::ai::{AiProvenance, AiProvenanceInput, ResourceLimit};
use crate::ai_monster_melee::MeleeDefinition;
use crate::ai_think::{AttackReadiness, CreatureThinkInput, PerceivedPlayer, PerceivedPlayerId};
use crate::content::{LogicalCell, ProjectV2AuthoringProfileData};
use crate::foundation::MovementLocalPosition;
use crate::foundation::owner_timer::SemanticTimeMicros;
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

impl ComposedFreshAdmission<'_, '_, '_> {
    /// One coalesced D115 think pass. The same Channel's scheduler is shared by
    /// all connections; player count never multiplies monster attack frequency.
    pub(in crate::gameplay_transport) async fn drain_monster_melee(&self) {
        let Some(room) = self.qualified_room else {
            return;
        };
        let Some(content) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return;
        };
        let now = self.owner_now();
        let mut runtime = self.runtime.lock().await;
        if runtime.owner_fence().is_err()
            || content.source_digest() != runtime.content_pin().server_artifact_digest()
            || room.compiled().server_digest() != runtime.content_pin().server_artifact_digest()
        {
            return;
        }
        let mut states = self.spell_states.lock().await;
        if now.get() < states.next_monster_ai_pass_us {
            return;
        }
        let Some(due) = now
            .get()
            .checked_add(crate::ai_think::D115_THINK_INTERVAL_MILLIS * 1_000)
        else {
            return;
        };
        let Some(sequence) = states.monster_ai_sequence.checked_add(1) else {
            return;
        };
        let Ok(census) = runtime.positioned_actor_census() else {
            return;
        };
        if census.len() > ResourceLimit::ActiveActors.maximum() {
            return;
        }
        let mut targets = census
            .iter()
            .filter_map(|(actor, position, session)| {
                let session = (*session)?;
                if states.has_pending_spell_commit(*actor, session)
                    || runtime.assert_actor_spell_unreserved(*actor).is_err()
                {
                    return None;
                }
                let state = states.get(&runtime, *actor, session)?;
                let invisible = state.owned_invisible_at(now.get()).ok()?;
                let protected = runtime
                    .current_player_reentry_protection(*actor, session, now.get())
                    .ok()?;
                let p = position.position();
                let tile = room
                    .movement_cells()
                    .spell_tiles()
                    .lookup(
                        room.movement_cells().scope(),
                        LogicalCell {
                            x: p.x,
                            y: p.y,
                            z: i32::from(p.floor),
                        },
                    )
                    .ok()?;
                Some((
                    *actor,
                    p,
                    session,
                    invisible,
                    protected || tile.flags().protection_zone,
                ))
            })
            .collect::<Vec<_>>();
        targets.sort_by_key(|(actor, ..)| actor.placement_identity());
        let monsters = census
            .iter()
            .filter(|(_, _, session)| session.is_none())
            .collect::<Vec<_>>();
        if monsters.is_empty() {
            return;
        }
        states.next_monster_ai_pass_us = due;
        states.monster_ai_sequence = sequence;
        let cursor = states.monster_ai_cursor % monsters.len();
        let count = monsters.len().min(ResourceLimit::EvaluationWork.maximum());
        let mut melee_owner = std::mem::take(&mut states.monster_melee);
        for offset in 0..count {
            let (actor, position, _) = monsters[(cursor + offset) % monsters.len()];
            let Ok(snapshot) = runtime.companion_snapshot(*actor) else {
                continue;
            };
            if snapshot.state.master.is_some() || snapshot.state.policy.is_familiar {
                continue;
            }
            let Some(record) = content.creature_profiles().records.iter().find(|record| {
                record.profile.target.key == snapshot.state.policy.definition_key
                    && record.profile.target.revision == snapshot.state.policy.definition_revision
            }) else {
                continue;
            };
            let Some(melee) = &record.monster_melee else {
                continue;
            };
            let Some(behavior) = &record.behavior else {
                continue;
            };
            let ProjectV2AuthoringProfileData::Behavior(behavior) = &behavior.data else {
                continue;
            };
            if !behavior.targeting.hostile || !behavior.targeting.can_target {
                continue;
            }
            let Some(definition) = MeleeDefinition::new(
                content.source_digest(),
                melee.interval_ms,
                melee.chance_ppm,
                melee.minimum,
                melee.maximum,
            ) else {
                continue;
            };
            let p = position.position();
            let Ok(tile) = room.movement_cells().spell_tiles().lookup(
                room.movement_cells().scope(),
                LogicalCell {
                    x: p.x,
                    y: p.y,
                    z: i32::from(p.floor),
                },
            ) else {
                continue;
            };
            if tile.flags().protection_zone {
                continue;
            }
            // Perception's limit applies to this monster's eligible local
            // candidates, not every player elsewhere in the Channel. Protected
            // players cannot occupy the nearest-candidate slot.
            let Some(indices) = local_target_indices(
                p,
                behavior.targeting.sense_invisible,
                targets
                    .iter()
                    .map(|(_, p, _, invisible, protected)| (*p, *invisible, *protected)),
            ) else {
                continue;
            };
            let perceived = indices
                .iter()
                .map(|index| PerceivedPlayer {
                    id: PerceivedPlayerId::new(*index as u64 + 1),
                    position: targets[*index].1,
                    legal_attack_target: true,
                })
                .collect::<Vec<_>>();
            let mut root = content.source_digest();
            let placement = actor.placement_identity();
            for (byte, actor_byte) in root.iter_mut().zip(placement) {
                *byte ^= actor_byte;
            }
            let mut occurrence = placement;
            for (byte, seq_byte) in occurrence.iter_mut().zip(sequence.to_be_bytes()) {
                *byte ^= seq_byte;
            }
            // Numeric provenance markers hash actual opaque identities/revisions;
            // they are local deterministic snapshot keys, never canonical IDs or grants.
            let marker = |bytes: &[u8]| {
                let mut value = [0; 8];
                value.copy_from_slice(&bytes[..8]);
                u64::from_be_bytes(value)
            };
            let input = CreatureThinkInput {
                provenance: AiProvenance::new(AiProvenanceInput {
                    scope_id: marker(runtime.binding().channel_id().as_bytes()),
                    scope_generation: runtime.binding().scope_generation().get(),
                    actor_generation: marker(&placement),
                    behavior_revision: marker(&content.source_digest()),
                    content_revision: marker(&content.source_digest()),
                    navigation_revision: marker(&room.map_revision_digest()),
                    ruleset_revision: 1,
                    determinism_profile_revision: 1,
                }),
                decision_root: GameplayDecisionRoot::from_bytes(root),
                occurrence: DecisionOccurrenceId::from_bytes(occurrence),
                path_work_id: sequence,
                position: p,
                home: p,
                attack: AttackReadiness {
                    off_cooldown: false,
                    chance_percent: 0,
                },
            };
            let digest = content
                .source_digest()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            let Ok(revisions) = crate::ability::RevisionSet::new(
                "ruleset:ai-v1",
                &format!("content:{digest}"),
                "world:ai-v1",
                "formula:source-melee-baseline",
                "simulation:v1",
            ) else {
                continue;
            };
            let _ = melee_owner.think(
                &mut runtime,
                &mut *states,
                *actor,
                sequence,
                definition,
                input,
                &perceived,
                |id| {
                    let index = usize::try_from(id.get().checked_sub(1)?).ok()?;
                    let (actor, _, session, _, protected) = *targets.get(index)?;
                    if protected {
                        return None;
                    }
                    // The held runtime owner and exact current protection
                    // read above remain live through the synchronous bite commit.
                    Some((
                        actor,
                        session,
                        ReentryProtection {
                            protected_until: None,
                        },
                    ))
                },
                revisions,
                SemanticTimeMicros::from_micros(now.get()),
            );
        }
        states.monster_melee = melee_owner;
        states.monster_ai_cursor = (cursor + count) % monsters.len();
    }
}

/// Preserve census identity/order while bounding only eligible local perception.
fn local_target_indices(
    origin: MovementLocalPosition,
    sense_invisible: bool,
    targets: impl IntoIterator<Item = (MovementLocalPosition, bool, bool)>,
) -> Option<Vec<usize>> {
    let mut indices = Vec::new();
    for (index, (position, invisible, protected)) in targets.into_iter().enumerate() {
        let distance = (i64::from(position.x) - i64::from(origin.x))
            .abs()
            .max((i64::from(position.y) - i64::from(origin.y)).abs());
        if protected
            || (invisible && !sense_invisible)
            || position.floor != origin.floor
            || distance > crate::ai_think::D115_PERCEPTION_RANGE_TILES
        {
            continue;
        }
        if indices.len() == ResourceLimit::PerceptionCandidates.maximum() {
            return None;
        }
        indices.push(index);
    }
    Some(indices)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn position(x: i32, floor: i16) -> MovementLocalPosition {
        MovementLocalPosition { x, y: 0, floor }
    }
    #[test]
    fn remote_players_do_not_consume_local_perception_budget() {
        let mut targets = vec![(position(100, 7), false, false); 65];
        targets.push((position(1, 7), false, false));
        assert_eq!(
            local_target_indices(position(0, 7), false, targets),
            Some(vec![65])
        );
        assert!(
            local_target_indices(
                position(0, 7),
                false,
                vec![(position(1, 7), false, false); 65]
            )
            .is_none()
        );
    }
    #[test]
    fn protected_nearest_and_unseen_players_do_not_block_legal_target() {
        let targets = vec![
            (position(1, 7), false, true),
            (position(1, 7), true, false),
            (position(1, 8), false, false),
            (position(2, 7), false, false),
        ];
        assert_eq!(
            local_target_indices(position(0, 7), false, targets.clone()),
            Some(vec![3])
        );
        assert_eq!(
            local_target_indices(position(0, 7), true, targets),
            Some(vec![1, 3])
        );
    }
}
