//! Shared coin-exchange rules. Persistence applies a successful plan as one item transaction.

use oteryn_gameplay_domain::domain::currency::Coin;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoinExchange {
    GoldToPlatinum,
    PlatinumToGold,
    PlatinumToCrystal,
    CrystalToPlatinum,
}

impl CoinExchange {
    pub const fn input(self) -> (Coin, u64) {
        match self {
            Self::GoldToPlatinum => (Coin::Gold, 100),
            Self::PlatinumToGold => (Coin::Platinum, 1),
            Self::PlatinumToCrystal => (Coin::Platinum, 100),
            Self::CrystalToPlatinum => (Coin::Crystal, 1),
        }
    }

    pub const fn output(self) -> (Coin, u64) {
        match self {
            Self::GoldToPlatinum => (Coin::Platinum, 1),
            Self::PlatinumToGold => (Coin::Gold, 100),
            Self::PlatinumToCrystal => (Coin::Crystal, 1),
            Self::CrystalToPlatinum => (Coin::Platinum, 100),
        }
    }

    pub const fn keyword(self) -> &'static str {
        match self {
            Self::GoldToPlatinum => "change gold",
            Self::PlatinumToGold | Self::PlatinumToCrystal => "change platinum",
            Self::CrystalToPlatinum => "change crystal",
        }
    }

    pub const fn insufficient_coins(self) -> &'static str {
        match self.input().0 {
            Coin::Gold => "Sorry, you do not have enough gold coins.",
            Coin::Platinum => "Sorry, you do not have enough platinum coins.",
            Coin::Crystal => "Sorry, you do not have enough crystal coins.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeRefusal {
    InvalidAmount,
    InsufficientCoins,
    NoRoom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExchangePlan {
    pub input: (Coin, u64),
    pub output: (Coin, u64),
}

/// How the exchange was requested. Converter tools spend one charge only after the complete
/// coin replacement has qualified; plain coin use has no tool mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeUse {
    Coin,
    GoldConverter { charges_before: u32 },
    MagicGoldConverter { charges_before: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemExchangePlan {
    pub exchange: ExchangePlan,
    pub charges_after: Option<u32>,
    pub remove_converter: bool,
}

/// Qualifies a single conversion. Direct use converts full gold/platinum stacks up; smaller
/// platinum stacks and every crystal stack change one coin down. Converter modes retain the
/// draft's qualification only; production currently applies plain coin use. Callers must apply
/// every field atomically or apply none of them.
pub fn qualify_item_use(
    coin: Coin,
    quantity: u64,
    request: ExchangeUse,
    free_slots_after_input: usize,
    output_can_merge: bool,
) -> Result<ItemExchangePlan, ExchangeRefusal> {
    let exchange = match (coin, quantity, request) {
        (Coin::Gold, 100, _) => CoinExchange::GoldToPlatinum,
        (Coin::Platinum, 100, _) => CoinExchange::PlatinumToCrystal,
        (Coin::Platinum, _, ExchangeUse::Coin | ExchangeUse::GoldConverter { .. })
            if quantity > 0 =>
        {
            CoinExchange::PlatinumToGold
        }
        (Coin::Crystal, _, ExchangeUse::Coin | ExchangeUse::GoldConverter { .. })
            if quantity > 0 =>
        {
            CoinExchange::CrystalToPlatinum
        }
        _ => return Err(ExchangeRefusal::InvalidAmount),
    };
    let charges_after = match request {
        ExchangeUse::Coin => None,
        ExchangeUse::GoldConverter { charges_before }
        | ExchangeUse::MagicGoldConverter { charges_before } => Some(
            charges_before
                .checked_sub(1)
                .ok_or(ExchangeRefusal::InvalidAmount)?,
        ),
    };
    Ok(ItemExchangePlan {
        exchange: qualify(
            exchange,
            1,
            quantity,
            free_slots_after_input,
            output_can_merge,
        )?,
        charges_after,
        remove_converter: charges_after == Some(0),
    })
}

/// Qualifies the complete exchange before persistence mutates either stack. `free_slots_after_input`
/// is computed after consuming the input, so consuming a whole stack can make room for the output.
pub fn qualify(
    exchange: CoinExchange,
    output_count: u64,
    available_input: u64,
    free_slots_after_input: usize,
    output_can_merge: bool,
) -> Result<ExchangePlan, ExchangeRefusal> {
    if output_count == 0 {
        return Err(ExchangeRefusal::InvalidAmount);
    }
    let (input_coin, per_output) = exchange.input();
    let input_count = per_output
        .checked_mul(output_count)
        .ok_or(ExchangeRefusal::InvalidAmount)?;
    let (output_coin, output_per_exchange) = exchange.output();
    let output_count = output_per_exchange
        .checked_mul(output_count)
        .ok_or(ExchangeRefusal::InvalidAmount)?;
    if available_input < input_count {
        return Err(ExchangeRefusal::InsufficientCoins);
    }
    if !output_can_merge && free_slots_after_input == 0 {
        return Err(ExchangeRefusal::NoRoom);
    }
    Ok(ExchangePlan {
        input: (input_coin, input_count),
        output: (output_coin, output_count),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banker_rates_match_both_reference_scripts() {
        for (exchange, input, output) in [
            (
                CoinExchange::GoldToPlatinum,
                (Coin::Gold, 100),
                (Coin::Platinum, 1),
            ),
            (
                CoinExchange::PlatinumToGold,
                (Coin::Platinum, 1),
                (Coin::Gold, 100),
            ),
            (
                CoinExchange::PlatinumToCrystal,
                (Coin::Platinum, 100),
                (Coin::Crystal, 1),
            ),
            (
                CoinExchange::CrystalToPlatinum,
                (Coin::Crystal, 1),
                (Coin::Platinum, 100),
            ),
        ] {
            assert_eq!(
                qualify(exchange, 1, input.1, 1, false),
                Ok(ExchangePlan { input, output })
            );
        }
    }

    #[test]
    fn qualification_is_all_or_nothing_for_funds_overflow_and_room() {
        assert_eq!(
            qualify(CoinExchange::GoldToPlatinum, 2, 199, 1, false),
            Err(ExchangeRefusal::InsufficientCoins)
        );
        assert_eq!(
            qualify(CoinExchange::CrystalToPlatinum, 1, 1, 0, false),
            Err(ExchangeRefusal::NoRoom)
        );
        assert_eq!(
            qualify(CoinExchange::GoldToPlatinum, u64::MAX, u64::MAX, 1, true),
            Err(ExchangeRefusal::InvalidAmount)
        );
    }

    #[test]
    fn item_use_matches_direct_and_converter_directions() {
        assert_eq!(
            qualify_item_use(Coin::Gold, 100, ExchangeUse::Coin, 1, false),
            Ok(ItemExchangePlan {
                exchange: ExchangePlan {
                    input: (Coin::Gold, 100),
                    output: (Coin::Platinum, 1),
                },
                charges_after: None,
                remove_converter: false,
            })
        );

        assert_eq!(
            qualify_item_use(
                Coin::Crystal,
                4,
                ExchangeUse::GoldConverter { charges_before: 2 },
                1,
                false,
            ),
            Ok(ItemExchangePlan {
                exchange: ExchangePlan {
                    input: (Coin::Crystal, 1),
                    output: (Coin::Platinum, 100),
                },
                charges_after: Some(1),
                remove_converter: false,
            })
        );
    }

    #[test]
    fn magic_converter_only_changes_full_stacks_up() {
        assert_eq!(
            qualify_item_use(
                Coin::Platinum,
                1,
                ExchangeUse::MagicGoldConverter { charges_before: 4 },
                1,
                false,
            ),
            Err(ExchangeRefusal::InvalidAmount)
        );
        assert_eq!(
            qualify_item_use(
                Coin::Platinum,
                100,
                ExchangeUse::MagicGoldConverter { charges_before: 1 },
                0,
                true,
            ),
            Ok(ItemExchangePlan {
                exchange: ExchangePlan {
                    input: (Coin::Platinum, 100),
                    output: (Coin::Crystal, 1),
                },
                charges_after: Some(0),
                remove_converter: true,
            })
        );
    }

    #[test]
    fn item_conversion_refuses_without_room_or_charge_before_mutation() {
        assert_eq!(
            qualify_item_use(Coin::Crystal, 1, ExchangeUse::Coin, 0, false),
            Err(ExchangeRefusal::NoRoom)
        );
        assert_eq!(
            qualify_item_use(
                Coin::Gold,
                100,
                ExchangeUse::GoldConverter { charges_before: 0 },
                1,
                false,
            ),
            Err(ExchangeRefusal::InvalidAmount)
        );
    }
    #[test]
    fn coin_exchange_whole_input_stack_and_existing_receiver_make_room() {
        assert!(qualify(CoinExchange::GoldToPlatinum, 1, 100, 1, false).is_ok());
        assert!(qualify(CoinExchange::GoldToPlatinum, 1, 100, 0, true).is_ok());
        assert_eq!(
            qualify(CoinExchange::GoldToPlatinum, 0, 100, 1, false),
            Err(ExchangeRefusal::InvalidAmount)
        );
        assert_eq!(
            qualify(CoinExchange::PlatinumToGold, u64::MAX, u64::MAX, 0, false),
            Err(ExchangeRefusal::InvalidAmount)
        );
    }
}
