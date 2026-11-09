//! Pure callback timer DATA; source qualification stays with its physical owner.
use oteryn_gameplay_encounters::bosses::goshnars_greed::GoshnarsGreedRoom;

#[derive(Debug, Default)]
pub(crate) struct EntryState {
    entered: bool,
}
impl EntryState {
    pub(crate) fn claim(&mut self) -> bool {
        !std::mem::replace(&mut self.entered, true)
    }
}

pub(crate) fn greed_at_birth() -> GoshnarsGreedRoom {
    GoshnarsGreedRoom::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srx3_committed_soul_war_entry_is_claimed_once() {
        let mut entry = EntryState::default();
        assert!(entry.claim());
        assert!(
            !entry.claim(),
            "second participant and recovered transfer cannot restart timers"
        );
    }
    #[test]
    fn srx3_greed_prewarmed_birth_does_not_start_source_lever_timers() {
        let mut clock = greed_at_birth();
        let _ = clock.boss_spawned();
        assert_eq!(clock.next_due_ms(), None, "onSpawn is not onUseExtra");
        assert!(
            clock.advance(60_000).is_empty(),
            "empty prewarm emits no warnings/spawns"
        );
    }
}
