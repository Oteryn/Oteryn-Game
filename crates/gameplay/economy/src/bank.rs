//! Pure bank results, transfer party rules and canonical coin output plans.

use oteryn_gameplay_domain::domain::currency::Coin;
use oteryn_gameplay_domain::domain::item_definition::{ItemDefinitionFacts, TypedDefinitionRef};

/// The current compatible coin definitions and the main backpack's definition (the caller's
/// Content facts): every live coin stack a deposit reads must be at its coin's revision, outputs
/// are created at these revisions, and the backpack's declared capacity bounds them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankCoinFacts {
    pub gold: TypedDefinitionRef,
    pub platinum: TypedDefinitionRef,
    pub crystal: TypedDefinitionRef,
    pub backpack: ItemDefinitionFacts,
}

/// The typed results of BANK-0 §4.1 and §4.3. BANK-NPC-1 maps them to the banker's replies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BankResult {
    Ok,
    /// A deposit's credit above `BANK0-RL-01`.
    BalanceLimit,
    InsufficientBalance,
    /// The main backpack's coins cannot pay a deposit.
    InsufficientCoins,
    /// A deposit's change or a withdrawal's coins do not fit the main backpack.
    NoRoom,
    JuniorAccount,
    UnknownRecipient,
    /// A junior recipient, or a credit above `BANK0-RL-01` on the recipient's balance.
    RecipientCannotReceiveTransfers,
    SameAccount,
}

impl BankResult {
    pub fn code(self) -> i16 {
        match self {
            Self::Ok => 0,
            Self::BalanceLimit => 1,
            Self::InsufficientBalance => 2,
            Self::InsufficientCoins => 3,
            Self::NoRoom => 4,
            Self::JuniorAccount => 5,
            Self::UnknownRecipient => 6,
            Self::RecipientCannotReceiveTransfers => 7,
            Self::SameAccount => 8,
        }
    }

    pub fn from_code(code: i16) -> Option<Self> {
        [
            Self::Ok,
            Self::BalanceLimit,
            Self::InsufficientBalance,
            Self::InsufficientCoins,
            Self::NoRoom,
            Self::JuniorAccount,
            Self::UnknownRecipient,
            Self::RecipientCannotReceiveTransfers,
            Self::SameAccount,
        ]
        .into_iter()
        .find(|result| result.code() == code)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BankCoinDirection {
    Input,
    Output,
}

/// One coin line: an input a deposit consumed (wholly when `quantity_after` is 0) or an output
/// created in a new backpack entry (`quantity_before` 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BankCoinLine {
    pub direction: BankCoinDirection,
    pub item_instance_id: [u8; 16],
    pub coin: Coin,
    pub placement_ordinal: u64,
    pub quantity_before: u32,
    pub quantity_after: u32,
}

/// The junior state of a character (BANK-0 §4.4, Q1 b): junior until it has left the starter
/// island (D119, DAWNPORT-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarterIslandFact {
    /// No starter island and departure fact exist yet: no character is junior.
    NoStarterIsland,
    OnIsland,
    Departed,
}

/// Whether the fact makes the character junior: only a character still on the island is.
pub fn is_junior(fact: StarterIslandFact) -> bool {
    matches!(fact, StarterIslandFact::OnIsland)
}

/// The party refusal of a transfer (BANK-0 §4.3): a junior sender is `JUNIOR_ACCOUNT`, a junior
/// recipient `RECIPIENT_CANNOT_RECEIVE_TRANSFERS`.
pub fn transfer_party_refusal(
    sender: StarterIslandFact,
    recipient: StarterIslandFact,
) -> Option<BankResult> {
    if is_junior(sender) {
        Some(BankResult::JuniorAccount)
    } else if is_junior(recipient) {
        Some(BankResult::RecipientCannotReceiveTransfers)
    } else {
        None
    }
}

/// The output coins of `split` in `slots` (crystal, platinum, gold) from `first_ordinal`, each
/// only when positive, in consecutive new entries. `None` past `u64::MAX`.
pub fn outputs(
    split: [(Coin, u64); 3],
    slots: [[u8; 16]; 3],
    first_ordinal: u64,
) -> Option<Vec<BankCoinLine>> {
    let mut ordinal = first_ordinal;
    let mut lines = Vec::new();
    for ((coin, quantity), slot) in split.into_iter().zip(slots) {
        if quantity == 0 {
            continue;
        }
        if ordinal == 0 {
            return None;
        }
        lines.push(BankCoinLine {
            direction: BankCoinDirection::Output,
            item_instance_id: slot,
            coin,
            placement_ordinal: ordinal,
            quantity_before: 0,
            quantity_after: u32::try_from(quantity).ok()?,
        });
        ordinal = ordinal.checked_add(1).unwrap_or(0);
    }
    Some(lines)
}

/// A withdrawal's canonical split: crystal, then platinum, then gold (BANK-0 §4.2).
pub fn withdraw_split(amount: u64) -> [(Coin, u64); 3] {
    [
        (Coin::Crystal, amount / 10_000),
        (Coin::Platinum, amount % 10_000 / 100),
        (Coin::Gold, amount % 100),
    ]
}

/// A credit only when the checked balance fits its registered maximum.
pub fn credit(balance: u64, amount: u64, maximum: u64) -> Option<u64> {
    balance
        .checked_add(amount)
        .filter(|after| *after <= maximum)
}

/// A debit only when the locked balance covers the amount.
pub fn debit(balance: u64, amount: u64) -> Option<u64> {
    balance.checked_sub(amount)
}

/// Whether the amount is positive and within its operation's registered row.
pub fn amount_within(amount: u64, cap: u64) -> bool {
    (1..=cap).contains(&amount)
}

/// The resolved recipient's canonical name key accepted by a transfer.
pub fn valid_recipient_name_key(key: &str) -> bool {
    (2..=29).contains(&key.len()) && key.bytes().all(|byte| byte.is_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    #[test]
    fn balances_and_amounts_keep_their_exact_bounds() {
        assert_eq!(credit(99, 1, 100), Some(100));
        assert_eq!(credit(100, 1, 100), None);
        assert_eq!(credit(u64::MAX, 1, u64::MAX), None);
        assert_eq!(debit(100, 100), Some(0));
        assert_eq!(debit(100, 101), None);
        assert!(amount_within(100, 100));
        assert!(!amount_within(0, 100));
        assert!(!amount_within(101, 100));
        assert!(valid_recipient_name_key("bankfriend"));
        for invalid in ["b", "BankFriend", "bank friend", "éé"] {
            assert!(!valid_recipient_name_key(invalid));
        }
    }

    fn id(seed: u8) -> [u8; 16] {
        [
            seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
        ]
    }
    #[test]
    fn a_withdrawal_is_the_canonical_split_in_consecutive_entries() {
        let lines = outputs(withdraw_split(1_009_999), [id(10), id(11), id(12)], 7).unwrap();
        assert_eq!(
            lines
                .iter()
                .map(|l| (
                    l.item_instance_id,
                    l.coin,
                    l.quantity_after,
                    l.placement_ordinal
                ))
                .collect::<Vec<_>>(),
            vec![
                (id(10), Coin::Crystal, 100, 7),
                (id(11), Coin::Platinum, 99, 8),
                (id(12), Coin::Gold, 99, 9),
            ]
        );
        // Only the needed slots are used.
        let lines = outputs(withdraw_split(10_005), [id(10), id(11), id(12)], 1).unwrap();
        assert_eq!(
            lines
                .iter()
                .map(|l| (l.item_instance_id, l.placement_ordinal))
                .collect::<Vec<_>>(),
            vec![(id(10), 1), (id(12), 2)]
        );
        // No ordinal past u64::MAX.
        assert!(outputs(withdraw_split(101), [id(10), id(11), id(12)], u64::MAX).is_none());
        assert!(outputs(withdraw_split(101), [id(10), id(11), id(12)], 0).is_none());
    }
}
