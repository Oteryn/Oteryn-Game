use super::*;
use crate::content::LogicalCell;
use crate::durability::raid_runner::RaidStepIdentity;
use crate::map::floor::{BundleDoors, CellBlocker, prepare_quest_encounter_ejection};
use oteryn_gameplay_encounter_rules::raid::{
    PALE_COUNT_EXIT_DELAY_MS, PALE_COUNT_OPEN_DELAY_MS, PALE_COUNT_RAID, PaleCountLair,
    RaidCatalogue, RaidPosition,
};

#[derive(Clone, Copy, Debug)]
pub(in crate::gameplay_transport) struct PaleCountAccess {
    lair: PaleCountLair,
    identity: RaidStepIdentity,
    deadline: Option<i64>,
}

impl PaleCountAccess {
    fn open(self, now: i64) -> bool {
        self.deadline.is_some_and(|close| {
            let open = close - PALE_COUNT_EXIT_DELAY_MS + PALE_COUNT_OPEN_DELAY_MS;
            (open..close).contains(&now)
        })
    }
}

pub(in crate::gameplay_transport) struct PaleCountBlocker<'a> {
    pub inner: &'a dyn CellBlocker,
    pub closed_lair: Option<PaleCountLair>,
}

impl PaleCountBlocker<'_> {
    pub(in crate::gameplay_transport) fn closed_lair(
        access: Option<PaleCountAccess>,
        generation: crate::foundation::ScopeOwnershipGeneration,
    ) -> Option<PaleCountLair> {
        let access = access.filter(|access| access.identity.generation == generation);
        let now = crate::world_clock::now_utc_ms().and_then(|ms| i64::try_from(ms).ok());
        if access.is_some_and(|access| now.is_some_and(|now| access.open(now))) {
            return None;
        }
        access.map(|access| access.lair).or_else(|| {
            static LAIR: std::sync::OnceLock<Option<PaleCountLair>> = std::sync::OnceLock::new();
            *LAIR.get_or_init(|| {
                let key = crate::content::ProductionKey::new(PALE_COUNT_RAID).ok()?;
                RaidCatalogue::embedded()
                    .ok()?
                    .get(&key)
                    .filter(|raid| raid.active)?
                    .pale_count_lair
            })
        })
    }
}

impl CellBlocker for PaleCountBlocker<'_> {
    fn blocks(&self, cell: LogicalCell) -> bool {
        self.inner.blocks(cell)
            || self.closed_lair.is_some_and(|lair| {
                i16::try_from(cell.z).is_ok_and(|floor| {
                    lair.area().contains(RaidPosition {
                        x: cell.x,
                        y: cell.y,
                        floor,
                    })
                })
            })
    }

    fn suppresses_native_teleport(&self, cell: LogicalCell) -> bool {
        self.inner.suppresses_native_teleport(cell)
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn project_pale_count_access(
        &self,
        lair: PaleCountLair,
        identity: RaidStepIdentity,
        now_ms: i64,
    ) -> Result<(), ()> {
        let map = self.bundle_map().ok_or(())?;
        let deadline = self
            .root
            .pale_count_deadline(
                self.holder,
                identity,
                now_ms,
                self.reset_epoch,
                map.overlay.base().digest(),
            )
            .await
            .map_err(|_| ())?;
        let runtime = self.runtime.lock().await;
        if runtime.binding().scope_generation() != identity.generation
            || runtime.owner_fence().is_err()
        {
            return Err(());
        }
        let mut states = self.states.spell_states.lock().await;
        if !states.pale_count_access.is_some_and(|current| {
            current.identity.generation == identity.generation
                && current.identity.firing_id != identity.firing_id
                && current
                    .deadline
                    .is_some_and(|close| close > now_ms && Some(close) >= deadline)
        }) {
            states.pale_count_access = Some(PaleCountAccess {
                lair,
                identity,
                deadline,
            });
        }
        Ok(())
    }

    pub(in crate::gameplay_transport) async fn evacuate_pale_count_lair(
        &self,
        lair: PaleCountLair,
        identity: RaidStepIdentity,
    ) -> Result<(), ()> {
        let _permit = self.spell_lane_permit().await.ok_or(())?;
        let map = self.bundle_map().ok_or(())?;
        let mut runtime = self.runtime.lock().await;
        if runtime.binding().scope_generation() != identity.generation
            || runtime.owner_fence().is_err()
        {
            return Err(());
        }
        let mut states = self.states.spell_states.lock().await;
        if let Some(access) = states.pale_count_access
            && access.identity.firing_id != identity.firing_id
            && access.deadline.is_some()
        {
            // An obsolete LOST cleanup cannot close a newer firing's room.
            return Ok(());
        }
        states.pale_count_access = Some(PaleCountAccess {
            lair,
            identity,
            deadline: None,
        });
        let exit = lair.exit;
        let destination = MovementLocalPosition {
            x: exit.x,
            y: exit.y,
            floor: exit.floor,
        };
        let doors = BundleDoors(&map.facts);
        let now = self.owner_now().get();
        let players = runtime.positioned_melee_census().map_err(|_| ())?;
        for (actor, before, session) in players {
            let Some(session) = session else {
                continue;
            };
            let from = before.position();
            if !lair.area().contains(RaidPosition {
                x: from.x,
                y: from.y,
                floor: from.floor,
            }) {
                continue;
            }
            let proof = prepare_quest_encounter_ejection(
                &runtime,
                self.movement_cells(),
                actor,
                session,
                before,
                destination,
                &doors,
            )
            .map_err(|_| ())?;
            let after = runtime
                .commit_source_step(proof)
                .map_err(|_| ())?
                .position();
            states.stage_special_movement_step(
                &runtime,
                map,
                actor,
                session,
                now,
                Some(from),
                after,
                None,
            );
            use super::super::paced_movement::underwater_drown::underwater;
            states.stage_underwater_step(
                &runtime,
                actor,
                session,
                now,
                underwater(map, from),
                underwater(map, after),
            );
        }
        Ok(())
    }
}
