//! Exact definition pins captured from published and reviewed quest catalogues.
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
        hash(b"5c6e9bc04b7dc5881f879e073bf3165d452e599d3633b64416cf1f611ac9523d"),
        hash(b"9f3e5b1cb514d0aa4c8b475fd041fc61910791a53da2ce3ebd1cc80c9d31d2ab"),
    ),
    (
        "oteryn:quest.grimvale_quest",
        hash(b"8d5144f88d2889bd95d34648f36383a90b0bda712e4bd2f2b40d0256f0f854cd"),
        hash(b"8485475de7287c1d16beffe6e14612fac04e2c8717c5c25ffb49012ea79d4de8"),
    ),
    (
        "oteryn:quest.grimvale_quest",
        hash(b"ffc0064a21640b08fd5cddd020cac005af21aed2fc98c8a0c76369e3ddbd33bf"),
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
    (
        "oteryn:quest.formogar_mine_hoist",
        hash(b"4dd6fb32f7883de4278a7096fa5a36ae077efa682522ae94abf97d8f02c1242c"),
        hash(b"17d1d5c6239e5a9b23e4fdaa6ee89231368e507ced80608c055b9f66ede55ebb"),
    ),
    (
        "oteryn:quest.giant_smithhammer_quest",
        hash(b"4c294712b959c58cfc957e27be05b89278dedecda374e8f8a242169c81319722"),
        hash(b"50bcf5c02dbbbd3a4ac6326dcd43ea563fa96fc8b1ca5d9274d3aa989a04d215"),
    ),
    (
        "oteryn:quest.goblin_merchant_quest",
        hash(b"681ebe711c3e9c9624881d6ccd9ed49f4045563e6429329a516ebc0cb496f5f8"),
        hash(b"6a7abf586d279cd807c0ebd03967cab2116fc3bb036e72072a9ac1fccf81129b"),
    ),
    (
        "oteryn:quest.grave_danger_quest",
        hash(b"540cd73ad49e78251491ea0266f3930f000b57c26007a5fcb0dcaf2bea4a4113"),
        hash(b"9f3e5b1cb514d0aa4c8b475fd041fc61910791a53da2ce3ebd1cc80c9d31d2ab"),
    ),
    (
        "oteryn:quest.grimvale_quest",
        hash(b"bf28ee0fb34e8cb739fcaa7c0f6717f1eafdb599e54afffc4f94ff04042a2ef5"),
        hash(b"8485475de7287c1d16beffe6e14612fac04e2c8717c5c25ffb49012ea79d4de8"),
    ),
    (
        "oteryn:quest.heart_of_destruction_quest",
        hash(b"ca161893db0debcd005d1359f804106925c73f3e9fe397abb2fb710425dbb157"),
        hash(b"4d4d919cff9c211b7996f838cf149b5ce8ea2437509eb323b65eaf8535d3cb5f"),
    ),
    (
        "oteryn:quest.hero_of_rathleton_quest",
        hash(b"36e6a39b51ab64edf9e285c81cb6248266c201b428f61841c490d136db4464e0"),
        hash(b"a124f146976b28bfedba9bf73d43e425fed9267ba60e7c23a523faba0bd11b38"),
    ),
    (
        "oteryn:quest.hidden_threats_quest",
        hash(b"c00b18f773d09c9c372771d066dc470d85365aa27bfd2a1014583812cbccbd9b"),
        hash(b"0863383d44e7c74131f8d5424156f50df17a5ceb675eeb1be02218b288ebeadc"),
    ),
    (
        "oteryn:quest.hydra_egg_quest",
        hash(b"93f799015394a36b5896f0a862a860befd3f55a871778c5cfa2cbb6aeb1e35a7"),
        hash(b"78c0d4800b0094afbdf9a91ad444c7b8adbad1b2c6c5d8b3b48602837c69ffca"),
    ),
    (
        "oteryn:quest.ferumbras_ascension_quest",
        hash(b"06686721ac1b4e2b9b8055018119ac9ac486bcc30976644dcc2bc1cd021f9811"),
        hash(b"2c5453b476d04277fef5fcb9a0816805c039d3361eea70bf28ffda1846693c8f"),
    ),
    (
        "oteryn:quest.forgotten_knowledge_quest",
        hash(b"2a0e84e7f04fe4eafff4c3824de035250c01f8c9317feb2773ba11391678e00b"),
        hash(b"4c96930f43d73c762b1f0e546c0f71aa11a24a504fbde760e03327b4e0db74ce"),
    ),
];

// These definitions changed gameplay. Preserve their exact saved pins and keep the
// new rules bound to the new hash. Existing absent-track defaults did not change.
const GAMEPLAY_PREDECESSORS: &[(&str, DefinitionHash, &[DefinitionHash])] = &[
    (
        "oteryn:quest.factions",
        hash(b"16a0ebe1810d54cdaa467424e4e3f0ab02fa4a4b8a728d7aea33706c9957d738"),
        &[hash(
            b"2bcb10953ae9889273762889d224e7f8f6d2d91fcb100c6a74a033227134c210",
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
    // Keep the Eclipse and Vampire Hunt saves, including the pre-hand-in dust counter.
    (
        "oteryn:quest.the_inquisition_quest",
        hash(b"8f43a82febdcd9b7fd7c9cd453b53a9b0a6b6de05273abd1d46c2414d562f48d"),
        &[
            hash(b"07b54f2106bcabfc742abf29a2cc09c4b271108a143d46685efae0ee52b151bd"),
            hash(b"50e57d828ac8398e9ba0f966bc1be0777a5640e49efc495b39af1ab16c4db8d4"),
            hash(b"655d26dbf3afa93c5dfc0e195d05b0150e0af298cf74935fa8494b0be895e3d5"),
            hash(b"e34698ec47856e1bf587a56de6c0ac3cdec9119108a4abad91c7a075c4ea73ca"),
            hash(b"895ecb3a0d429bd18ca8be0fc1da84d5c6439cfb6922ae9ecacafb151895ebfe"),
            hash(b"dae45f70cd8e05af9249684351d22d26a6458f29ec3be4f70f3d8645e90dbe52"),
        ],
    ),
    // The Nomad task and the Mina Losa exchange extend the live quest from main at 888210d08.
    (
        "oteryn:quest.the_thieves_guild_quest",
        hash(b"33d25a33aa923a7ef18664d529e7dbfb87bef7a00a32cd696d5ffb4fefc5b616"),
        &[hash(
            b"11c7dc44cf1e00ccd7e4073c3cc933f8de33985b5637959e18fd93f076ab4eac",
        )],
    ),
    // The Ghostlands diary receipt extends existing Catacombs access saves from main at 888210d08.
    (
        "oteryn:quest.the_white_raven_monastery_quest",
        hash(b"8d80e0dd32a81d237326e103df7883482be51fe46297e42fe216861e51d4fd66"),
        &[hash(
            b"670719e1881e54cf19372252c332db10a3143382c420866f586bc16f6e23c53c",
        )],
    ),
    (
        "oteryn:quest.tibia_tales",
        hash(b"8d57876f51450399a161496c4b34d1ab16ccf7f57b3102302774cd5f05585e45"),
        &[hash(
            b"8d75517e41359a25abea653c8fd44fa5b4d2768c1470d9acaa9442b73f543065",
        )],
    ),
    // The native crypt and parcel extensions preserve saves from 3e0991642.
    (
        "oteryn:quest.too_hot_to_handle_quest",
        hash(b"f61af94ff30ae403c700fe861ecc1282bd6eaeb5dd7e0bf9af0e596a0edd295d"),
        &[hash(
            b"c37288dbe48d92cf0b1ba0bb0a108d5690393d564beed3775be9b55583e0f2ae",
        )],
    ),
    (
        "oteryn:quest.tower_defence_quest",
        hash(b"fd8a39997df088fbcbe3dd14aafb92bbd84280756eb3b1cee95c4c9bd04950f8"),
        &[hash(
            b"e83857acbdc70f0e3398417dd50fb1a0cae5d864dd580a6653dea645cd2f2a32",
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
    fn package_accepts_main_grave_danger_pin_as_predecessor() -> Result<(), String> {
        let loaded =
            loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        let grave = "oteryn:quest.grave_danger_quest";
        let main = hash(b"9f3e5b1cb514d0aa4c8b475fd041fc61910791a53da2ce3ebd1cc80c9d31d2ab");
        assert_ne!(catalogue.definition_hash(grave), Some(main));
        assert!(npc_rules::legacy_defaults(catalogue, grave, &main).is_some());
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
            if quest == "oteryn:quest.grave_danger_quest" {
                // Zelos credit changes the live definition. Both exact citation
                // variants still resolve to the published predecessor.
                assert_ne!(catalogue.definition_hash(quest), Some(published));
                assert!(npc_rules::legacy_defaults(catalogue, quest, &published).is_some());
            } else {
                assert_eq!(catalogue.definition_hash(quest), Some(published), "{quest}");
            }
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

    #[test]
    fn native_quest_journeys_can_continue_saves_from_main() -> Result<(), String> {
        let loaded =
            loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        for (quest, saved) in [
            (
                "oteryn:quest.too_hot_to_handle_quest",
                hash(b"c37288dbe48d92cf0b1ba0bb0a108d5690393d564beed3775be9b55583e0f2ae"),
            ),
            (
                "oteryn:quest.tower_defence_quest",
                hash(b"e83857acbdc70f0e3398417dd50fb1a0cae5d864dd580a6653dea645cd2f2a32"),
            ),
            (
                "oteryn:quest.troll_sabotage_quest",
                hash(b"f20fac3b939f0dbc0553079fd2894eb82c2e098e82944370cfecf0c150c49d96"),
            ),
        ] {
            assert_ne!(loaded.catalogue().definition_hash(quest), Some(saved));
            assert!(
                npc_rules::legacy_defaults(loaded.catalogue(), quest, &saved).is_some(),
                "{quest}"
            );
        }
        Ok(())
    }
}
