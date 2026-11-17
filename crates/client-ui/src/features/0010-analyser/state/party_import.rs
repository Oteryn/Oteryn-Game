//! Bounded, explicitly advisory session imports reuse the live party settlement algorithm.
use oteryn_client_runtime::analyser::party::{self as wire, Member};
use std::{collections::BTreeSet, fmt::Write};
pub const MAX_BYTES: usize = 8192;
const HEADER: &str = "OTERYN-PARTY-SUMMARY\t1";
// Version 2 preserves advisory edits alongside the original measured values; version 1 remains readable.
const ADJUSTED_HEADER: &str = "OTERYN-PARTY-SUMMARY\t2";
const ADJUSTED_COLUMNS: &str = "Name\tLoot\tSupplies\tExtraCost\tExcluded";
const COLUMNS: &str = "Name\tLoot\tSupplies";
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub(super) rows: Vec<Member>,
    pub(super) adjustments: super::party::Adjustments,
}
impl Summary {
    pub fn parse(text: &str) -> Result<Self, &'static str> {
        if text.len() > MAX_BYTES {
            return Err(tr!("Party summary exceeds the 8 KiB limit"));
        }
        let mut lines = text.lines();
        let header = lines.next();
        let adjusted = header == Some(ADJUSTED_HEADER);
        if (!adjusted && header != Some(HEADER))
            || lines.next() != Some(if adjusted { ADJUSTED_COLUMNS } else { COLUMNS })
        {
            return Err(tr!(
                "Choose an Oteryn party summary exported by the analyser"
            ));
        }
        let mut rows = Vec::new();
        let mut adjustments = super::party::Adjustments::new();
        let mut names = BTreeSet::new();
        for line in lines {
            if line.is_empty() {
                continue;
            }
            if rows.len() == wire::MAX_MEMBERS {
                return Err(tr!("Party summary supports at most 50 members"));
            }
            let columns: Vec<_> = line.split('\t').collect();
            let (name, loot, cost) = match columns.as_slice() {
                [name, loot, cost] if !adjusted => (*name, *loot, *cost),
                [name, loot, cost, extra, excluded] if adjusted => {
                    let extra_cost = extra
                        .parse::<u64>()
                        .map_err(|_| tr!("Extra costs must be whole nonnegative gold amounts"))?;
                    let excluded = match *excluded {
                        "0" => false,
                        "1" => true,
                        _ => return Err(tr!("Invalid party summary")),
                    };
                    adjustments.insert(
                        (*name).to_owned(),
                        super::party::Adjustment {
                            extra_cost,
                            excluded,
                        },
                    );
                    (*name, *loot, *cost)
                }
                _ => return Err(tr!("Each member needs a name, loot value and supply cost")),
            };
            if name.trim() != name || !names.insert(name.to_lowercase()) {
                return Err(tr!(
                    "Member names must be distinct and have no surrounding whitespace"
                ));
            }
            rows.push(Member {
                handle: u64::try_from(rows.len() + 1).map_err(|_| tr!("Member limit"))?,
                name: name.into(),
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
        Ok(Self { rows, adjustments })
    }
    pub fn export(&self) -> Result<String, &'static str> {
        export_adjusted(&self.rows, &self.adjustments)
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
        text.push_str(&super::party::adjustment_text(
            &self.rows,
            &self.adjustments,
        ));
        if let Some(transfers) = super::party::split_adjusted(&self.rows, &self.adjustments) {
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
        } else {
            text.push_str(tr!(
                "Split unavailable until every member has complete, priced measurements.\r\n"
            ));
        }
        text.push_str(tr!(
            "Refresh party hunt returns to the current server party.\r\n"
        ));
        text
    }
}
pub fn export_rows(rows: &[Member]) -> Result<String, &'static str> {
    export_adjusted(rows, &super::party::Adjustments::new())
}
pub fn export_adjusted(
    rows: &[Member],
    adjustments: &super::party::Adjustments,
) -> Result<String, &'static str> {
    wire::encode_reply(rows).map_err(|_| tr!("Invalid party summary"))?;
    if super::party::split(rows).is_none() {
        return Err(tr!(
            "Every member needs complete, priced measurements before export"
        ));
    }
    let mut names = BTreeSet::new();
    let adjusted = rows.iter().any(|row| {
        adjustments
            .get(&row.name)
            .is_some_and(|a| *a != super::party::Adjustment::default())
    });
    let mut text = if adjusted {
        format!("{ADJUSTED_HEADER}\n{ADJUSTED_COLUMNS}\n")
    } else {
        format!("{HEADER}\n{COLUMNS}\n")
    };
    for row in rows {
        if row.name.trim() != row.name || !names.insert(row.name.to_lowercase()) {
            return Err(tr!(
                "Member names must be distinct and have no surrounding whitespace"
            ));
        }
        let _ = write!(
            text,
            "{}\t{}\t{}",
            row.name, row.loot_value, row.supply_cost
        );
        if adjusted {
            let a = adjustments.get(&row.name).copied().unwrap_or_default();
            let _ = write!(text, "\t{}\t{}", a.extra_cost, u8::from(a.excluded));
        }
        text.push('\n');
    }
    if text.len() > MAX_BYTES {
        return Err(tr!("Party summary exceeds the 8 KiB limit"));
    }
    Ok(text)
}
