//! Crystal 201189f eventcallback_on_combat_taint.lua:6–58.
//! Data-only delayed callback queue; callers prove actual actors, source and map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    Warning(Position),
    Create {
        player: u64,
        target: u64,
        at: Position,
        realm: u8,
    },
}
#[derive(Debug)]
struct Pending {
    player: u64,
    target: u64,
    at: Position,
    realm: u8,
    born: u64,
    next: u8,
}
#[derive(Debug, Default)]
pub struct SecondTaint {
    pending: Vec<Pending>,
    cooldowns: Vec<(u64, u64)>,
}
impl SecondTaint {
    pub fn schedule(&mut self, now_ms: u64, player: u64, target: u64, at: Position, realm: u8) {
        self.pending.push(Pending {
            player,
            target,
            at,
            realm,
            born: now_ms,
            next: 1,
        });
    }
    pub fn advance(
        &mut self,
        now_ms: u64,
        utc_seconds: u64,
        mut exists: impl FnMut(u64, u64) -> bool,
    ) -> Vec<Effect> {
        let mut effects = Vec::new();
        loop {
            let due = self
                .pending
                .iter()
                .enumerate()
                .filter_map(|(index, p)| {
                    p.born
                        .checked_add(u64::from(p.next) * 1000)
                        .filter(|due| *due <= now_ms)
                        .map(|due| (due, index))
                })
                .min();
            let Some((_, index)) = due else {
                break;
            };
            let pending = &mut self.pending[index];
            if pending.next < 4 {
                effects.push(Effect::Warning(pending.at));
                pending.next += 1;
                continue;
            }
            let pending = self.pending.remove(index);
            if !exists(pending.player, pending.target) {
                continue;
            }
            let old = self
                .cooldowns
                .iter()
                .position(|(player, _)| *player == pending.player);
            if old.is_some_and(|index| utc_seconds <= self.cooldowns[index].1) {
                continue;
            }
            let Some(until) = utc_seconds.checked_add(30) else {
                continue;
            };
            if let Some(index) = old {
                self.cooldowns[index].1 = until;
            } else {
                self.cooldowns.push((pending.player, until));
            }
            effects.push(Effect::Create {
                player: pending.player,
                target: pending.target,
                at: pending.at,
                realm: pending.realm,
            });
        }
        effects
    }
    pub fn next_due_ms(&self) -> Option<u64> {
        self.pending
            .iter()
            .filter_map(|p| p.born.checked_add(u64::from(p.next) * 1000))
            .min()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const AT: Position = Position {
        x: 1,
        y: 2,
        floor: 14,
    };
    #[test]
    fn srx3_second_taint_warning_positions_survive_both_original_actors_disappearing() {
        let mut owner = SecondTaint::default();
        owner.schedule(100, 1, 2, AT, 3);
        for now in [1100, 2100, 3100] {
            assert_eq!(
                owner.advance(now, 100, |_, _| false),
                vec![Effect::Warning(AT)]
            );
        }
        assert!(owner.advance(4100, 100, |_, _| false).is_empty());
        assert_eq!(owner.next_due_ms(), None);
    }
    #[test]
    fn srx3_second_taint_concurrent_pending_callbacks_share_strict_second_cooldown() {
        let mut owner = SecondTaint::default();
        owner.schedule(0, 1, 2, AT, 3);
        owner.schedule(0, 1, 4, AT, 3);
        let effects = owner.advance(4000, 100, |_, _| true);
        assert_eq!(
            effects
                .iter()
                .filter(|e| matches!(e, Effect::Create { .. }))
                .count(),
            1
        );
        // A failed native creation still consumed the original cooldown.
        owner.schedule(5000, 1, 2, AT, 3);
        assert!(
            !owner
                .advance(9000, 130, |_, _| true)
                .iter()
                .any(|e| matches!(e, Effect::Create { .. }))
        );
        owner.schedule(10000, 1, 2, AT, 3);
        assert!(
            owner
                .advance(14000, 131, |_, _| true)
                .iter()
                .any(|e| matches!(e, Effect::Create { .. }))
        );
    }
}
