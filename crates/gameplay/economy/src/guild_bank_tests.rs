#![allow(clippy::unwrap_used)]
use super::*;
use crate::bank::StarterIslandFact::{NoStarterIsland, OnIsland};
#[test]
fn guild_bank_transfers_conserve_and_refuse() {
    for bank in [0, 1, 10, GUILD_BALANCE_MAX, ACCOUNT_HARD_MAX] {
        for guild in [0, 1, 10, GUILD_BALANCE_MAX] {
            for amount in [0, 1, 9, GUILD_BALANCE_MAX, u64::MAX] {
                for direction in [TransferDirection::Deposit, TransferDirection::Withdraw] {
                    let headroom = match direction {
                        TransferDirection::Deposit => GUILD_BALANCE_MAX - guild,
                        TransferDirection::Withdraw => ACCOUNT_HARD_MAX - bank,
                    };
                    if let Ok(plan) =
                        plan_transfer(direction, 1, NoStarterIsland, bank, guild, amount, headroom)
                    {
                        assert_eq!(bank + guild, plan.bank_after + plan.guild_after);
                        assert!(plan.bank_after <= ACCOUNT_HARD_MAX);
                        assert!(plan.guild_after <= GUILD_BALANCE_MAX);
                    }
                }
            }
        }
    }
    assert_eq!(
        plan_transfer(
            TransferDirection::Withdraw,
            3,
            NoStarterIsland,
            0,
            10,
            1,
            10
        ),
        Err(Refusal::NotAllowed)
    );
    assert_eq!(
        plan_transfer(TransferDirection::Deposit, 3, OnIsland, 10, 0, 1, 10),
        Err(Refusal::Junior)
    );
    assert_eq!(
        plan_transfer(TransferDirection::Withdraw, 2, NoStarterIsland, 0, 10, 5, 4),
        Err(Refusal::BalanceLimit)
    );
}
#[test]
fn guild_bank_payout_conserves_at_every_ceiling() {
    for amount in [0, 1, 17, GUILD_BALANCE_MAX] {
        for headroom in [0, 1, 9, GUILD_BALANCE_MAX, ACCOUNT_HARD_MAX] {
            let (credit, claim) =
                split_payout(amount, ACCOUNT_HARD_MAX - headroom, headroom).unwrap();
            assert_eq!(credit + claim, amount);
            assert!(credit <= headroom);
        }
    }
    assert!(split_payout(u64::MAX, 0, 0).is_none());
    assert!(split_payout(1, ACCOUNT_HARD_MAX, 1).is_none());
}

#[test]
fn guild_bank_property_conservation() {
    oteryn_test_support::property::for_each_case("guild bank transfers and custody", |rng, _| {
        let bank = rng.u64() % (ACCOUNT_HARD_MAX + 1);
        let guild = rng.u64() % (GUILD_BALANCE_MAX + 1);
        let amount = 1 + rng.u64() % GUILD_BALANCE_MAX;
        for direction in [TransferDirection::Deposit, TransferDirection::Withdraw] {
            let room = if direction == TransferDirection::Deposit {
                GUILD_BALANCE_MAX - guild
            } else {
                ACCOUNT_HARD_MAX - bank
            };
            if let Ok(plan) =
                plan_transfer(direction, 2, NoStarterIsland, bank, guild, amount, room)
            {
                assert_eq!(bank + guild, plan.bank_after + plan.guild_after);
            }
        }
        let room = rng.u64() % (ACCOUNT_HARD_MAX - bank + 1);
        let (credit, claim) = split_payout(guild, bank, room).unwrap();
        assert_eq!(credit + claim, guild);
        assert!(credit <= room);
        assert!(bank + credit <= ACCOUNT_HARD_MAX);
        Ok(())
    });
}
