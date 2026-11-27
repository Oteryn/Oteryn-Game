//! B24. Crystal main@201189f871d23597b82caf88fd611454a2ddefb8:
//! data-global/scripts/custom/ambient_sounds.lua:1-36,41-50,66-107;
//! src/creatures/creatures_definitions.hpp:862-901 (separate ambient ids, zero stops).
use super::*;

pub(in crate::gameplay_transport) fn zone(
    runtime: &mut ChannelRuntimeV1,
    cells: &crate::gameplay_transport::NativeEntryMovementCells,
    actor: ExactActorRef,
) -> Option<&'static str> {
    let at = crate::gameplay_transport::bundle_map_position(runtime, cells, actor)?;
    let table = &at.map.overlay.base().subareas;
    let (_, id) = table.resolve(
        u16::try_from(at.at.x).ok()?,
        u16::try_from(at.at.y).ok()?,
        i8::try_from(at.at.floor).ok()?,
    )?;
    let area = table.areas.iter().find(|area| area.id == id)?;
    zone_key(&area.key)
}

pub(super) fn zone_key(key: &str) -> Option<&'static str> {
    // oteryn-choice: use pinned city memberships, including explicit *_city subregions.
    // Region membership alone never expands city ambience into surrounding wilderness.
    let key = key.strip_prefix("oteryn:content.area.")?;
    let city = key
        .strip_prefix("city.")
        .or_else(|| key.strip_prefix("subregion.")?.strip_suffix("_city"))?;
    match city {
        "venore" => Some("venore-city"),
        "thais" => Some("thais-city"),
        "carlin" => Some("carlin-city"),
        "edron" => Some("edron-city"),
        "kazordoon" => Some("kazordoon-city"),
        "port_hope" => Some("port-hope-city"),
        _ => None,
    }
}

impl SoundPublisher {
    pub fn forget_ambient(&mut self, session: GameSessionId) {
        self.ambient.remove(&session);
    }

    /// Private state, bounded by the runtime's current player census. Repeated polls are silent.
    pub fn ambient(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        zone: Option<&str>,
        minute: u32,
    ) {
        let Ok(binding) = context(runtime) else {
            return;
        };
        if runtime.player_control_facts(actor, session).is_err() {
            return;
        }
        if self.binding.as_ref() != Some(&binding) {
            self.sounds.clear();
            self.ambient.clear();
            self.binding = Some(binding);
        }
        self.ambient.retain(|session, (actor, _, _, _)| {
            runtime.player_control_facts(*actor, *session).is_ok()
        });
        // Crystal ambient_sounds.lua:43-50: sunrise selects day; sunset selects night.
        let day = (crate::world_clock::SUNRISE_START..crate::world_clock::SUNSET_START)
            .contains(&(minute % 1_440));
        let (zone, day_cue, night_cue) = match zone {
            Some("venore-city") => (1, 23, 28),
            Some("thais-city") => (2, 25, 28),
            Some("carlin-city") => (3, 25, 28),
            Some("edron-city") => (4, 3, 36),
            Some("kazordoon-city") => (5, 15, 41),
            Some("port-hope-city") => (6, 11, 8),
            _ => (0, 0, 0),
        };
        let cue = if day { day_cue } else { night_cue };
        if self
            .ambient
            .get(&session)
            .is_some_and(|(held, previous_zone, previous, _)| {
                *held == actor && *previous_zone == zone && *previous == cue
            })
        {
            return;
        }
        let Some(revision) = self.revision.checked_add(1) else {
            return;
        };
        self.revision = revision;
        self.ambient.insert(session, (actor, zone, cue, revision));
    }
}
