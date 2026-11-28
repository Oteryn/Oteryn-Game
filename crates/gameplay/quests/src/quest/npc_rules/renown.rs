//! Gnomally's numeric quote is held in her conversation topic until confirmation.
//! The item planner evaluates the same terms against the locked character inventory.
use super::*;
use crate::quest::npc_rules::RenownAction;

const RANK: &str = "oteryn:quest-progress/quest/u9_60/bigfoots_burden/rank";
const LINE: &str = "oteryn:quest-progress/quest/u9_60/bigfoots_burden/quest_line";
const TOKEN: &str = "oteryn:item.tibia.i16128";
const QUOTED: i32 = 1_000;
// The carried-input owner scans at most 500 stacks of 100.
const MAX_TOKENS: i32 = 50_000;
type Response = (Vec<NpcRuleEffect>, Vec<String>);

pub(super) fn resolve(
    action: RenownAction,
    input: &NpcRuleInput<'_>,
    state: &mut Staged<'_>,
    storage_only: bool,
) -> Result<Option<Response>, NpcRuleError> {
    let rank = state.track(RANK)?.max(0);
    let mut effects = Vec::new();
    let quoted = input.topic - QUOTED;
    let (topic, reply) = match action {
        RenownAction::Relations => (121,vec![
            "Our relations improve with every mission you undertake on our behalf. Another way to improve your relations with us gnomes is to trade in minor crystal tokens. ...".into(),
            format!("Your renown amongst us gnomes is currently {{{rank}}}. Do you want to improve your standing by sacrificing tokens? One token will raise your renown by 5 points.")]),
        RenownAction::Ask => (122,vec!["How many tokens do you want to trade?".into()]),
        RenownAction::Quote => {
            let Ok(amount) = input.message.trim().parse::<i32>() else {return Ok(None)};
            if !(1..=MAX_TOKENS).contains(&amount) {return Ok(None)}
            (QUOTED+amount,vec![format!("Do you really want to trade {amount} minor tokens for {} renown?",amount*5)])
        }
        RenownAction::Confirm if (1..=MAX_TOKENS).contains(&quoted) => {
            if storage_only {return Err(NpcRuleError::MissingNativeResolution(TOKEN.into()))}
            let amount = quoted as u32;
            if item_count(&state.inventory,TOKEN,None)? < u64::from(amount) {
                (0,vec!["You don't have these many tokens.".into()])
            } else {
                let next = rank + i64::from(amount)*5;
                effects.push(NpcRuleEffect::TakeItem {item:TOKEN.into(),count:amount,subtype:None,optional:false});
                effects.push(NpcRuleEffect::Set {track:RANK.into(),value:next});
                let line = state.track(LINE)?;
                let promoted = match next {1440.. => 30,480.. => 28,120.. => 27,30.. => 26,_=>25};
                if promoted > line {
                    effects.push(NpcRuleEffect::Set {track:LINE.into(),value:promoted});
                    effects.push(NpcRuleEffect::MagicEffect {effect:"CONST_ME_MAGIC_BLUE".into()});
                    for (threshold,name) in [(26,"Gnome Little Helper"),(27,"Gnome Friend"),(28,"Gnomelike"),(30,"Honorary Gnome")] {
                        if promoted >= threshold {effects.push(NpcRuleEffect::Achievement {name:name.into()});}
                    }
                }
                (0,vec![format!("As you wish! Your new renown is {{{next}}}.")])
            }
        }
        RenownAction::Cancel if matches!(input.topic,121|122) || (1..=MAX_TOKENS).contains(&quoted) => (0,vec!["Maybe another time.".into()]),
        _=>return Ok(None),
    };
    effects.push(NpcRuleEffect::SetTopic { value: topic });
    Ok(Some((effects, reply)))
}
