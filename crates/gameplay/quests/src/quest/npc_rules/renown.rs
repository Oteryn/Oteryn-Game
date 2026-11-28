//! Gnomally's quote belongs to the current NPC conversation. The fenced
//! inventory planner resolves payment and promotion together on confirmation.
use super::*;
use crate::quest::npc_rules::RenownAction;

const RANK: &str = "oteryn:quest-progress/quest/u9_60/bigfoots_burden/rank";
const LINE: &str = "oteryn:quest-progress/quest/u9_60/bigfoots_burden/quest_line";
const TOKEN: &str = "oteryn:item.tibia.i16128";
const QUOTED: i32 = 1_000;
// A paid NPC hand-in can read up to 500 stacks of 100.
const MAX_TOKENS: i32 = 50_000;
type Response = (Vec<NpcRuleEffect>, Vec<String>);

pub(super) fn resolve(
    action: RenownAction,
    rule: &NpcRule,
    input: &NpcRuleInput<'_>,
    state: &mut Staged<'_>,
    storage_only: bool,
) -> Result<Option<Response>, NpcRuleError> {
    let mut effects = Vec::new();
    let mut reply = rule.reply.clone();
    let quoted = input.topic - QUOTED;
    let topic = match action {
        RenownAction::Relations => {
            let renown = state.track(RANK)?.max(0).to_string();
            for line in &mut reply {
                *line = line.replace("$renown", &renown);
            }
            121
        }
        RenownAction::Ask => 122,
        RenownAction::Quote => {
            let Some(amount) = input
                .message
                .split(|ch: char| !ch.is_ascii_digit())
                .find(|part| !part.is_empty())
                .and_then(|digits| digits.parse::<i32>().ok())
            else {
                return Ok(None);
            };
            if !(1..=MAX_TOKENS).contains(&amount) {
                return Ok(None);
            }
            for line in &mut reply {
                *line = line
                    .replace("$tokens", &amount.to_string())
                    .replace("$renown", &(amount * 5).to_string());
            }
            QUOTED + amount
        }
        RenownAction::Confirm if (1..=MAX_TOKENS).contains(&quoted) => {
            if storage_only {
                return Err(NpcRuleError::MissingNativeResolution(TOKEN.into()));
            }
            let amount = quoted as u32;
            if item_count(&state.inventory, TOKEN, None)? < u64::from(amount) {
                reply.clone_from(&rule.inventory_failure_reply);
            } else {
                let next = state.track(RANK)?.max(0) + i64::from(amount) * 5;
                effects.push(NpcRuleEffect::TakeItem {
                    item: TOKEN.into(),
                    count: amount,
                    subtype: None,
                    optional: false,
                });
                effects.push(NpcRuleEffect::Set {
                    track: RANK.into(),
                    value: next,
                });
                let promoted = match next {
                    1440.. => 30,
                    480.. => 28,
                    120.. => 27,
                    30.. => 26,
                    _ => 25,
                };
                if promoted > state.track(LINE)? {
                    effects.push(NpcRuleEffect::Set {
                        track: LINE.into(),
                        value: promoted,
                    });
                    effects.push(NpcRuleEffect::MagicEffect {
                        effect: "CONST_ME_MAGIC_BLUE".into(),
                    });
                    for (threshold, name) in [
                        (26, "Gnome Little Helper"),
                        (27, "Gnome Friend"),
                        (28, "Gnomelike"),
                        (30, "Honorary Gnome"),
                    ] {
                        if promoted >= threshold {
                            effects.push(NpcRuleEffect::Achievement { name: name.into() });
                        }
                    }
                }
                for line in &mut reply {
                    *line = line.replace("$renown", &next.to_string());
                }
            }
            0
        }
        RenownAction::Cancel if (1..=MAX_TOKENS).contains(&quoted) => 0,
        _ => return Ok(None),
    };
    effects.push(NpcRuleEffect::SetTopic { value: topic });
    Ok(Some((effects, reply)))
}
