//! Banker coin-exchange rules. Persistence applies a successful plan as one item transaction.

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
    if available_input < input_count {
        return Err(ExchangeRefusal::InsufficientCoins);
    }
    if !output_can_merge && free_slots_after_input == 0 {
        return Err(ExchangeRefusal::NoRoom);
    }
    let (output_coin, output_per_exchange) = exchange.output();
    let output_count = output_per_exchange
        .checked_mul(output_count)
        .ok_or(ExchangeRefusal::InvalidAmount)?;
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
}
