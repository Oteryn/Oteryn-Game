//! Owned-value Guild payouts may exceed the soft deposit cap; ordinary debits still audit.
use super::*;
use oteryn_gameplay_economy::guild_bank::ACCOUNT_HARD_MAX;

#[test]
fn guild_bank_payout_balance_remains_spendable_by_the_bank_audit_codec() {
    let mut id = [1; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    let mut line = BankValueLineV1 {
        entry_id: id.to_vec(),
        asset: ASSET_GOLD.into(),
        account_id: id.to_vec(),
        world_id: id.to_vec(),
        kind: LEDGER_WITHDRAW,
        line_class: LINE_CLASS_CONVERSION,
        amount: 1,
        balance_before: ACCOUNT_HARD_MAX,
        balance_after: ACCOUNT_HARD_MAX - 1,
    };
    assert_eq!(
        check_value_line(&line, &id, LEDGER_WITHDRAW, LINE_CLASS_CONVERSION, 1),
        Ok(())
    );
    line.balance_before += 1;
    line.balance_after += 1;
    assert_eq!(
        check_value_line(&line, &id, LEDGER_WITHDRAW, LINE_CLASS_CONVERSION, 1),
        Err(AuditError::InvalidInput)
    );
}
