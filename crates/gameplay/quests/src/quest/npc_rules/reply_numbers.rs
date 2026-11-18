//! Numeric dialogue values use the same staged tracks as the NPC writer.
//! Crystal npc/sandomo.lua:105-107,119-129,135-165.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{ContributionTerm, NpcRule, NpcRuleError, contributions};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NpcReplyNumber {
    NonnegativeStorage { track: String },
    ContributionTotal { terms: Vec<ContributionTerm> },
    RedemptionLimit { track: String, unit_cost: u32 },
}

pub(super) fn valid(rule: &NpcRule, owns_track: impl Fn(&String) -> bool) -> bool {
    let numbers = &rule.reply_numbers;
    if numbers.is_empty() {
        return true;
    }
    if numbers.len() > 8
        || numbers.iter().any(|(name, value)| {
            name.is_empty()
                || name.len() > 32
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
                || match value {
                    NpcReplyNumber::NonnegativeStorage { track } => !owns_track(track),
                    NpcReplyNumber::ContributionTotal { terms } => {
                        !contributions::valid_terms(terms, |track| owns_track(track))
                    }
                    NpcReplyNumber::RedemptionLimit { track, unit_cost } => {
                        *unit_cost == 0 || !owns_track(track)
                    }
                }
        })
    {
        return false;
    }
    let widest = numbers
        .keys()
        .map(|name| (name.as_str(), i64::MAX.to_string()))
        .collect();
    rule.reply.iter().all(|line| expand(line, &widest).is_ok())
        && numbers.keys().all(|name| {
            let token = format!("{{{{{name}}}}}");
            rule.reply.iter().any(|line| line.contains(&token))
        })
}

pub(super) fn append(
    rule: &NpcRule,
    mut track: impl FnMut(&str) -> Result<i64, NpcRuleError>,
    reply: &mut Vec<String>,
) -> Result<(), NpcRuleError> {
    if rule.reply_numbers.is_empty() {
        reply.extend_from_slice(&rule.reply);
        return Ok(());
    }
    let mut values = BTreeMap::new();
    for (name, value) in &rule.reply_numbers {
        let number = match value {
            NpcReplyNumber::NonnegativeStorage { track: key } => track(key)?.max(0),
            NpcReplyNumber::ContributionTotal { terms } => contributions::total(terms, &mut track)?,
            NpcReplyNumber::RedemptionLimit {
                track: key,
                unit_cost,
            } => {
                let affordable = track(key)?.max(0) / i64::from(*unit_cost);
                affordable.min(i64::from(contributions::MAX_REDEMPTION_QUANTITY))
            }
        };
        values.insert(name.as_str(), number.to_string());
    }
    for line in &rule.reply {
        reply.push(expand(line, &values)?);
    }
    Ok(())
}

fn expand(line: &str, values: &BTreeMap<&str, String>) -> Result<String, NpcRuleError> {
    let mut rendered = String::with_capacity(line.len());
    let mut rest = line;
    while let Some((prefix, token_and_tail)) = rest.split_once("{{") {
        let (name, tail) = token_and_tail
            .split_once("}}")
            .ok_or_else(|| NpcRuleError::Invalid("unclosed numeric NPC reply token".into()))?;
        let value = values.get(name).ok_or_else(|| {
            NpcRuleError::Invalid(format!("unknown numeric NPC reply value: {name}"))
        })?;
        rendered.push_str(prefix);
        rendered.push_str(value);
        rest = tail;
    }
    rendered.push_str(rest);
    if rendered.len() > 2048 {
        return Err(NpcRuleError::LimitExceeded);
    }
    Ok(rendered)
}
