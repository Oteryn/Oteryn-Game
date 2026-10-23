use super::bank::{amount_within, credit, debit, outputs, withdraw_split};
use oteryn_gameplay_domain::domain::currency::Coin;
use oteryn_test_support::property::for_each_case;

#[test]
fn property_bank_balances_reject_overflow_and_underflow() {
    for_each_case("bank balances", |rng, _| {
        let balance = rng.u64();
        let amount = rng.u64();
        let maximum = rng.u64();
        for (balance, amount, maximum) in [
            (balance, amount, maximum),
            (balance, 0, balance),
            (balance, u64::MAX - balance, u64::MAX),
            (balance, u64::MAX - balance, u64::MAX - 1),
            (u64::MAX, 1, u64::MAX),
            (0, 1, maximum),
        ] {
            let sum = u128::from(balance) + u128::from(amount);
            let expected = (sum <= u128::from(maximum)).then_some(sum as u64);
            assert_eq!(credit(balance, amount, maximum), expected);
            let expected = if amount <= balance {
                Some(balance - amount)
            } else {
                None
            };
            assert_eq!(debit(balance, amount), expected);
        }
        Ok(())
    });
}

#[test]
fn property_bank_deposit_withdraw_and_transfer_conserve_gold() {
    for_each_case("bank conservation", |rng, _| {
        let wallet = rng.u64();
        let bank = rng.u64();
        let recipient = rng.u64();
        let amount = rng.u64();
        // The pure operations composed by the transaction owner: neither side is
        // applied unless both plans exist. Compare totals in a wider integer.
        for (from, to) in [(wallet, bank), (bank, wallet), (bank, recipient)] {
            for amount in [0, amount, from.min(u64::MAX - to)] {
                match (debit(from, amount), credit(to, amount, u64::MAX)) {
                    (Some(after_from), Some(after_to)) => {
                        assert_eq!(
                            u128::from(after_from) + u128::from(after_to),
                            u128::from(from) + u128::from(to)
                        );
                        assert_eq!(credit(after_from, amount, u64::MAX), Some(from));
                        assert_eq!(debit(after_to, amount), Some(to));
                    }
                    _ => assert!(
                        amount > from || u128::from(to) + u128::from(amount) > u128::from(u64::MAX)
                    ),
                }
            }
        }
        Ok(())
    });
}

#[test]
fn property_bank_amount_caps_and_coin_outputs_are_exact() {
    for_each_case("bank amounts and coins", |rng, _| {
        let cap = rng.u64();
        for amount in [
            0,
            1,
            99,
            100,
            101,
            9_999,
            10_000,
            10_001,
            rng.below(1_000_000_000),
            u64::from(u32::MAX) * 10_000,
            (u64::from(u32::MAX) + 1) * 10_000,
            cap,
            cap.saturating_add(1),
            rng.u64(),
            u64::MAX,
        ] {
            assert_eq!(amount_within(amount, cap), amount != 0 && amount <= cap);
            let split = withdraw_split(amount);
            let value: u128 = split
                .iter()
                .map(|(coin, count)| {
                    let unit = match coin {
                        Coin::Gold => 1_u128,
                        Coin::Platinum => 100,
                        Coin::Crystal => 10_000,
                    };
                    unit * u128::from(*count)
                })
                .sum();
            assert_eq!(value, u128::from(amount));
            assert!(split[1].1 < 100 && split[2].1 < 100);
            let ordinal = *rng.pick(&[0, 1, u64::MAX - 1, u64::MAX]);
            let positive: Vec<_> = split.into_iter().filter(|(_, count)| *count > 0).collect();
            let fits = positive
                .iter()
                .all(|(_, count)| u32::try_from(*count).is_ok())
                && (positive.is_empty()
                    || (ordinal > 0 && ordinal.checked_add(positive.len() as u64 - 1).is_some()));
            let result = outputs(split, [[1; 16], [2; 16], [3; 16]], ordinal);
            assert_eq!(result.is_some(), fits);
            if let Some(lines) = result {
                assert_eq!(lines.len(), positive.len());
                for (index, (line, (coin, count))) in lines.iter().zip(positive).enumerate() {
                    assert_eq!((line.coin, u64::from(line.quantity_after)), (coin, count));
                    assert_eq!(line.quantity_before, 0);
                    assert_eq!(line.placement_ordinal, ordinal + index as u64);
                }
            }
        }
        Ok(())
    });
}
