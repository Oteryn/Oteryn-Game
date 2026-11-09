#[test]
fn party_split_conserves_positive_negative_and_remainder_balances() -> Result<(), &'static str> {
    use oteryn_client_runtime::analyser::party::Member;
    for values in [
        vec![(100, 10), (0, 20), (5, 0)],
        vec![(0, 100), (0, 11), (0, 0)],
        vec![(0, 0), (0, 0), (1, 0)],
    ] {
        let mut rows: Vec<_> = values
            .iter()
            .enumerate()
            .map(|(i, (loot, cost))| Member {
                handle: i as u64 + 1,
                name: format!("Scout {i}"),
                available: true,
                loot_value: *loot,
                supply_cost: *cost,
                ..Default::default()
            })
            .collect();
        let mut balances: std::collections::BTreeMap<_, _> = rows
            .iter()
            .map(|r| {
                (
                    r.handle,
                    i128::from(r.loot_value) - i128::from(r.supply_cost),
                )
            })
            .collect();
        let original: i128 = balances.values().sum();
        let transfers = super::party::split(&rows).ok_or("complete report")?;
        for transfer in &transfers {
            *balances.get_mut(&transfer.from).ok_or("payer")? -=
                i128::try_from(transfer.gold).map_err(|_| "transfer overflow")?;
            *balances.get_mut(&transfer.to).ok_or("receiver")? +=
                i128::try_from(transfer.gold).map_err(|_| "transfer overflow")?;
        }
        assert_eq!(balances.values().sum::<i128>(), original);
        assert!(balances.values().max().ok_or("max")? - balances.values().min().ok_or("min")? <= 1);
        rows.reverse();
        assert_eq!(super::party::split(&rows), Some(transfers));
        rows[0].unpriced_supplies = 1;
        assert!(super::party::split(&rows).is_none());
        rows[0].unpriced_supplies = 0;
        rows[0].available = false;
        assert!(super::party::split(&rows).is_none());
        rows[0].available = true;
        rows[0].incomplete = true;
        assert!(super::party::split(&rows).is_none());
    }
    Ok(())
}
