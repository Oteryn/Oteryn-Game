//! The Cathedral shares one timed hole between the pick, USE and STEP callers.
use super::*;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::stag_cathedral as source;

pub(super) fn source(map: &BundleMap, key: u64) -> bool {
    let pos = tile(source::SOURCE);
    crate::map::view::placement_of(key) == Some((pos, 0))
        && !map.facts.house_tile(pos)
        && !map.facts.base_hidden(key)
        && map
            .overlay
            .base()
            .tile(pos.x, pos.y, pos.floor)
            .and_then(|base| base.ids().first())
            .and_then(|id| map.facts.base_entry(pos, 0, *id))
            .is_some_and(|entry| matches!(entry.appearance_id, source::CLOSED | source::OPEN))
}

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(
        clippy::too_many_arguments,
        reason = "the exact pick target and Channel ground share one owner turn"
    )]
    pub(in crate::gameplay_transport) fn open_cathedral_hole(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
        session: GameSessionId,
        map: &BundleMap,
        ground: &super::super::loop_move::LoopGroundOverlay,
        key: u64,
        revision: u64,
        before: crate::map::view::EntryFacts,
    ) -> UseDisposition {
        if !source(map, key) {
            return UseDisposition::NothingToUse;
        }
        let Some(current) = self
            .states
            .loop_ground
            .current_for(runtime, map.overlay.base())
        else {
            return UseDisposition::StaleState;
        };
        if !std::ptr::eq(current.as_ref(), ground) {
            return UseDisposition::StaleState;
        }
        let stand = tile(source::STAND);
        let has_starlight = !map.facts.house_tile(stand)
            && ground.overlay().tile(stand).is_some_and(|cell| {
                cell.added().iter().any(|entry| {
                    matches!(entry.item(), crate::map::overlay::AddedItem::Ground(item)
                        if item.definition.family == "Item"
                            && item.definition.production_key == source::STARLIGHT
                            && item.quantity > 0)
                })
            });
        let pos = tile(source::SOURCE);
        if !has_starlight {
            self.states.chat.push_quest_feedback(
                session,
                "The floor does not give in. Something seems to be missing on the stand nearby.",
            );
            map.facts.emit_movement_effect(pos, 3);
            return UseDisposition::Rejected;
        }
        let Some(due) = self.owner_now().get().checked_add(source::OPEN_US) else {
            return UseDisposition::Exhausted;
        };
        let Some(after) = map.facts.quest_callback_replacement(key, source::OPEN) else {
            return UseDisposition::Rejected;
        };
        let restore = if before.appearance_id == source::CLOSED {
            before
        } else {
            let Some(restore) = map.facts.quest_callback_replacement(key, source::CLOSED) else {
                return UseDisposition::Rejected;
            };
            restore
        };
        let projection =
            super::execute::timed_entrance(due, restore, source::SOURCE, source::DESTINATION);
        if !map
            .facts
            .apply_quest_callback(key, revision, after, &[], &[], false, Some(&projection))
        {
            return UseDisposition::StaleState;
        }
        states.owner_wake.notify_one();
        self.states.chat.push_quest_feedback(
            session,
            "Hitting the ideal weak spot, the stone floor gives in and opens a narrow passage.",
        );
        map.facts.emit_movement_effect(pos, 35);
        UseDisposition::Committed
    }

    pub(super) fn descend_cathedral(
        &self,
        runtime: &mut ChannelRuntimeV1,
        proof: impl crate::foundation::SourceStepCommitProof,
    ) -> Option<MovementPositionSnapshot> {
        let creatures = runtime.current_live_creature_census().ok()?;
        if creatures.iter().any(|actor| {
            runtime.matches_live_creature_identity(*actor, source::WARLOCK.as_bytes())
                && runtime.read_actor_position(*actor).is_ok_and(|snapshot| {
                    let at = snapshot.position();
                    at.floor == 5
                        && (32399..=32404).contains(&at.x)
                        && (32466..=32471).contains(&at.y)
                })
        }) {
            return runtime.commit_source_step(proof).ok();
        }
        let native = self.active_generation?.native_gameplay()?;
        native.install_companion_policies(runtime).ok()?;
        let policy = runtime.companion_policy(source::WARLOCK).ok()?;
        let spawn = local(source::WARLOCK_SPAWN);
        let map = self.bundle_map()?;
        if self.movement_cells.index().lookup(
            self.movement_cells.scope(),
            crate::content::LogicalCell {
                x: spawn.x,
                y: spawn.y,
                z: i32::from(spawn.floor),
            },
        ) != Ok(crate::content::CollisionClass::Walkable)
            || map.facts.quest_callback_blocked(
                map.overlay.base(),
                tile(source::WARLOCK_SPAWN),
                false,
            )
            || runtime
                .positioned_melee_census()
                .ok()?
                .iter()
                .any(|(_, at, _)| at.position() == spawn)
        {
            return None;
        }
        runtime
            .commit_quest_encounter_entry(
                vec![proof],
                &[(source::WARLOCK, policy.maximum_health, spawn)],
                || true,
            )
            .ok()?
            .into_iter()
            .next()
    }

    pub(super) async fn use_cathedral_hole(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        pos: TilePos,
        key: u64,
        revision: u64,
    ) -> UseOutcome {
        let disposition = async {
            let permit = self
                .spell_lane_permit()
                .await
                .ok_or(UseDisposition::Rejected)?;
            let session = command.game_session_id;
            let expected = self
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(|_| UseDisposition::Rejected)?;
            let at = native(expected.position()).ok_or(UseDisposition::Rejected)?;
            if at.floor != pos.floor || at.x.abs_diff(pos.x) > 1 || at.y.abs_diff(pos.y) > 1 {
                return Err(UseDisposition::TooFar);
            }
            if self
                .traverse_dynamic_quest_portal(
                    &permit,
                    actor,
                    session,
                    expected,
                    pos,
                    Some((key, revision)),
                )
                .await
                == expected
            {
                return Err(UseDisposition::NothingToUse);
            }
            Ok(UseDisposition::Committed)
        }
        .await
        .unwrap_or_else(|refusal| refusal);
        UseOutcome {
            disposition,
            ..UseOutcome::rejected()
        }
    }
}
