//! Definition pins captured from the published 871c1dec7 catalogue and the reviewed 0f605da3 catalogue.
use super::{NpcDefinitionPredecessor, QuestStateCatalogue};

type DefinitionHash = [u8; 32];
type PredecessorPin = (&'static str, DefinitionHash, &'static [DefinitionHash]);

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
        "oteryn:quest.formogar_mine_hoist",
        hash(b"617bff2ce025747e4a7747fce2009d039c79c0416d2ba226ed735d0ea5d76ede"),
        hash(b"17d1d5c6239e5a9b23e4fdaa6ee89231368e507ced80608c055b9f66ede55ebb"),
    ),
    (
        "oteryn:quest.giant_smithhammer_quest",
        hash(b"8c6b244adca3ac947bee105824abdc268648729a62add19adc897cc47845b1f1"),
        hash(b"50bcf5c02dbbbd3a4ac6326dcd43ea563fa96fc8b1ca5d9274d3aa989a04d215"),
    ),
    (
        "oteryn:quest.goblin_merchant_quest",
        hash(b"fe96732992ad3b207b75433d64569fffc0792eacc0eb6e2a16e80a636d5046ad"),
        hash(b"6a7abf586d279cd807c0ebd03967cab2116fc3bb036e72072a9ac1fccf81129b"),
    ),
    (
        "oteryn:quest.grave_danger_quest",
        hash(b"5c6e9bc04b7dc5881f879e073bf3165d452e599d3633b64416cf1f611ac9523d"),
        hash(b"9f3e5b1cb514d0aa4c8b475fd041fc61910791a53da2ce3ebd1cc80c9d31d2ab"),
    ),
    (
        "oteryn:quest.grimvale_quest",
        hash(b"be926e9f2acf5908483637cc2f83c79dbeb7b3418c4051cb43a2ffa779233d65"),
        hash(b"8485475de7287c1d16beffe6e14612fac04e2c8717c5c25ffb49012ea79d4de8"),
    ),
    (
        "oteryn:quest.heart_of_destruction_quest",
        hash(b"5fd58496a9bbec4ab0feda817d182f73fc9b562cfeee6c48ccec84df16fe064d"),
        hash(b"4d4d919cff9c211b7996f838cf149b5ce8ea2437509eb323b65eaf8535d3cb5f"),
    ),
    (
        "oteryn:quest.hero_of_rathleton_quest",
        hash(b"45f1e1cfde073c475a485ef92d29e9b6177b6e3fa1de7df67e4e2652aa3c462c"),
        hash(b"a124f146976b28bfedba9bf73d43e425fed9267ba60e7c23a523faba0bd11b38"),
    ),
    (
        "oteryn:quest.hidden_threats_quest",
        hash(b"5710efb6a47e898a65a81660b74ee74e469d1ea3e4ceb14b5e3a0b5029ed4215"),
        hash(b"0863383d44e7c74131f8d5424156f50df17a5ceb675eeb1be02218b288ebeadc"),
    ),
    (
        "oteryn:quest.hydra_egg_quest",
        hash(b"bf21c8ce62fd697df94d9125c787278372f8869088542a1a718ec7ba814f7dad"),
        hash(b"78c0d4800b0094afbdf9a91ad444c7b8adbad1b2c6c5d8b3b48602837c69ffca"),
    ),
];

// These definitions changed gameplay. Preserve their exact saved pins and keep the
// new rules bound to the new hash. Existing absent-track defaults did not change.
const GAMEPLAY_PREDECESSORS: &[PredecessorPin] = &[
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
];

// Captured from 3e099164 and origin/main with quest_definition_hashes.
// The daily bindings add tracks and operations while retaining existing defaults.
const DAILY_PREDECESSORS: &[(&str, DefinitionHash)] = &[
    (
        "oteryn:quest.bigfoot_s_burden_quest",
        hash(b"5ec0c3c6eb10e5876e39fd742b9fa39905980b2d81cea8077116aef91f873258"),
    ),
    (
        "oteryn:quest.dangerous_depths_quest",
        hash(b"b19e1d3f99ecdaf123501f7ba3a54ca184be811b7e4e6a9f8c6f3307f32a0acb"),
    ),
    (
        "oteryn:quest.dangerous_depths_quest",
        hash(b"7fa750757e1538451eef088596b846c0bd4369b78b40a0c57efdbcb047ee6830"),
    ),
    (
        "oteryn:quest.spike_task",
        hash(b"056a2e30ee5ff71b52331a31967a02804df035b2f8b0befe321d8869809829bb"),
    ),
];

pub(super) fn published_hash(quest: &str, current: DefinitionHash) -> DefinitionHash {
    CITATION_REFRESHES
        .iter()
        .find(|(owner, refreshed, _)| *owner == quest && *refreshed == current)
        .map_or(current, |(_, _, published)| *published)
}

pub(super) fn retain_predecessors(catalogue: &mut QuestStateCatalogue) {
    retain(catalogue, GAMEPLAY_PREDECESSORS);
    for &(quest, previous) in DAILY_PREDECESSORS {
        if catalogue
            .definition_hash(quest)
            .is_some_and(|current| current != previous)
        {
            catalogue
                .predecessors
                .entry(quest.into())
                .or_default()
                .push(NpcDefinitionPredecessor {
                    hash: previous,
                    defaults: Default::default(),
                });
        }
    }
}

pub(super) fn retain_hunt_and_wrath_predecessors(catalogue: &mut QuestStateCatalogue) {
    // Captured with quest_definition_hashes on the merge base, origin/main,
    // and the current catalogue. Existing absent-track defaults are unchanged.
    const PINS: &[PredecessorPin] = &[
        (
            "oteryn:quest.killing_in_the_name_of_quest",
            hash(b"dbb0bbd7ead5a280f2085e685637ff0071e3c04013fb937376d89901950e6e61"),
            &[hash(
                b"eec799aef21078c75e2e7ffaabab6e71f8c59f9310d376854f9bbe9ab57ddab5",
            )],
        ),
        (
            "oteryn:quest.wrath_of_the_emperor_quest",
            hash(b"542b939eb16b9ce3692470efa448e7a90e5dfc3e409061960aa894cb90fd3ba6"),
            &[hash(
                b"2956758279011af614d2ef72d62625dfa7532a8788fdc04ac9d9bbbf46c48e86",
            )],
        ),
    ];
    retain(catalogue, PINS);
    retain(catalogue, TOKEN_EXCHANGE_PREDECESSORS);
}

// Generated with quest_definition_hashes on main and the reviewed exchange catalogue.
const TOKEN_EXCHANGE_PREDECESSORS: &[PredecessorPin] = &[
    (
        "oteryn:quest.hunting_for_tokens",
        hash(b"48caf0d645e2b8d2763906efe45fb95363aaa5115f2e8a918f57c05417e7a979"),
        &[hash(
            b"f68593b143c5de45970b3a2375e9264d9781bc7968f59cc67c218b4e66cd05b3",
        )],
    ),
    (
        "oteryn:quest.rift_warrior_outfits_quest",
        hash(b"2bea7fe1165bfca2838ecf0913dee9dbbe253b05e001f367ae19944a020abc49"),
        &[hash(
            b"c78651384e378afe5e740ecccbbd20354a80c4baf1abf6190fae1d21247c1a94",
        )],
    ),
];

fn retain(catalogue: &mut QuestStateCatalogue, pins: &[PredecessorPin]) {
    for &(quest, current, previous) in pins {
        if catalogue.definition_hash(quest) == Some(current) {
            catalogue
                .predecessors
                .entry(quest.into())
                .or_default()
                .extend(previous.iter().map(|&hash| NpcDefinitionPredecessor {
                    hash,
                    defaults: Default::default(),
                }));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quest::{loader, npc_rules};

    #[test]
    fn review_batch_loads_merge_base_quest_saves() -> Result<(), String> {
        let loaded =
            loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        // Captured with quest_definition_hashes on the merge base d481467205
        // and origin/main 896958e567 (identical quest loader inputs).
        let saved = [
            (
                "oteryn:quest.bigfoot_s_burden_quest",
                hash(b"5ec0c3c6eb10e5876e39fd742b9fa39905980b2d81cea8077116aef91f873258"),
            ),
            (
                "oteryn:quest.dangerous_depths_quest",
                hash(b"7fa750757e1538451eef088596b846c0bd4369b78b40a0c57efdbcb047ee6830"),
            ),
            (
                "oteryn:quest.hunting_for_tokens",
                hash(b"f68593b143c5de45970b3a2375e9264d9781bc7968f59cc67c218b4e66cd05b3"),
            ),
            (
                "oteryn:quest.killing_in_the_name_of_quest",
                hash(b"eec799aef21078c75e2e7ffaabab6e71f8c59f9310d376854f9bbe9ab57ddab5"),
            ),
            (
                "oteryn:quest.spike_task",
                hash(b"056a2e30ee5ff71b52331a31967a02804df035b2f8b0befe321d8869809829bb"),
            ),
            (
                "oteryn:quest.wrath_of_the_emperor_quest",
                hash(b"2956758279011af614d2ef72d62625dfa7532a8788fdc04ac9d9bbbf46c48e86"),
            ),
        ];
        let mut refused = Vec::new();
        for (quest, stored_hash) in saved {
            if catalogue.definition_hash(quest) != Some(stored_hash)
                && npc_rules::legacy_defaults(catalogue, quest, &stored_hash).is_none()
            {
                refused.push(quest);
            }
        }
        assert!(refused.is_empty(), "merge-base saves refused: {refused:?}");
        Ok(())
    }

    #[test]
    fn token_exchanges_keep_unfinished_published_saves_readable() -> Result<(), String> {
        let loaded =
            loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        let mut rejected = Vec::new();
        for (quest, published) in [
            (
                "oteryn:quest.hunting_for_tokens",
                hash(b"f68593b143c5de45970b3a2375e9264d9781bc7968f59cc67c218b4e66cd05b3"),
            ),
            (
                "oteryn:quest.rift_warrior_outfits_quest",
                hash(b"c78651384e378afe5e740ecccbbd20354a80c4baf1abf6190fae1d21247c1a94"),
            ),
        ] {
            assert_ne!(catalogue.definition_hash(quest), Some(published));
            if npc_rules::legacy_defaults(catalogue, quest, &published).is_none() {
                rejected.push(quest);
            }
        }
        assert!(rejected.is_empty(), "published saves refused: {rejected:?}");
        Ok(())
    }
    #[test]
    fn hunting_and_wrath_loaders_accept_published_saves() -> Result<(), String> {
        let loaded =
            loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        for (quest, predecessor) in [
            (
                "oteryn:quest.killing_in_the_name_of_quest",
                hash(b"eec799aef21078c75e2e7ffaabab6e71f8c59f9310d376854f9bbe9ab57ddab5"),
            ),
            (
                "oteryn:quest.wrath_of_the_emperor_quest",
                hash(b"2956758279011af614d2ef72d62625dfa7532a8788fdc04ac9d9bbbf46c48e86"),
            ),
        ] {
            assert_ne!(catalogue.definition_hash(quest), Some(predecessor));
            let defaults = npc_rules::legacy_defaults(catalogue, quest, &predecessor)
                .ok_or("published predecessor missing")?;
            assert!(defaults.is_empty());
        }
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

        for &(quest, pin) in DAILY_PREDECESSORS {
            assert_ne!(catalogue.definition_hash(quest), Some(pin), "{quest}");
            assert_eq!(
                npc_rules::legacy_defaults(catalogue, quest, &pin),
                Some(&Default::default()),
                "{quest}"
            );
        }
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
