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

/// Crystal 201189f8 `data-global/scripts/creaturescripts/others/login.lua:5-6`: the towns of the
/// free-account area, by the engine town ids of the pinned catalogue (Ab'Dendriel 5, Carlin 6,
/// Kazordoon 7, Thais 8, Venore 9, Rookgaard 3, Dawnport 2). Its Dawnport Tutorial, Island of
/// Destiny and Targuna have no hometown entry in the catalogue, so no Character lives there.
/// Yalahar 17 is added by the accepted Premium decision (PREMIUM-ACTIVATION-0 §4.5, D70: "the six
/// free cities (Yalahar included)"), which Crystal's list predates.
const FREE_ACCOUNT_TOWNS: [u32; 8] = [2, 3, 5, 6, 7, 8, 9, 17];
/// The same file, :5 (`defaultTown = "Thais"`): where a home town in the premium area moves.
const FREE_ACCOUNT_DEFAULT_TOWN: u32 = 8;

/// P20 (the same file, :8): a home town outside the free-account area.
fn premium_area_town(town: u32) -> bool {
    !FREE_ACCOUNT_TOWNS.contains(&town)
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
        if !premium_area_town(self.home_town(session, None).await?) {
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
        let Some(map) = self.bundle_map() else {
            return Some(self.runtime.lock().await.respawn_position());
        };
        let table = &map.overlay.base().subareas;
        // Synthetic fixture Worlds without a hometown catalogue keep their fixture start.
        if table == &Default::default() {
            return Some(self.runtime.lock().await.respawn_position());
        }
        let town = self.home_town(session, None).await?;
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
        self.commit_home_town(
            session,
            citizenship.map(|(actor, _)| actor),
            citizenship.map(|(_, town)| town),
            None,
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
    ) -> Option<u32> {
        let map = self.bundle_map()?;
        let table = &map.overlay.base().subareas;
        let default = table.default_town?;
        if citizenship.is_some_and(|id| table.town(id).is_none()) {
            return None;
        }
        let mut fence = self.current_quest_fence(session).await.ok()??;
        if fence.runtime_scope
            != crate::foundation::RuntimeScopeRefV1::channel(self.world_id, self.channel_id)
        {
            return None;
        }
        {
            let runtime = self.runtime.lock().await;
            if runtime.binding().scope_generation() != fence.scope_ownership_generation
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
        if let Ok(proof) = crate::map::floor::prepare_quest_callback_teleport(
            &runtime,
            self.movement_cells,
            actor,
            session,
            entered,
            target,
            blocked,
            &closed,
        ) {
            let _ = runtime.commit_source_step(proof);
        }
        runtime.read_actor_position(actor).unwrap_or(entered)
    }
}

#[cfg(test)]
mod tests {
    /// P20: Crystal `login.lua:6` lists the free-account towns, with Yalahar added by
    /// PREMIUM-ACTIVATION-0 §4.5; every other hometown of the pinned catalogue is premium.
    #[test]
    fn only_the_free_account_towns_are_outside_the_premium_area() {
        let premium: Vec<u32> = (1..=22)
            .filter(|town| super::premium_area_town(*town))
            .collect();
        assert_eq!(
            premium,
            vec![1, 4, 10, 11, 12, 13, 14, 15, 16, 18, 19, 20, 21, 22]
        );
        assert!(!super::premium_area_town(super::FREE_ACCOUNT_DEFAULT_TOWN));
    }
}
