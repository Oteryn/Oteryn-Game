//! Exact custody for the coherent pinned Corym NPC programme.
use super::NpcRuleDocument;
pub(super) const PROGRAM: &str = "oteryn:quest-npc-program.hidden_threats";
pub(super) const QUEST: &str = "oteryn:quest.tibia_tales";
const REVISION: &str = "04b83b512114bfd888000d6e1433ed8ecaec7c5b";
const PREFIX: &str = "canary:04b83b512114bfd888000d6e1433ed8ecaec7c5b:";
const FILES: &[(&str, &str)] = &[
    (
        "data-otservbr-global/npc/corym_butler.lua",
        "045a7f5ecc773e962492dc1d7c92ab23b28d5cacdd193cc4818514d47cb54b57",
    ),
    (
        "data-otservbr-global/npc/corym_footman.lua",
        "c535be49f54dbb4c64f0de21a2df099443ac671a691757662269f1dcef5f1097",
    ),
    (
        "data-otservbr-global/npc/corym_ratter.lua",
        "6cdadfe5c542508fd7c10bcff7f752fd42528a1b8ab0b852de1ceae7b3869f0a",
    ),
    (
        "data-otservbr-global/npc/corym_servant.lua",
        "c5edcb889d6149169d5e7f03dd27bda47f3df62161ae62a043277aa42784bd31",
    ),
    (
        "data-otservbr-global/npc/corym_slave.lua",
        "b76801379572ecf46561b6352135921731b14a1ac82f9b0d48248b68d509d60b",
    ),
    (
        "data-otservbr-global/npc/corym_worker_01.lua",
        "52fcb188a51ad723746b5f8b35b00b2f2b0b164c8250e96b7fac3a7442ce00ad",
    ),
    (
        "data-otservbr-global/npc/corym_worker_02.lua",
        "fd6292307e436789b2811f52a8aeb332c954ffcbe8a700a4b54f661c1d62b561",
    ),
    (
        "data-otservbr-global/npc/corym_worker_03.lua",
        "e69b29006bfbf2bdc5aca02a1fd6887dbe449fa3bb223413f2045204b9e5f320",
    ),
    (
        "data-otservbr-global/npc/corym_worker_04.lua",
        "210df7efbdffa4935969486b756d8fe99b148e6ca6b719878eeab32a0a69404c",
    ),
    (
        "data-otservbr-global/npc/corym_worker_05.lua",
        "142b4ad3f51e2f0ba0d6a7fcc8903149dcd8bd8ebca53cc8fd5c6be09c9efb9b",
    ),
];

pub(super) fn reference<'a>(quest: &str, source: &'a str) -> Option<&'a str> {
    if quest != QUEST {
        return None;
    }
    let reference = source.strip_prefix(PREFIX)?;
    let (path, _) = reference.rsplit_once(':')?;
    FILES
        .iter()
        .any(|(allowed, _)| *allowed == path)
        .then_some(reference)
}

pub(super) fn file_is_pinned(quest: &str, path: &str, revision: &str, sha: &str) -> bool {
    quest == QUEST && revision == REVISION && FILES.contains(&(path, sha))
}

pub(super) fn custody(document: &NpcRuleDocument) -> bool {
    if document.program_key.as_deref() != Some(PROGRAM) {
        return !document
            .source_files
            .iter()
            .any(|file| file.revision == REVISION)
            && !document
                .tracks
                .iter()
                .flat_map(|track| &track.sources)
                .chain(document.rules.iter().flat_map(|rule| &rule.sources))
                .any(|reference| reference.starts_with(PREFIX));
    }
    document.quest == QUEST
        && document.source_files.len() == FILES.len()
        && FILES.iter().all(|(path, sha)| {
            document
                .source_files
                .iter()
                .filter(|file| {
                    file.path == *path && file.sha256 == *sha && file.revision == REVISION
                })
                .count()
                == 1
        })
}

#[cfg(test)]
mod tests {
    use super::super::*;
    const SOURCE: &str = include_str!(
        "../../../../../../content/dialogues/quest-rules/quest-batch3-c-hidden_threats.json"
    );

    #[test]
    fn corym_programme_requires_every_exact_pinned_source() -> Result<(), Box<dyn std::error::Error>>
    {
        NpcRuleCatalog::from_json_documents(&[SOURCE])?;
        let mut document: serde_json::Value = serde_json::from_str(SOURCE)?;
        document["source_files"][0]["sha256"] = serde_json::json!("0".repeat(64));
        assert!(
            NpcRuleCatalog::from_json_documents(&[&serde_json::to_string(&document)?]).is_err()
        );
        let mut document: serde_json::Value = serde_json::from_str(SOURCE)?;
        document["source_files"]
            .as_array_mut()
            .ok_or("files")?
            .pop();
        assert!(
            NpcRuleCatalog::from_json_documents(&[&serde_json::to_string(&document)?]).is_err()
        );
        Ok(())
    }

    #[test]
    fn corym_start_is_three_atomic_writes_and_never_rewinds_later_progress()
    -> Result<(), NpcRuleError> {
        let catalogue = NpcRuleCatalog::from_json_documents(&[SOURCE])?;
        let inventory = NpcInventorySnapshot::default();
        let outfits = BTreeMap::new();
        let mut tracks = BTreeMap::new();
        for stage in [0, 5] {
            tracks.insert(
                "oteryn:quest-progress/quest/u11_50/hidden_threats/quest_line".into(),
                stage,
            );
            let input = NpcRuleInput {
                npc: "oteryn:npc.corym_ratter",
                trigger: NpcRuleTrigger::Say,
                message: "yes",
                player_name: None,
                topic: 1,
                tracks: &tracks,
                inventory: &inventory,
                now_epoch_seconds: 0,
                calendar_month: 1,
                premium_current: false,
                level: 0,
                vocation: "",
                sex: 0,
                outfits: &outfits,
            };
            let plan = catalogue
                .plan_storage(&input)?
                .ok_or_else(|| NpcRuleError::Invalid("start".into()))?;
            assert_eq!(plan.track_changes.len(), if stage == 0 { 3 } else { 2 });
            assert_eq!(plan.next_topic, 2);
            assert!(
                !plan
                    .track_changes
                    .iter()
                    .any(|change| change.key.ends_with("/quest_line") && change.after < stage)
            );
        }
        Ok(())
    }
}
