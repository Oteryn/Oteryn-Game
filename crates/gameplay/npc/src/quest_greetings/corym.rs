//! Separate closed greeting admission for the pinned Corym programme.
use super::{Document, Greeting, Source};
const REVISION: &str = "04b83b512114bfd888000d6e1433ed8ecaec7c5b";
const HANDLER: &str = "data/npclib/npc_system/npc_handler.lua";
const HANDLER_SHA: &str = "632be3d5b445ec233d80ed2e60c3cd6bc43fec0691330d35aea10f5a21551f55";
const NPCS: &[(&str, &str, &str)] = &[
    (
        "oteryn:npc.corym_butler",
        "data-otservbr-global/npc/corym_butler.lua",
        "045a7f5ecc773e962492dc1d7c92ab23b28d5cacdd193cc4818514d47cb54b57",
    ),
    (
        "oteryn:npc.corym_footman",
        "data-otservbr-global/npc/corym_footman.lua",
        "c535be49f54dbb4c64f0de21a2df099443ac671a691757662269f1dcef5f1097",
    ),
    (
        "oteryn:npc.corym_ratter",
        "data-otservbr-global/npc/corym_ratter.lua",
        "6cdadfe5c542508fd7c10bcff7f752fd42528a1b8ab0b852de1ceae7b3869f0a",
    ),
    (
        "oteryn:npc.corym_servant",
        "data-otservbr-global/npc/corym_servant.lua",
        "c5edcb889d6149169d5e7f03dd27bda47f3df62161ae62a043277aa42784bd31",
    ),
    (
        "oteryn:npc.corym_slave",
        "data-otservbr-global/npc/corym_slave.lua",
        "b76801379572ecf46561b6352135921731b14a1ac82f9b0d48248b68d509d60b",
    ),
    (
        "oteryn:npc.corym_worker_1",
        "data-otservbr-global/npc/corym_worker_01.lua",
        "52fcb188a51ad723746b5f8b35b00b2f2b0b164c8250e96b7fac3a7442ce00ad",
    ),
    (
        "oteryn:npc.corym_worker_2",
        "data-otservbr-global/npc/corym_worker_02.lua",
        "fd6292307e436789b2811f52a8aeb332c954ffcbe8a700a4b54f661c1d62b561",
    ),
    (
        "oteryn:npc.corym_worker_3",
        "data-otservbr-global/npc/corym_worker_03.lua",
        "e69b29006bfbf2bdc5aca02a1fd6887dbe449fa3bb223413f2045204b9e5f320",
    ),
    (
        "oteryn:npc.corym_worker_4",
        "data-otservbr-global/npc/corym_worker_04.lua",
        "210df7efbdffa4935969486b756d8fe99b148e6ca6b719878eeab32a0a69404c",
    ),
    (
        "oteryn:npc.corym_worker_5",
        "data-otservbr-global/npc/corym_worker_05.lua",
        "142b4ad3f51e2f0ba0d6a7fcc8903149dcd8bd8ebca53cc8fd5c6be09c9efb9b",
    ),
];
fn source(source: &Source, path: &str, sha: &str) -> bool {
    source.repository == "opentibiabr/canary"
        && source.revision == REVISION
        && source.path == path
        && source.sha256 == sha
        && source.line > 0
}

pub(super) fn parse(text: &str) -> Option<Vec<Greeting>> {
    let data: Document = serde_json::from_str(text).ok()?;
    if data.schema != 1
        || data.source_revision != REVISION
        || data.npcs.len() != NPCS.len()
        || !source(&data.placeholder_source, HANDLER, HANDLER_SHA)
        || data.placeholder_source.line != 64
    {
        return None;
    }
    let mut seen = std::collections::BTreeSet::new();
    for entry in &data.npcs {
        let (_, path, sha) = NPCS.iter().find(|(key, _, _)| *key == entry.npc)?;
        if !seen.insert(&entry.npc)
            || !entry.variants.is_empty()
            || !source(&entry.greet.source, path, sha)
            || !source(&entry.farewell.source, path, sha)
            || !source(&entry.walkaway.source, HANDLER, HANDLER_SHA)
            || entry.walkaway.source.line != 97
            || [&entry.greet, &entry.farewell, &entry.walkaway]
                .into_iter()
                .any(|message| {
                    message.text.is_empty()
                        || message.text.len() > 1024
                        || message.text.contains(['\0', '\r', '\n'])
                })
        {
            return None;
        }
    }
    Some(data.npcs)
}

#[cfg(test)]
mod tests {
    use super::*;
    const DATA: &str =
        include_str!("../../../../../content/dialogues/quests-2/corym-npc-greetings.json");
    #[test]
    fn corym_greetings_require_the_exact_closed_cohort_and_source_hashes() -> Result<(), String> {
        assert_eq!(parse(DATA).ok_or("source greetings")?.len(), 10);
        for change in 0..3 {
            let mut value: serde_json::Value =
                serde_json::from_str(DATA).map_err(|e| e.to_string())?;
            match change {
                0 => value["npcs"][0]["npc"] = "oteryn:npc.corym_fake".into(),
                1 => value["npcs"][0]["greet"]["source"]["sha256"] = "0".repeat(64).into(),
                _ => {
                    value["npcs"].as_array_mut().ok_or("npcs")?.pop();
                }
            }
            assert!(parse(&value.to_string()).is_none());
        }
        assert!(
            super::super::parse(DATA).is_none(),
            "the original greeting parser retains its source boundary"
        );
        Ok(())
    }
}
