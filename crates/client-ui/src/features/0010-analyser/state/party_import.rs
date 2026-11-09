//! Bounded, explicitly advisory session imports reuse the live party settlement algorithm.
use oteryn_client_runtime::analyser::party::{self as wire, Member};
use std::{collections::BTreeSet, fmt::Write};
pub const MAX_BYTES: usize = 8192;
const HEADER: &str = "OTERYN-PARTY-SUMMARY\t1";
const COLUMNS: &str = "Name\tLoot\tSupplies";
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    rows: Vec<Member>,
}
impl Summary {
    pub fn parse(text: &str) -> Result<Self, &'static str> {
        if text.len() > MAX_BYTES {
            return Err(tr!("Party summary exceeds the 8 KiB limit"));
        }
        let mut lines = text.lines();
        if lines.next() != Some(HEADER) || lines.next() != Some(COLUMNS) {
            return Err(tr!(
                "Choose an Oteryn party summary exported by the analyser"
            ));
        }
        let mut rows = Vec::new();
        let mut names = BTreeSet::new();
        for line in lines {
            if line.is_empty() {
                continue;
            }
            if rows.len() == wire::MAX_MEMBERS {
                return Err(tr!("Party summary supports at most 50 members"));
            }
            let columns: Vec<_> = line.split('\t').collect();
            let [name, loot, cost] = columns.as_slice() else {
                return Err(tr!("Each member needs a name, loot value and supply cost"));
            };
            if name.trim() != *name || !names.insert(name.to_lowercase()) {
                return Err(tr!(
                    "Member names must be distinct and have no surrounding whitespace"
                ));
            }
            rows.push(Member {
                handle: u64::try_from(rows.len() + 1).map_err(|_| tr!("Member limit"))?,
                name: (*name).into(),
                available: true,
                loot_value: loot
                    .parse()
                    .map_err(|_| tr!("Loot values must be whole nonnegative gold amounts"))?,
                supply_cost: cost
                    .parse()
                    .map_err(|_| tr!("Supply costs must be whole nonnegative gold amounts"))?,
                ..Default::default()
            });
        }
        if rows.is_empty() {
            return Err(tr!("Party summary has no members"));
        }
        wire::encode_reply(&rows).map_err(|_| tr!("Invalid member name or values"))?;
        Ok(Self { rows })
    }
    pub fn export(&self) -> Result<String, &'static str> {
        export_rows(&self.rows)
    }
    pub fn text(&self) -> String {
        let mut text = String::from(tr!(
            "Imported party session · advisory\r\nVerify the supplied names and values before using these suggestions.\r\nName | loot value | supply cost | balance\r\n"
        ));
        for row in &self.rows {
            let _ = write!(
                text,
                "{} | {} | {} | {}\r\n",
                row.name,
                row.loot_value,
                row.supply_cost,
                i128::from(row.loot_value) - i128::from(row.supply_cost)
            );
        }
        if let Some(transfers) = super::party::split(&self.rows) {
            text.push_str(tr!(
                "Equal balance split · suggestions only; no currency is moved\r\n"
            ));
            if transfers.is_empty() {
                text.push_str(tr!("Already balanced\r\n"));
            }
            for transfer in transfers {
                let name = |handle| {
                    self.rows
                        .iter()
                        .find(|row| row.handle == handle)
                        .map_or("?", |row| row.name.as_str())
                };
                let _ = lwrite!(
                    text,
                    "{} pays {}: {} gold\r\n",
                    name(transfer.from),
                    name(transfer.to),
                    transfer.gold
                );
            }
        }
        text.push_str(tr!(
            "Refresh party hunt returns to the current server party.\r\n"
        ));
        text
    }
}
pub fn export_rows(rows: &[Member]) -> Result<String, &'static str> {
    wire::encode_reply(rows).map_err(|_| tr!("Invalid party summary"))?;
    if super::party::split(rows).is_none() {
        return Err(tr!(
            "Every member needs complete, priced measurements before export"
        ));
    }
    let mut names = BTreeSet::new();
    let mut text = format!("{HEADER}\n{COLUMNS}\n");
    for row in rows {
        if row.name.trim() != row.name || !names.insert(row.name.to_lowercase()) {
            return Err(tr!(
                "Member names must be distinct and have no surrounding whitespace"
            ));
        }
        let _ = writeln!(
            text,
            "{}\t{}\t{}",
            row.name, row.loot_value, row.supply_cost
        );
    }
    if text.len() > MAX_BYTES {
        return Err(tr!("Party summary exceeds the 8 KiB limit"));
    }
    Ok(text)
}
