//! C20d-W23: the pinned hometown catalogue and citizen step ingress.
use super::*;
use crate::foundation::{MovementLocalPosition, MovementPositionSnapshot};

/// Crystal/Canary citizen.lua: these are placement unique ids, not appearance ids.
fn citizen_town(unique: u16) -> Option<u32> {
    Some(match unique {
        9056 => 6,
        9057 => 8,
        9058 => 9,
        9059 => 5,
        9060 => 7,
        9061 => 13,
        9062 => 10,
        9063 => 11,
        9064 => 14,
        9065 => 15,
        9066 => 16,
        9067 => 17,
        9068 => 12,
        9240 => 18,
        9500 => 21,
        9510 => 20,
        9515 => 22,
        _ => return None,
    })
}

/// Crystal 201189f8 `data-global/scripts/creaturescripts/others/login.lua:5-8` and
/// `data-global/lib/tables/town.lua:1-30`: the hometowns in the free-account area. Dawnport
/// Tutorial and Island of Destiny are not hometowns in the pinned Oteryn catalogue.
/// Targuna34 is admitted by the M38 source-town adapter and is free-account territory. Yalahar
/// 17 is added by the accepted Premium decision (PREMIUM-ACTIVATION-0 §4.5, D70: "the six free
/// cities (Yalahar included)"), which Crystal's list predates.
const FREE_ACCOUNT_TOWNS: [u32; 9] = [2, 3, 5, 6, 7, 8, 9, 17, 34];
/// The remaining hometowns admitted by the pinned catalogue are Premium territory. Crystal's
/// two movement-side checks agree with the boundary: `premium_bridge.lua:1-17` protects the
/// Rookgaard Premium bridge, while `oramond/teleport.lua:1-29` makes each Oramond destination's
/// Premium flag explicit. Non-hometown engine towns and unknown ids are intentionally unclassified.
const PREMIUM_ACCOUNT_TOWNS: [u32; 11] = [10, 11, 12, 13, 14, 15, 16, 18, 20, 21, 22];
/// The same file, :5 (`defaultTown = "Thais"`): where a home town in the premium area moves.
const FREE_ACCOUNT_DEFAULT_TOWN: u32 = 8;

/// P20 (the same file, :8): a home town outside the free-account area.
fn premium_area_town(town: u32) -> Option<bool> {
    FREE_ACCOUNT_TOWNS
        .contains(&town)
        .then_some(false)
        .or_else(|| PREMIUM_ACCOUNT_TOWNS.contains(&town).then_some(true))
}

/// Crystal `login.lua:15`.
const PREMIUM_EXPIRED_TEXT: &str = "Your premium time has expired!";

impl ComposedFreshAdmission<'_, '_, '_> {
    /// P20: at login, a Character whose account's Premium entitlement ended and whose home town
    /// is in the premium area becomes a citizen of Thais and enters at its temple (Crystal
    /// `login.lua:8-15`). `None` leaves the admission as it was: Premium current or unknown, a
    /// free-area town, or no hometown catalogue. Citizenship is committed only after placement.
    pub(super) async fn premium_ended_temple(
        &self,
        account: [u8; 16],
        session: GameSessionId,
    ) -> Option<MovementLocalPosition> {
        let coordinator = self.premium_coordinator?;
        let table = &self.bundle_map()?.overlay.base().subareas;
        let temple = table.town(FREE_ACCOUNT_DEFAULT_TOWN)?.temple?;
        if premium_area_town(self.home_town(session, None).await?) != Some(true) {
            return None;
        }
        // Settle delivery, then load the durable fence: this account's pulls start only after
        // admission, and an ended entitlement is a restrictive durable fact.
        let _ = coordinator.premium_gate(account).await;
        let _ = coordinator.consumer().load(self.root, account).await;
        if !coordinator.premium_entitlement_ended(account) {
            return None;
        }
        Some(MovementLocalPosition {
            x: i32::from(temple.x),
            y: i32::from(temple.y),
            floor: i16::from(-temple.floor),
        })
    }

    /// Persist the actual P20 placement and consume its obligation before changing citizenship.
    pub(super) async fn commit_premium_ended_relocation(
        &self,
        session: GameSessionId,
        actor: ExactActorRef,
        decided: &last_position::AdmissionPosition,
        placed: crate::durability::character_last_position::FinalLastPosition,
    ) -> bool {
        use last_position::AdmissionPosition;
        let relocation = crate::durability::character_home_town::HomeTownRelocation {
            cause: crate::durability::character_home_town::HomeTownRelocationCause::PremiumExpired,
            placed,
            respawn: match decided {
                AdmissionPosition::Respawn(pending) => Some(pending.occurrence),
                _ => None,
            },
            arrival: match decided {
                AdmissionPosition::Arrival(pending, _, _) => Some(pending.occurrence),
                _ => None,
            },
        };
        if self
            .commit_home_town(
                session,
                Some(actor),
                Some(FREE_ACCOUNT_DEFAULT_TOWN),
                Some(relocation),
                self,
            )
            .await
            != Some(FREE_ACCOUNT_DEFAULT_TOWN)
        {
            return false;
        }
        self.states
            .chat
            .push_quest_feedback(session, PREMIUM_EXPIRED_TEXT);
        true
    }

    pub(super) async fn home_temple(
        &self,
        session: GameSessionId,
    ) -> Option<MovementLocalPosition> {
        let scope = self.runtime.lock().await.binding().runtime_scope();
        if matches!(scope, crate::foundation::RuntimeScopeRefV1::Instance { .. }) {
            return self.activity_home_temple(session).await;
        }
        self.home_temple_in_world(session, self).await
    }

    /// The current Instance supplies authority to read citizenship; the retained
    /// origin Channel supplies the authored temple catalogue and physical pin.
    async fn activity_home_temple(&self, session: GameSessionId) -> Option<MovementLocalPosition> {
        let fence = self.current_quest_fence(session).await.ok()??;
        let placement =
            crate::durability::activity_handoff::ActivityHandoffOwner::new(self.root.clone())
                .read_return_placement(self.character, self.holder, fence.item_fence())
                .await
                .ok()??;
        let origin = self.activity_runtimes.get(
            placement.destination_scope,
            placement.destination_generation.get(),
        )?;
        let origin_view = self.runtime_view(&origin);
        let pin = placement.exit_position;
        {
            let runtime = origin.runtime.lock().await;
            runtime.owner_fence().ok()?;
            if runtime.binding().runtime_scope() != placement.destination_scope
                || runtime.binding().scope_generation() != placement.destination_generation
                || runtime.content_pin().frame_binding_digest() != pin.coordinate_frame_digest
                || runtime.map_revision_digest() != pin.map_revision_digest
                || runtime.content_pin().server_artifact_digest() != pin.server_artifact_digest
            {
                return None;
            }
        }
        let temple = self.home_temple_in_world(session, &origin_view).await?;
        let doors = origin.door.lock().await;
        super::respawn_cell_admissible(
            origin_view.movement_cells().index(),
            origin_view.movement_cells().scope(),
            placement.destination_scope.world_id(),
            pin.server_artifact_digest,
            doors.blocking_cells(),
            temple,
        )
        .then_some(temple)
    }

    async fn home_temple_in_world(
        &self,
        session: GameSessionId,
        world: &Self,
    ) -> Option<MovementLocalPosition> {
        let Some(map) = world.bundle_map() else {
            return Some(world.runtime.lock().await.respawn_position());
        };
        let table = &map.overlay.base().subareas;
        // Synthetic fixture Worlds without a hometown catalogue keep their fixture start.
        if table == &Default::default() {
            return Some(world.runtime.lock().await.respawn_position());
        }
        let town = self.home_town_in_world(session, None, world).await?;
        // TUTORIAL-1 retains its accepted islet exception. Read the durable step now;
        // the death transaction resets that same step when it records the islet cell.
        if self.tutorial_enabled().await {
            let fence = self.current_quest_fence(session).await.ok()??;
            if let Some(stored) = self.tutorial_progress(fence).await.ok()?
                && stored.progress.step != oteryn_gameplay_quests::tutorial::TutorialStep::Departed
            {
                let build = self
                    .root
                    .read_character_build_state(self.character, fence.character_id)
                    .await
                    .ok()?;
                let group = self
                    .root
                    .read_familiar_group(self.character, self.holder, fence)
                    .await
                    .ok()??;
                if oteryn_gameplay_quests::tutorial::tutorial_transition(
                    true,
                    Some(stored.progress),
                    oteryn_gameplay_quests::tutorial::TutorialEvent::Death,
                    build.vocation(),
                    group.access(),
                )
                .is_some_and(|next| next.place_islet)
                {
                    let (x, y, floor) = oteryn_gameplay_quests::tutorial::ISLET_TEMPLE;
                    return Some(MovementLocalPosition { x, y, floor });
                }
            }
        }
        let temple = table.town(town)?.temple?;
        Some(MovementLocalPosition {
            x: i32::from(temple.x),
            y: i32::from(temple.y),
            floor: i16::from(-temple.floor),
        })
    }

    pub(super) async fn home_town(
        &self,
        session: GameSessionId,
        citizenship: Option<(ExactActorRef, u32)>,
    ) -> Option<u32> {
        self.home_town_in_world(session, citizenship, self).await
    }

    async fn home_town_in_world(
        &self,
        session: GameSessionId,
        citizenship: Option<(ExactActorRef, u32)>,
        world: &Self,
    ) -> Option<u32> {
        self.commit_home_town(
            session,
            citizenship.map(|(actor, _)| actor),
            citizenship.map(|(_, town)| town),
            None,
            world,
        )
        .await
    }

    /// [`Self::home_town`]; a citizenship chosen by an actor's step also proves that actor's
    /// bound lease. P20 commits only after the first-entry step binds the lease and places it.
    async fn commit_home_town(
        &self,
        session: GameSessionId,
        actor: Option<ExactActorRef>,
        citizenship: Option<u32>,
        relocation: Option<crate::durability::character_home_town::HomeTownRelocation>,
        world: &Self,
    ) -> Option<u32> {
        let fence = self.current_quest_fence(session).await.ok()??;
        self.commit_home_town_at_fence(session, actor, (citizenship, relocation), world, fence)
            .await
    }

    /// Retain the authority of an already captured placement across asynchronous reads.
    pub(super) async fn commit_home_town_at_fence(
        &self,
        session: GameSessionId,
        actor: Option<ExactActorRef>,
        change: (
            Option<u32>,
            Option<crate::durability::character_home_town::HomeTownRelocation>,
        ),
        world: &Self,
        mut fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    ) -> Option<u32> {
        let (citizenship, relocation) = change;
        let map = world.bundle_map()?;
        let table = &map.overlay.base().subareas;
        let default = table.default_town?;
        if citizenship.is_some_and(|id| table.town(id).is_none()) {
            return None;
        }
        // A Channel must match this composed owner. An activity may read its
        // existing World citizenship, but cannot claim a private citizen tile.
        if fence.runtime_scope.world_id() != self.world_id
            || match fence.runtime_scope {
                crate::foundation::RuntimeScopeRefV1::Channel { channel_id, .. } => {
                    channel_id != self.channel_id
                }
                crate::foundation::RuntimeScopeRefV1::Instance { .. } => citizenship.is_some(),
            }
        {
            return None;
        }
        {
            let runtime = self.runtime.lock().await;
            if runtime.binding().runtime_scope() != fence.runtime_scope
                || runtime.binding().scope_generation() != fence.scope_ownership_generation
                || runtime.owner_fence().is_err()
            {
                return None;
            }
            if let Some(actor) = actor {
                let lease = runtime.bound_attacker_lease(actor, session).ok()?;
                if lease.character_id().as_bytes() != fence.character_id.as_bytes()
                    || lease.generation() != fence.character_lease_generation
                {
                    return None;
                }
            }
        }
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        fence.expected_character_revision = slot.cursor(self.root, self.character).await.ok()?;
        let outcome = self
            .root
            .character_home_town(
                self.character,
                self.holder,
                fence,
                default,
                citizenship,
                relocation,
            )
            .await;
        slot.observe_home_town();
        outcome.ok()
    }

    /// A portal may already have applied its map teleport. Inspect the entered cardinal tile,
    /// rather than only the final landing, while retaining the ordinary movement lane.
    pub(super) async fn citizen_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        direction: StepDirection,
        origin: MovementPositionSnapshot,
        entered: MovementPositionSnapshot,
    ) -> MovementPositionSnapshot {
        let Some(map) = self.bundle_map() else {
            return entered;
        };
        let p = origin.position();
        let (dx, dy) = match direction {
            StepDirection::North => (0, -1),
            StepDirection::East => (1, 0),
            StepDirection::South => (0, 1),
            StepDirection::West => (-1, 0),
            StepDirection::NorthEast => (1, -1),
            StepDirection::SouthEast => (1, 1),
            StepDirection::SouthWest => (-1, 1),
            StepDirection::NorthWest => (-1, -1),
        };
        let (Ok(x), Ok(y), Ok(floor)) = (
            u16::try_from(p.x + dx),
            u16::try_from(p.y + dy),
            i8::try_from(-p.floor),
        ) else {
            return entered;
        };
        let Some(tile) = map.overlay.base().tile(x, y, floor) else {
            return entered;
        };
        let mut citizens = (0..tile.depths().iter().filter(|d| **d == 0).count())
            .filter_map(|ordinal| tile.unique(u8::try_from(ordinal).ok()?))
            .filter_map(|entry| citizen_town(entry.unique));
        let Some(town) = citizens.next() else {
            return entered;
        };
        if citizens.next().is_some() {
            return entered;
        }
        let Some(temple) = map
            .overlay
            .base()
            .subareas
            .town(town)
            .and_then(|a| a.temple)
        else {
            return entered;
        };
        let current = {
            let runtime = self.runtime.lock().await;
            runtime.read_actor_position(actor) == Ok(entered)
                && runtime.player_control_facts(actor, session).is_ok()
        };
        if !current {
            return entered;
        }
        let committed = self.home_town(session, Some((actor, town))).await;
        if town == 16 && committed.is_some_and(|id| id != town) {
            self.states.chat.push_quest_feedback(
                session,
                "You first need to absolve the Barbarian Test Quest to become citizen!",
            );
        }
        let target = if committed.is_some() {
            MovementLocalPosition {
                x: i32::from(temple.x),
                y: i32::from(temple.y),
                floor: i16::from(-temple.floor),
            }
        } else {
            origin.position()
        };
        let mut runtime = self.runtime.lock().await;
        if runtime.read_actor_position(actor) != Ok(entered) {
            return entered;
        }
        let states = self.states.spell_states.lock().await;
        let closed = crate::map::floor::BundleDoors(map.facts.doors());
        let blocked = super::pvp_live::pz_blocked(&states, actor, session, self.owner_now());
        if let Ok(proof) = crate::map::floor::prepare_quest_callback_teleport_facing(
            &runtime,
            self.movement_cells(),
            actor,
            session,
            entered,
            target,
            blocked,
            &closed,
            None,
        ) {
            // Crystal 201189f8 data-global/scripts/movements/teleport/citizen.lua:46-49:
            // successful citizenship is confirmed after the temple teleport.
            if runtime.commit_source_step(proof).is_ok()
                && committed == Some(town)
                && let Some(area) = map.overlay.base().subareas.town(town)
            {
                self.states.chat.push_quest_feedback(
                    session,
                    &format!("You are now a citizen of {}.", area.name),
                );
            }
        }
        runtime.read_actor_position(actor).unwrap_or(entered)
    }
}

#[cfg(test)]
mod tests {
    /// P20: Crystal 201189f8 `data-global/lib/tables/town.lua:1-30` supplies the ids and
    /// `login.lua:6` the free names; the hometown subset is pinned in area-report.json.
    #[test]
    fn p20_pinned_hometowns_have_a_complete_premium_partition() {
        let free = [2, 3, 5, 6, 7, 8, 9, 17, 34];
        let premium = [10, 11, 12, 13, 14, 15, 16, 18, 20, 21, 22];
        for town in free {
            assert_eq!(super::premium_area_town(town), Some(false), "town {town}");
        }
        for town in premium {
            assert_eq!(super::premium_area_town(town), Some(true), "town {town}");
        }
        for non_hometown in [1, 4, 19, 23, 24, 25, 26, 32, 33] {
            assert_eq!(super::premium_area_town(non_hometown), None);
        }
    }
}
