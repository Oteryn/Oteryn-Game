//! Ordinary corpse loot reports use the existing server status-text surface.
use super::*;
use crate::content::ProjectV2AuthoringProfileData;
use crate::content::ReferenceItemField;
use crate::content::native_gameplay::NativeGameplayState;
use oteryn_protocol_oteryn::zones_nologout::MAX_SERVER_MOTD_BYTES;
use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Debug)]
pub(in crate::gameplay_transport) struct LootNoticeRecipients {
    pending: Vec<CapturedRewardPrincipal>,
}

impl KillSettlementQueue {
    pub(super) fn capture_loot_notice(
        &self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        facts: ProjectedCreatureDeathFacts,
    ) {
        let principal = facts.principal;
        if facts.top_damage_character != principal.character {
            return;
        }
        let receipts = self.analyser_receipts(runtime, target);
        let Ok(mut receipts) = receipts.lock() else {
            return;
        };
        receipts.loot_notice.get_or_insert_with(|| {
            let mut pending = vec![principal];
            if let Ok(Some(party)) = runtime.cached_world_party(principal.actor, principal.session)
                && party.is_well_formed()
                && party.party_id.is_some()
                && let Ok(census) = runtime.positioned_actor_census()
            {
                pending.extend(census.into_iter().filter_map(|(actor, _, session)| {
                    let session = session?;
                    let lease = runtime.bound_attacker_lease(actor, session).ok()?;
                    let character = lease.character_id();
                    (character != principal.character
                        && party
                            .members
                            .iter()
                            .any(|member| &member.character == character.as_bytes()))
                    .then_some(CapturedRewardPrincipal {
                        actor,
                        session,
                        character,
                        lease_generation: lease.generation(),
                    })
                }));
            }
            LootNoticeRecipients { pending }
        });
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn publish_loot_notice(
        &self,
        runtime: &ChannelRuntimeV1,
        entry: &PendingKillSettlement,
        outcome: &crate::combat::CreatureDeathRewardOutcome,
    ) {
        let Ok(Some(_)) = &outcome.loot else {
            return;
        };
        let Some(native) = self
            .active_generation
            .filter(|pin| {
                pin.identity().server_artifact_digest()
                    == runtime.content_pin().server_artifact_digest()
            })
            .and_then(|pin| pin.native_gameplay())
        else {
            return;
        };
        let Some(creature) = creature_name(native, &entry.creature) else {
            return;
        };
        let descriptions = outcome
            .committed_loot
            .iter()
            .map(|(_, item, count)| item_description(native, item, *count));
        let lines = format_report(creature, descriptions);
        let receipt = self
            .states
            .attack
            .lock()
            .await
            .kills()
            .analyser_receipts(runtime, entry.target);
        if let Ok(mut receipt) = receipt.lock()
            && let Some(notice) = &mut receipt.loot_notice
        {
            notice.pending.retain(|recipient| {
                let current = runtime
                    .bound_attacker_lease(recipient.actor, recipient.session)
                    .is_ok_and(|lease| {
                        lease.character_id() == recipient.character
                            && lease.generation() == recipient.lease_generation
                    });
                current
                    && !self
                        .states
                        .server_messages
                        .send_loot_report(recipient.session, &lines)
            });
        }
    }
}

fn creature_name<'a>(native: &'a NativeGameplayState, key: &str) -> Option<&'a str> {
    let row = native
        .creature_profiles()
        .records
        .iter()
        .find(|row| row.profile.target.key == key)?;
    let ProjectV2AuthoringProfileData::Creature(profile) = &row.profile.data else {
        return None;
    };
    let details = profile.details.as_deref()?;
    Some(if details.inspection.is_empty() {
        &details.display_name
    } else {
        details
            .inspection
            .strip_prefix("You see ")
            .unwrap_or(&details.inspection)
    })
}

struct ItemName {
    name: &'static str,
    article: &'static str,
    plural: &'static str,
}

// Infobox Object facts keyed by the Global numeric Item ID, not by a display
// alias. Unavailable forms are left empty in the Fandom archive extract.
static ITEM_NAMES: LazyLock<HashMap<u32, ItemName>> = LazyLock::new(|| {
    include_str!("loot_item_names.tsv")
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let id = fields.next()?.parse().ok()?;
            Some((
                id,
                ItemName {
                    name: fields.next()?,
                    article: sourced_form(fields.next()?),
                    plural: sourced_form(fields.next()?),
                },
            ))
        })
        .collect()
});

fn sourced_form(form: &str) -> &str {
    if form == "-" { "" } else { form }
}

fn item_description(
    native: &NativeGameplayState,
    item: &crate::combat::LootDefinitionRef,
    count: u32,
) -> String {
    let sourced = item
        .production_key
        .strip_prefix("oteryn:item.tibia.i")
        .and_then(|key| key.parse().ok())
        .and_then(|id| ITEM_NAMES.get(&id));
    if let Some(sourced) = sourced {
        return describe_item(sourced.name, sourced.article, sourced.plural, count);
    }
    let policy = native.item_policy(&item.production_key, &item.revision_ref);
    if let Some(policy) = policy
        && let ReferenceItemField::Known(presentation) = &policy.record().semantics.presentation
        && let ReferenceItemField::Known(name) = &presentation.name
    {
        return describe_item(name, "", "", count);
    }
    describe_item(&item.production_key, "", "", count)
}

fn describe_item(name: &str, article: &str, plural: &str, count: u32) -> String {
    if count > 1 {
        format!("{count} {}", if plural.is_empty() { name } else { plural })
    } else if article.is_empty() {
        name.to_owned()
    } else {
        format!("{article} {name}")
    }
}

fn format_report(creature: &str, descriptions: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut line = format!("Loot of {creature}: ");
    let mut lines = Vec::new();
    let mut items = descriptions.into_iter().peekable();
    if items.peek().is_none() {
        line.push_str("nothing.");
    }
    while let Some(item) = items.next() {
        let suffix = if items.peek().is_some() { "," } else { "." };
        if line.len() + item.len() + suffix.len() > MAX_SERVER_MOTD_BYTES {
            lines.push(std::mem::take(&mut line).trim_end().to_owned());
        }
        line.push_str(&item);
        line.push_str(suffix);
        if items.peek().is_some() {
            line.push(' ');
        }
    }
    lines.push(line);
    // Bounds on the known item names keep each entry within the existing
    // status-text limit. Split a longer admitted fallback at UTF-8 boundaries.
    lines.into_iter().flat_map(split_status_text).collect()
}

fn split_status_text(mut line: String) -> Vec<String> {
    let mut lines = Vec::new();
    while line.len() > MAX_SERVER_MOTD_BYTES {
        let end = line.floor_char_boundary(MAX_SERVER_MOTD_BYTES);
        let tail = line.split_off(end);
        lines.push(std::mem::replace(&mut line, tail));
    }
    lines.push(line);
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loot_report_uses_global_articles_counts_and_nothing() {
        let coins = &ITEM_NAMES[&3031];
        let rapier = &ITEM_NAMES[&3284];
        assert_eq!(
            format_report(
                "a demon",
                [
                    describe_item(coins.name, coins.article, coins.plural, 11),
                    describe_item(rapier.name, rapier.article, rapier.plural, 1),
                ]
            ),
            ["Loot of a demon: 11 gold coins, an ice rapier."]
        );
        assert_eq!(
            format_report("an angry demon", []),
            ["Loot of an angry demon: nothing."]
        );
    }
    #[test]
    fn long_loot_report_fits_existing_utf8_status_frames() {
        let descriptions = vec!["é".repeat(90); 6];
        let expected = format!("Loot of a demon: {}.", descriptions.join(", "));
        let lines = format_report("a demon", descriptions);
        assert!(lines.len() > 1);
        assert!(lines.iter().all(|line| line.len() <= MAX_SERVER_MOTD_BYTES));
        assert_eq!(lines.join(" "), expected);
    }
}
