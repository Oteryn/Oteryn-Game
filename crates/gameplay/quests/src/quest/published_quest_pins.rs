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
        hash(b"8cb40f052f6304f54079a7986e0a77c4802b5e3260fe2242e0d2faa5c937c751"),
        hash(b"5054575cd55c76480522bdc31bab711873fc7dd859b3832d4bcacbeeede0259e"),
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
const GAMEPLAY_PREDECESSORS: &[(&str, DefinitionHash, &[DefinitionHash])] = &[
    (
        "oteryn:quest.dangerous_depths_quest",
        hash(b"e177a8f38383a25e9c0fea9ee86bf6831782c7253431a75fa911dc5e2e2d620a"),
        &[hash(
            b"b19e1d3f99ecdaf123501f7ba3a54ca184be811b7e4e6a9f8c6f3307f32a0acb",
        )],
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
        hash(b"7da8382a26e05c98cdf07b52f4ff6961ec70138baf90ffbb98c164d5240fccca"),
        &[
            hash(b"298abe628683287d3aa5939bb4769361cccfeb586946e8956daa886f9b37a51a"),
            hash(b"4c96930f43d73c762b1f0e546c0f71aa11a24a504fbde760e03327b4e0db74ce"),
        ],
    ),
    (
        "oteryn:quest.the_ancient_tombs_quest",
        hash(b"2cecdbb9b63e475fc2111ebedb9046ad4dcfb287d62024a449eee1dc88fca485"),
        &[
            hash(b"4c3fff8d01302e1c6854af72abc7bc65d369e6874890557088a53844da2cbe14"),
            hash(b"8a188b5b0b564e34830783f89b5b1a24cc00eb29a1639495bdfb8a7124b584af"),
        ],
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
    fn reviewed_package_keeps_exact_published_definition_pins() -> Result<(), String> {
        let loaded =
            loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();

        for &(quest, refreshed, published) in CITATION_REFRESHES {
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
