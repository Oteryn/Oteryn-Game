//! I13 client capture: use a real Bag You Covet and retain the inventory payload.
//!
//! The bag only enters the game as the reward of a level-250 premium quest
//! (`content/quests/definitions/quests-00100-00199.json`) or as random loot;
//! no server path grants it to a fresh qualification character, and minting
//! it with raw SQL would skip the mint receipt and audit outbox. The row
//! therefore reports the missing content precondition instead of faking it.
use super::*;

pub(super) async fn run_packet(_clients: &SeamClients<'_>) {
    evidence(
        "e2e_all scenario=I13.client status=SKIPPED seconds=0.000 evidence=content:i43895-only-level-250-quest-reward-or-random-loot",
    );
}
