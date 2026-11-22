//! Definition pins captured from the published 871c1dec7 catalogue and the reviewed 0f605da3 catalogue.
use super::{NpcDefinitionPredecessor, QuestStateCatalogue};

type DefinitionHash = [u8; 32];

const fn hash(hex: &[u8; 64]) -> DefinitionHash {
    const fn digit(byte: u8) -> u8 {
        if byte.is_ascii_digit() {
            byte - b'0'
        } else {
            byte - b'a' + 10
        }
    }
    let mut output = [0; 32];
    let mut index = 0;
    while index < output.len() {
        output[index] = digit(hex[2 * index]) * 16 + digit(hex[2 * index + 1]);
        index += 1;
    }
    output
}

// A canonical citation can change when a different quest in its definition shard is
// edited. For these exact citation-only refreshes, keep the published save identity.
const CITATION_REFRESHES: &[(&str, DefinitionHash, DefinitionHash)] = &[
    (
        "oteryn:quest.ferumbras_ascension_quest",
        hash(b"0af32c0df483861158003518300f8766f35d117c8838aa44fe12e3f97ae1a705"),
        hash(b"2c5453b476d04277fef5fcb9a0816805c039d3361eea70bf28ffda1846693c8f"),
    ),
    (
        "oteryn:quest.forgotten_knowledge_quest",
        hash(b"538537f1b4c192bca0794d4d8b6f6e20a1957f8b8c3079bda770bd72aa1b9b31"),
        hash(b"4c96930f43d73c762b1f0e546c0f71aa11a24a504fbde760e03327b4e0db74ce"),
    ),
    (
        "oteryn:quest.formogar_mine_hoist",
        hash(b"3e2e5ec6ee0fe7796d1387faf441e8dcaf3620467e93fe7e0d2775e3177e0552"),
        hash(b"17d1d5c6239e5a9b23e4fdaa6ee89231368e507ced80608c055b9f66ede55ebb"),
    ),
    (
        "oteryn:quest.giant_smithhammer_quest",
        hash(b"d4eea61343703bae88dc80c62798886894c463e200e56c78a45ded3b7bd7338e"),
        hash(b"50bcf5c02dbbbd3a4ac6326dcd43ea563fa96fc8b1ca5d9274d3aa989a04d215"),
    ),
    (
        "oteryn:quest.goblin_merchant_quest",
        hash(b"3d8cf6a516500022a4830ebc2a304e6d73d24322267b634820c828f25d362f87"),
        hash(b"6a7abf586d279cd807c0ebd03967cab2116fc3bb036e72072a9ac1fccf81129b"),
    ),
    (
        "oteryn:quest.grave_danger_quest",
        hash(b"3c16ae76d0a5984f9112dc92bfa88bd90187f276478186d547c8b1a7a60acee5"),
        hash(b"9f3e5b1cb514d0aa4c8b475fd041fc61910791a53da2ce3ebd1cc80c9d31d2ab"),
    ),
    (
        "oteryn:quest.grimvale_quest",
        hash(b"18d1b3cbf8791ee126c5a5506e645c05a7ab5e30c3bf8d1adee7c2e995d6e1e8"),
        hash(b"8485475de7287c1d16beffe6e14612fac04e2c8717c5c25ffb49012ea79d4de8"),
    ),
    (
        "oteryn:quest.heart_of_destruction_quest",
        hash(b"15bdfdba656ba4183afb32de71d85196d189651085bdfeced88187f14b9f95b5"),
        hash(b"4d4d919cff9c211b7996f838cf149b5ce8ea2437509eb323b65eaf8535d3cb5f"),
    ),
    (
        "oteryn:quest.hero_of_rathleton_quest",
        hash(b"a933918380eb4c4bd93e06f3c53a43f68fe7257dbcf26608b1bc402f50cb599b"),
        hash(b"a124f146976b28bfedba9bf73d43e425fed9267ba60e7c23a523faba0bd11b38"),
    ),
    (
        "oteryn:quest.hidden_threats_quest",
        hash(b"15ad7a816ba5b4117a2a648458a96889861c3e21508e65fe0965801dcb5a1e60"),
        hash(b"0863383d44e7c74131f8d5424156f50df17a5ceb675eeb1be02218b288ebeadc"),
    ),
    (
        "oteryn:quest.hydra_egg_quest",
        hash(b"4a82936b0cd2def1d5f7f581c173f39ad778f0f83d926f165e0a8e23f7eef8a2"),
        hash(b"78c0d4800b0094afbdf9a91ad444c7b8adbad1b2c6c5d8b3b48602837c69ffca"),
    ),
];

const MAGIC_SWORD: &str = "oteryn:quest.authored.magic_sword_quest";
// Captured by quest_definition_hashes from 2b639c659 before the passage removal.
const MAGIC_SWORD_WITH_PASSAGE: DefinitionHash =
    hash(b"be324e0177ccefdfb5275e84ebebcd7a420128b352aadb233b8a2508a6f72dcf");

// These definitions changed gameplay. Preserve their exact saved pins and keep the
// new rules bound to the new hash. Existing absent-track defaults did not change.
const GAMEPLAY_PREDECESSORS: &[(&str, DefinitionHash, &[DefinitionHash])] = &[
    (
        MAGIC_SWORD,
        hash(b"b4c39f0264af5ac1d290b16d312f3d836f016120b222155c52fecac8d9a3851a"),
        &[MAGIC_SWORD_WITH_PASSAGE],
    ),
    (
        "oteryn:quest.ferumbras_ascension_quest",
        hash(b"2c5453b476d04277fef5fcb9a0816805c039d3361eea70bf28ffda1846693c8f"),
        &[hash(
            b"e86e12f668231e6ff93181caf6bd34a4dde5b7d53b9bacff6bc4a44bd12fb464",
        )],
    ),
    (
        "oteryn:quest.forgotten_knowledge_quest",
        hash(b"4c96930f43d73c762b1f0e546c0f71aa11a24a504fbde760e03327b4e0db74ce"),
        &[hash(
            b"298abe628683287d3aa5939bb4769361cccfeb586946e8956daa886f9b37a51a",
        )],
    ),
    (
        "oteryn:quest.the_ancient_tombs_quest",
        hash(b"5a7765b1239512ce87772a6d2c366b9443cd5596969c7b52ff789d50db0d501a"),
        &[
            hash(b"4c3fff8d01302e1c6854af72abc7bc65d369e6874890557088a53844da2cbe14"),
            hash(b"8a188b5b0b564e34830783f89b5b1a24cc00eb29a1639495bdfb8a7124b584af"),
        ],
    ),
    // Captured by quest_definition_hashes from dc4f1b293 and its content successor.
    (
        "oteryn:quest.mad_mage_room_quest",
        hash(b"6272a77fbda7ea9fe3083f0a08d94c06d4ce8152a80e42ef9a5cc6a42077a5c6"),
        &[hash(
            b"bee52340c68d21b8b93951a9a6944572162e714538ca745766eb88414f5d630e",
        )],
    ),
    (
        "oteryn:quest.the_gravedigger_of_drefia_quest",
        hash(b"35fe5d0a0cff3a0feee0c14d2d1c8a81bc1c86a3f676a450a33903202d0c7586"),
        &[hash(
            b"08c3c13a5037239a0a9d19a9b41f8b4ac6846d4569fe74897edbd10bc36a50d2",
        )],
    ),
    (
        "oteryn:quest.troll_sabotage_quest",
        hash(b"c9006bbddd470649f4f287c4fcc2861ae637cd7306146a7a70101da8d5f7382b"),
        &[hash(
            b"f20fac3b939f0dbc0553079fd2894eb82c2e098e82944370cfecf0c150c49d96",
        )],
    ),
];

pub(super) fn published_hash(quest: &str, current: DefinitionHash) -> DefinitionHash {
    CITATION_REFRESHES
        .iter()
        .find(|(owner, refreshed, _)| *owner == quest && *refreshed == current)
        .map_or(current, |(_, _, published)| *published)
}

pub(super) fn retain_predecessors(catalogue: &mut QuestStateCatalogue) {
    for &(quest, current, previous) in GAMEPLAY_PREDECESSORS {
        if catalogue.definition_hash(quest) == Some(current) {
            let retained = catalogue.predecessors.entry(quest.into()).or_default();
            for &hash in previous {
                if !retained.iter().any(|entry| entry.hash == hash) {
                    retained.push(NpcDefinitionPredecessor {
                        hash,
                        defaults: Default::default(),
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quest::{loader, npc_rules};

    #[test]
    fn magic_sword_loader_retains_the_saved_passage_revision() -> Result<(), String> {
        let loaded =
            loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        assert_ne!(
            catalogue.definition_hash(MAGIC_SWORD),
            Some(MAGIC_SWORD_WITH_PASSAGE)
        );
        assert_eq!(
            npc_rules::legacy_defaults(catalogue, MAGIC_SWORD, &MAGIC_SWORD_WITH_PASSAGE),
            Some(&Default::default())
        );
        Ok(())
    }

    #[test]
    fn package_keeps_main_grave_danger_pin() -> Result<(), String> {
        let loaded =
            loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        let grave = "oteryn:quest.grave_danger_quest";
        assert_eq!(
            catalogue.definition_hash(grave),
            Some(hash(
                b"9f3e5b1cb514d0aa4c8b475fd041fc61910791a53da2ce3ebd1cc80c9d31d2ab"
            ))
        );
        assert!(
            npc_rules::legacy_defaults(
                catalogue,
                grave,
                &hash(b"5054575cd55c76480522bdc31bab711873fc7dd859b3832d4bcacbeeede0259e"),
            )
            .is_some()
        );
        Ok(())
    }

    #[test]
    fn reviewed_package_keeps_exact_published_definition_pins() -> Result<(), String> {
        let loaded =
            loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();

        for &(quest, refreshed, published) in CITATION_REFRESHES {
            assert_eq!(published_hash(quest, refreshed), published, "{quest}");
            assert_eq!(catalogue.definition_hash(quest), Some(published), "{quest}");
            let mut unrelated_revision = refreshed;
            unrelated_revision[0] ^= 1;
            assert_eq!(
                published_hash(quest, unrelated_revision),
                unrelated_revision
            );
        }
        for &(quest, current, previous) in GAMEPLAY_PREDECESSORS {
            assert_eq!(catalogue.definition_hash(quest), Some(current), "{quest}");
            for &pin in previous {
                assert_eq!(
                    npc_rules::legacy_defaults(catalogue, quest, &pin),
                    Some(&Default::default()),
                    "{quest}"
                );
                let mut foreign = pin;
                foreign[0] ^= 1;
                assert!(npc_rules::legacy_defaults(catalogue, quest, &foreign).is_none());
            }
        }
        Ok(())
    }
}
