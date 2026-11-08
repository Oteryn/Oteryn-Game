use super::party_import::{self, Summary};
use oteryn_client_runtime::analyser::party::{MAX_MEMBERS, Member};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
fn rows() -> Vec<Member> {
    vec![
        Member {
            handle: 9,
            name: "Alice".into(),
            available: true,
            loot_value: 1000,
            supply_cost: 100,
            ..Default::default()
        },
        Member {
            handle: 7,
            name: "Bob".into(),
            available: true,
            loot_value: 0,
            supply_cost: 300,
            ..Default::default()
        },
    ]
}
#[test]
fn party_summary_roundtrips_real_values_and_reuses_conserving_splits() -> Result {
    let source = rows();
    let text = party_import::export_rows(&source)?;
    let imported = Summary::parse(&text)?;
    assert_eq!(imported.export()?, text);
    let report = imported.text();
    assert!(report.contains("advisory"));
    assert!(report.contains("Alice pays Bob: 600 gold"));
    let reversed = party_import::export_rows(&[source[1].clone(), source[0].clone()])?;
    assert!(
        Summary::parse(&reversed)?
            .text()
            .contains("Alice pays Bob: 600 gold")
    );
    let loss = Summary::parse(
        "OTERYN-PARTY-SUMMARY\t1\nName\tLoot\tSupplies\nAlice\t40\t61\nBob\t0\t2\n",
    )?;
    assert!(loss.text().contains("Bob pays Alice: 10 gold"));
    Ok(())
}
#[test]
fn party_summary_rejects_ambiguous_malformed_unbounded_and_incomplete_inputs() -> Result {
    let good = party_import::export_rows(&rows())?;
    for bad in [
        good.replace("Bob", "alice"),
        good.replace("Alice", " Alice"),
        good.replace("1000", "-1"),
        good.replace("1000", "18446744073709551616"),
        good.replace("1000", "1.5"),
        good.replace("Alice", "A\0lice"),
        good.replace("Alice", &"A".repeat(65)),
        good.replace("\t1\n", "\t2\n"),
        "OTERYN-PARTY-SUMMARY\t1\nName\tLoot\tSupplies\n".into(),
        "x".repeat(party_import::MAX_BYTES + 1),
    ] {
        assert!(Summary::parse(&bad).is_err());
    }
    let mut incomplete = rows();
    incomplete[0].incomplete = true;
    assert!(party_import::export_rows(&incomplete).is_err());
    incomplete[0].incomplete = false;
    incomplete[0].unpriced_supplies = 1;
    assert!(party_import::export_rows(&incomplete).is_err());
    let mut many = String::from("OTERYN-PARTY-SUMMARY\t1\nName\tLoot\tSupplies\n");
    for i in 0..MAX_MEMBERS {
        many.push_str(&format!("Member{i}\t{}\t{}\n", u64::MAX, u64::MAX));
    }
    assert!(Summary::parse(&many)?.text().contains("Already balanced"));
    many.push_str("Extra\t0\t0\n");
    assert!(Summary::parse(&many).is_err());
    Ok(())
}
