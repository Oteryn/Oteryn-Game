//! Source registration and checked Sandomo plans for the shared NPC quest writer.
use super::*;
use crate::quest::roshamuul::{Contributions, Registry, Topic};

pub const NPC: &str = "oteryn:npc.sandomo";
pub(in crate::quest) const QUEST: &str = "oteryn:quest.roshamuul_quest";
const RULE: &str = "oteryn:quest-npc/roshamuul_quest/sandomo";
const CLUSTER: &str = "oteryn:item.tibia.i20062";
pub(in crate::quest) const TRACKS: [&str; 4] = [
    "oteryn:quest-progress/roshamuul/frazzlemaws",
    "oteryn:quest-progress/roshamuul/gold-record",
    crate::quest::roshamuul::MORTAR_TRACK,
    "oteryn:quest-progress/roshamuul/silencers",
];

pub(super) fn program() -> Result<NpcRuleProgram, NpcRuleError> {
    let source = format!("{SOURCE_PIN}data-global/npc/sandomo.lua:53-165");
    let document = NpcRuleDocument {
        schema: 1,
        program_key: Some("oteryn:quest-npc-program.sandomo".into()),
        quest: QUEST.into(),
        premium_required: false,
        tracks: TRACKS
            .iter()
            .map(|key| NpcTrackDefinition {
                key: (*key).into(),
                quest: QUEST.into(),
                initial: 0,
                min: 0,
                max: if *key == TRACKS[1] {
                    i64::from(u32::MAX) * 102
                } else {
                    i64::from(u32::MAX)
                },
                sources: vec![source.clone()],
            })
            .collect(),
        rules: vec![NpcRule {
            key: RULE.into(),
            quest: QUEST.into(),
            npc: NPC.into(),
            trigger: NpcRuleTrigger::Say,
            any_message: true,
            keyword_mode: NpcKeywordMode::default(),
            keywords: Vec::new(),
            match_player_name: false,
            exclude_keywords: Vec::new(),
            topics: None,
            guards: Vec::new(),
            reply: Vec::new(),
            effects: Vec::new(),
            continue_matching: false,
            inventory_failure_reply: Vec::new(),
            completes: false,
            sources: vec![source],
        }],
        source_files: vec![NpcSourceFile {
            path: "data-global/npc/sandomo.lua".into(),
            sha256: "b77b196a4ccd988d7443c4103948a006356aca4e1d66fa092f694142ad735a92".into(),
            revision: "201189f871d23597b82caf88fd611454a2ddefb8".into(),
        }],
    };
    let bytes =
        serde_json::to_vec(&document).map_err(|error| NpcRuleError::Json(error.to_string()))?;
    Ok(NpcRuleProgram {
        document,
        digest: Sha256::digest(bytes).into(),
    })
}

impl NpcRuleCatalog {
    pub fn plan_sandomo(
        &self,
        input: &NpcRuleInput<'_>,
        occurrence: &str,
    ) -> Result<Option<NpcRulePlan>, NpcRuleError> {
        let Some(program) = self.program(QUEST).filter(|program| {
            program.document.program_key.as_deref() == Some("oteryn:quest-npc-program.sandomo")
        }) else {
            return Ok(None);
        };
        if input.npc != NPC || input.trigger != NpcRuleTrigger::Say {
            return Ok(None);
        }
        let topic = match input.topic {
            0 => Topic::Idle,
            3 => Topic::Repairs,
            4 => Topic::Books,
            5 => Topic::Confirm,
            6 => Topic::Quantity,
            _ => return Ok(None),
        };
        let before = TRACKS.map(|key| input.tracks.get(key).copied().unwrap_or(0));
        let [frazzlemaws, gold, mortar, silencers] = before;
        let invalid = |_| NpcRuleError::Arithmetic(RULE.into());
        let registry = Registry {
            recent: Contributions {
                frazzlemaws: u32::try_from(frazzlemaws).map_err(invalid)?,
                mortar: u32::try_from(mortar).map_err(invalid)?,
                silencers: u32::try_from(silencers).map_err(invalid)?,
            },
            registered_gold: u64::try_from(gold)
                .map_err(|_| NpcRuleError::Arithmetic(RULE.into()))?,
        };
        let Some(reply) = registry.talk(topic, input.message) else {
            return Ok(None);
        };
        let next_topic = match reply.topic {
            Topic::Idle => 0,
            Topic::Repairs => 3,
            Topic::Books => 4,
            Topic::Confirm => 5,
            Topic::Quantity => 6,
        };
        let mut effects = Vec::new();
        let mut output = reply.registry;
        let mut line = reply.text;
        if reply.clusters > 100 {
            output = registry;
            line = "I can exchange up to 100 clusters at a time.".into();
        } else if reply.clusters > 0 {
            effects.push(NpcRuleEffect::GiveItem {
                item: CLUSTER.into(),
                count: reply.clusters,
                subtype: None,
            });
        }
        let next_topic = if reply.clusters > 100 {
            input.topic
        } else {
            next_topic
        };
        effects.push(NpcRuleEffect::SetTopic { value: next_topic });
        let after = [
            i64::from(output.recent.frazzlemaws),
            i64::try_from(output.registered_gold)
                .map_err(|_| NpcRuleError::Arithmetic(RULE.into()))?,
            i64::from(output.recent.mortar),
            i64::from(output.recent.silencers),
        ];
        let track_changes = TRACKS
            .into_iter()
            .zip(before)
            .zip(after)
            .filter(|((_, before), after)| before != after)
            .map(|((key, before), after)| NpcTrackChange {
                key: key.into(),
                before,
                after,
            })
            .collect();
        Ok(Some(NpcRulePlan {
            quest: QUEST.into(),
            completes: false,
            program_digest: program.digest,
            selected_rule_keys: vec![format!("{RULE}/{occurrence}")],
            reply: vec![line],
            next_topic,
            observed_tracks: TRACKS
                .into_iter()
                .zip(before)
                .map(|(key, value)| (key.into(), value))
                .collect(),
            track_changes,
            effects,
            inventory_before: input.inventory.clone(),
            inventory_after: input.inventory.clone(),
            inventory_failure_reply: vec!["Make room for the clusters and try again.".into()],
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandomo_runtime_admits_source_and_prices_requested_clusters()
    -> Result<(), Box<dyn std::error::Error>> {
        let state = crate::quest::loader::load_embedded_quest_state("content-1")
            .map_err(|error| format!("{error:?}"))?;
        let active = crate::quest::native_kills::active_for(state.catalogue(), &BTreeMap::new());
        assert!(
            active
                .iter()
                .any(|key| key.ends_with("/roshamuul-frazzlemaw"))
        );
        assert!(
            active
                .iter()
                .any(|key| key.ends_with("/roshamuul-silencer"))
        );
        let source = NpcRuleCatalog::runtime()?
            .admitted_for_npc(NPC, state.catalogue())?
            .ok_or("Sandomo source admission")?;
        let tracks = BTreeMap::from([(TRACKS[1].into(), 201)]);
        let input = NpcRuleInput {
            npc: NPC,
            trigger: NpcRuleTrigger::Say,
            message: "2",
            player_name: None,
            topic: 6,
            tracks: &tracks,
            inventory: &NpcInventorySnapshot::default(),
            now_epoch_seconds: 0,
            calendar_month: 1,
            premium_current: false,
            level: 1,
            vocation: "",
            sex: 0,
            outfits: &BTreeMap::new(),
        };
        let quote = source.plan_sandomo(&input, "first")?.ok_or("exchange")?;
        assert_eq!(
            quote.track_changes,
            vec![NpcTrackChange {
                key: TRACKS[1].into(),
                before: 201,
                after: 1,
            }]
        );
        assert!(quote.effects.contains(&NpcRuleEffect::GiveItem {
            item: CLUSTER.into(),
            count: 2,
            subtype: None,
        }));
        let other = source
            .plan_sandomo(&input, "second")?
            .ok_or("second exchange")?;
        assert_ne!(quote.selected_rule_keys, other.selected_rule_keys);
        Ok(())
    }
}
