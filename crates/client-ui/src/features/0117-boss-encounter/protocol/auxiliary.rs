use super::{BossEncounterPanel, Intent};
use oteryn_client_runtime::{boss_encounter as wire, raw_feature::FeatureCommand};
use wire::bosstiary_slots::{BossSlotReply, BossSlotRequest};
pub(super) fn command(panel: &BossEncounterPanel, intent: Intent) -> Option<FeatureCommand> {
    let (kind, payload) = match intent {
        Intent::HazardQuery => (wire::hazard::COMMAND_TYPE_HAZARD, vec![]),
        Intent::HazardSelect | Intent::HazardLower | Intent::HazardRaise => {
            let level = match intent {
                Intent::HazardLower => panel.hazard?.1.checked_sub(1)?,
                Intent::HazardRaise => panel.hazard?.1.checked_add(1)?,
                _ => panel.fields[2].parse::<u8>().ok()?,
            };
            if panel.hazard.is_some_and(|(unlocked, _)| level > unlocked) {
                return None;
            }
            let bytes = vec![level];
            wire::hazard::decode_request(&bytes).ok()?;
            (wire::hazard::COMMAND_TYPE_HAZARD, bytes)
        }
        Intent::SlotsQuery | Intent::SlotEquip | Intent::SlotRemove => {
            let request = match intent {
                Intent::SlotsQuery => BossSlotRequest::Query,
                Intent::SlotEquip => BossSlotRequest::Equip {
                    slot: panel.fields[1].parse().ok()?,
                    boss: panel.fields[0].clone(),
                },
                _ => BossSlotRequest::Remove {
                    slot: panel.fields[1].parse().ok()?,
                },
            };
            (
                wire::bosstiary_slots::COMMAND_TYPE_BOSSTIARY_SLOTS,
                request.encode()?,
            )
        }
        Intent::Rewards | Intent::RewardsNext => {
            let after = if intent == Intent::RewardsNext {
                let chest = panel.rewards.as_ref()?;
                if chest.entries.len() != wire::REWARD_CHEST_PAGE_SIZE {
                    return None;
                }
                Some(chest.entries.last()?.death)
            } else {
                None
            };
            (
                wire::COMMAND_TYPE_BOSS_REWARD_OPEN,
                wire::encode_reward_chest_query(after).ok()?,
            )
        }
        _ => return None,
    };
    Some(FeatureCommand {
        capability: wire::CAPABILITY_BOSS_ENCOUNTER_V1,
        kind,
        payload,
    })
}
pub(super) fn summary(panel: &mut BossEncounterPanel, bytes: &[u8]) -> Option<String> {
    match panel.pending_kind {
        wire::hazard::COMMAND_TYPE_HAZARD => match bytes {
            [unlocked @ 1..=12, selected @ 1..=12] if selected <= unlocked => {
                let draft = panel.fields[2].parse::<u8>().ok();
                if draft.is_none_or(|level| !(1..=*unlocked).contains(&level))
                    || panel
                        .hazard
                        .is_some_and(|(_, previous)| Some(previous) == draft)
                {
                    panel.fields[2] = selected.to_string();
                }
                panel.hazard = Some((*unlocked, *selected));
                Some(lfmt!(
                    "Hazard: {selected} | Unlocked: {unlocked}",
                    selected = selected,
                    unlocked = unlocked
                ))
            }
            _ => None,
        },
        wire::bosstiary_slots::COMMAND_TYPE_BOSSTIARY_SLOTS => {
            let reply = BossSlotReply::decode(bytes)?;
            let status = match reply.status {
                0 => tr!("Updated"),
                1 => tr!("Locked or ineligible"),
                2 => tr!("Occupied or duplicate"),
                3 => tr!("Cooldown"),
                4 => tr!("Insufficient money"),
                _ => tr!("Change cannot fit"),
            };
            let text = lfmt!(
                "Boss slots: {status}\r\n1: {} | 2: {}\r\nUnlocked mask: {} | Bonus: {} | Points: {}\r\nRemoval fee: {} | Ready at: {}",
                reply.bosses[0],
                reply.bosses[1],
                reply.unlocked,
                reply.bonus,
                reply.points,
                reply.remove_fee,
                reply.ready_at_ms,
                status = status
            );
            panel.slots = Some(reply);
            Some(text)
        }
        wire::COMMAND_TYPE_BOSS_REWARD_OPEN => {
            use std::fmt::Write;
            let chest = wire::decode_rewards(bytes)?;
            let mut text = lfmt!(
                "Reward groups: {} | Server time: {}\r\n",
                chest.entries.len(),
                chest.now
            );
            for entry in &chest.entries {
                let death: String = entry.death.iter().map(|b| format!("{b:02x}")).collect();
                let _ = lwriteln!(
                    text,
                    "{} | Death: {death} | Items: {} | Expires: {}",
                    entry.boss_key,
                    entry.item_count,
                    entry.expires_at_unix_ms,
                    death = death
                );
            }
            text.push_str(tr!(
                "Use Boss rewards (Ctrl+Shift+1) with a death id to collect."
            ));
            panel.rewards = Some(chest);
            Some(text)
        }
        _ => None,
    }
}
