//! A source dialogue selects one fenced reward branch from two server dice.
use super::*;
use crate::foundation::GameSessionId;
use crate::gameplay_transport::SecureIdentifiers;
use oteryn_gameplay_items::reward_bag::reward_bag_bounded_draw;
use oteryn_gameplay_quests::quest::reward_callbacks::{DiceOutcome, RewardCallback};
use std::cmp::Ordering;
use std::sync::OnceLock;

// Independent uniform d6 rolls. The secret keeps prospective commands from
// revealing their outcomes to clients and keeps a live command stable on retry.
pub(in crate::gameplay_transport) fn roll(session: GameSessionId, id: u64) -> Option<(u64, u64)> {
    static SEED: OnceLock<Option<[u8; 16]>> = OnceLock::new();
    let seed = (*SEED.get_or_init(SecureIdentifiers::random::<16>))?;
    let die = |domain| {
        reward_bag_bounded_draw(*session.as_bytes(), id, seed, domain, 6).map(|face| face + 1)
    };
    Some((die(0)?, die(1)?))
}

pub(super) struct Round<'a> {
    pub(super) callback: &'a RewardCallback,
    pub(super) player: u64,
    pub(super) npc: u64,
}

pub(super) fn select<'a>(
    command: UseCommand,
    candidates: impl Iterator<Item = &'a RewardCallback>,
) -> Option<Result<Round<'a>, ()>> {
    let mut marked = candidates.filter(|callback| callback.dice_outcome.is_some());
    let first = marked.next()?;
    let Some(second) = marked.next() else {
        return Some(Err(()));
    };
    if first.dice_outcome == second.dice_outcome || marked.next().is_some() {
        return Some(Err(()));
    }
    let Some((player, npc)) = roll(command.game_session_id, command.command_id) else {
        return Some(Err(()));
    };
    let outcome = match player.cmp(&npc) {
        Ordering::Greater => DiceOutcome::Win,
        Ordering::Less => DiceOutcome::Loss,
        // Source transcripts give win and loss settlement only.
        Ordering::Equal => return Some(Err(())),
    };
    let callback = if first.dice_outcome == Some(outcome) {
        first
    } else {
        second
    };
    Some(Ok(Round {
        callback,
        player,
        npc,
    }))
}
