//! Definition pins from published and reviewed quest catalogues.
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
const CITATION_REFRESHES: &[(&str, &[DefinitionHash], DefinitionHash)] = &[
    (
        "oteryn:quest.ferumbras_ascension_quest",
        &[
            hash(b"046bc3914cc79a5d5f746a6dd56bab3ec6d0c77db6282615992fdb29cb3f3348"),
            hash(b"b78d634978fcca046d0a4a9863448f6bdce8768754d80fa14fd040260a7834dd"),
            hash(b"06686721ac1b4e2b9b8055018119ac9ac486bcc30976644dcc2bc1cd021f9811"),
        ],
        hash(b"2c5453b476d04277fef5fcb9a0816805c039d3361eea70bf28ffda1846693c8f"),
    ),
    (
        "oteryn:quest.forgotten_knowledge_quest",
        &[
            hash(b"b8de92cb0d194f955969ae0e85b6ce47e18849b664742cebd656780cf6afad38"),
            hash(b"1f2491941e888347f73919d391ecfbedbea6f1fb03433ed7e7edb5b008176d23"),
            hash(b"2a0e84e7f04fe4eafff4c3824de035250c01f8c9317feb2773ba11391678e00b"),
        ],
        hash(b"4c96930f43d73c762b1f0e546c0f71aa11a24a504fbde760e03327b4e0db74ce"),
    ),
    (
        "oteryn:quest.formogar_mine_hoist",
        &[
            hash(b"d91460774be98a4b174ae1c83d7d2c7e7efcf5fc5ac115f7cd60ae12094840fa"),
            hash(b"617bff2ce025747e4a7747fce2009d039c79c0416d2ba226ed735d0ea5d76ede"),
            hash(b"8e6dd324609ad288a973fc96293b0d8230bb51f8093f6e676199e7c173b6a191"),
            hash(b"4dd6fb32f7883de4278a7096fa5a36ae077efa682522ae94abf97d8f02c1242c"),
        ],
        hash(b"17d1d5c6239e5a9b23e4fdaa6ee89231368e507ced80608c055b9f66ede55ebb"),
    ),
    (
        "oteryn:quest.giant_smithhammer_quest",
        &[
            hash(b"4db65fa625c6c170dd62e21fac749c3052ff2d98c9ef6257ea64693373647b04"),
            hash(b"8c6b244adca3ac947bee105824abdc268648729a62add19adc897cc47845b1f1"),
            hash(b"ae411e4ba985d12638e50fc7497cca6ca52ce87e02e2cb7b1382b9097a89ea5b"),
            hash(b"4c294712b959c58cfc957e27be05b89278dedecda374e8f8a242169c81319722"),
        ],
        hash(b"50bcf5c02dbbbd3a4ac6326dcd43ea563fa96fc8b1ca5d9274d3aa989a04d215"),
    ),
    (
        "oteryn:quest.goblin_merchant_quest",
        &[
            hash(b"30c5f41357098fd2eda299fc3e35bb5f1d69e8469872f1d3368fc4d7792041da"),
            hash(b"fe96732992ad3b207b75433d64569fffc0792eacc0eb6e2a16e80a636d5046ad"),
            hash(b"c47908e169bd3d739136df750c3d5d9a227e73e579457abeb054329c0c3d3b5b"),
            hash(b"681ebe711c3e9c9624881d6ccd9ed49f4045563e6429329a516ebc0cb496f5f8"),
        ],
        hash(b"6a7abf586d279cd807c0ebd03967cab2116fc3bb036e72072a9ac1fccf81129b"),
    ),
    (
        "oteryn:quest.grave_danger_quest",
        &[
            hash(b"de3fd7c3b92165f4cc71c57effa8beaa5a90aa8a305d365f22fc51a82baad1e5"),
            hash(b"5c6e9bc04b7dc5881f879e073bf3165d452e599d3633b64416cf1f611ac9523d"),
            hash(b"5e996375f282ec9d2d3b9d0fb17f46e3b19e8ae4443f6bf336299dedb010ad7d"),
            hash(b"540cd73ad49e78251491ea0266f3930f000b57c26007a5fcb0dcaf2bea4a4113"),
        ],
        hash(b"9f3e5b1cb514d0aa4c8b475fd041fc61910791a53da2ce3ebd1cc80c9d31d2ab"),
    ),
    (
        "oteryn:quest.grimvale_quest",
        &[
            hash(b"845f231a52f3b61457c2bfd659756cc076c73948215b0c1e4e5c686873dad697"),
            hash(b"be926e9f2acf5908483637cc2f83c79dbeb7b3418c4051cb43a2ffa779233d65"),
            hash(b"efe0803a67f5d52dd56ad28b9cece21419f5b7109b0f57752ebfc6eb2f978d17"),
            hash(b"bf28ee0fb34e8cb739fcaa7c0f6717f1eafdb599e54afffc4f94ff04042a2ef5"),
        ],
        hash(b"8485475de7287c1d16beffe6e14612fac04e2c8717c5c25ffb49012ea79d4de8"),
    ),
    (
        "oteryn:quest.heart_of_destruction_quest",
        &[
            hash(b"24fc148f94a14a65c654cb3c583396ecd78928dfe925a3e02d96d4b939f3f0bb"),
            hash(b"5fd58496a9bbec4ab0feda817d182f73fc9b562cfeee6c48ccec84df16fe064d"),
            hash(b"d2b693bda2cb942c371056f6267c980bedff85882b81dbb1e3661732ba4b6e5d"),
            hash(b"ca161893db0debcd005d1359f804106925c73f3e9fe397abb2fb710425dbb157"),
        ],
        hash(b"4d4d919cff9c211b7996f838cf149b5ce8ea2437509eb323b65eaf8535d3cb5f"),
    ),
    (
        "oteryn:quest.hero_of_rathleton_quest",
        &[
            hash(b"7b5d449d50f32ca6d1a72ff58aa3c557f882358d32cc42259ea8614f7934bd05"),
            hash(b"45f1e1cfde073c475a485ef92d29e9b6177b6e3fa1de7df67e4e2652aa3c462c"),
            hash(b"59828756c8cdfdfcef9853acde04b5b420d372f1ca7b021213c2423ffa1469dd"),
            hash(b"36e6a39b51ab64edf9e285c81cb6248266c201b428f61841c490d136db4464e0"),
        ],
        hash(b"a124f146976b28bfedba9bf73d43e425fed9267ba60e7c23a523faba0bd11b38"),
    ),
    (
        "oteryn:quest.hidden_threats_quest",
        &[
            hash(b"b9fd699bb82723a99a039107862d2920348cc47e38ab1dd99d8df5b3769965d9"),
            hash(b"5710efb6a47e898a65a81660b74ee74e469d1ea3e4ceb14b5e3a0b5029ed4215"),
            hash(b"8cbf1b4b88b94598939d9f352e1a4a69a2ba5ff13d0f5486d1f59eff8ccddc2d"),
            hash(b"c00b18f773d09c9c372771d066dc470d85365aa27bfd2a1014583812cbccbd9b"),
        ],
        hash(b"0863383d44e7c74131f8d5424156f50df17a5ceb675eeb1be02218b288ebeadc"),
    ),
    (
        "oteryn:quest.hydra_egg_quest",
        &[
            hash(b"165adeeae7115720f419bc2aa99d6a942b24dd8b5a11af44016929c566335ada"),
            hash(b"bf21c8ce62fd697df94d9125c787278372f8869088542a1a718ec7ba814f7dad"),
            hash(b"afabcb21148c24b6a84813da6046861ff35ea512193f4dd2e775234193336049"),
            hash(b"93f799015394a36b5896f0a862a860befd3f55a871778c5cfa2cbb6aeb1e35a7"),
        ],
        hash(b"78c0d4800b0094afbdf9a91ad444c7b8adbad1b2c6c5d8b3b48602837c69ffca"),
    ),
    (
        "oteryn:quest.the_ice_islands_quest",
        &[hash(
            b"2466334ebc8e1fe1c21a06e2eaec92d0048920c28ed8cf1abf27ba2a0038167d",
        )],
        hash(b"b31bab1af5526262b5b0590cf13b3b99947d18fe0d40ff1fbbbf24189199fdea"),
    ),
];

// These definitions changed gameplay. Preserve their exact saved pins and keep the
// new rules bound to the new hash. Existing absent-track defaults did not change.
const GAMEPLAY_PREDECESSORS: &[(&str, DefinitionHash, &[DefinitionHash])] = &[
    (
        "oteryn:quest.an_uneasy_alliance_quest",
        hash(b"f070da81828f66fd12f5e724544bb8bed5d956eadb6f818154ad3780e74c6839"),
        &[hash(
            b"5bd6915a3824b432e3e312bbdca7d301065770ee64cda78a1d1bc925b88732f5",
        )],
    ),
    (
        "oteryn:quest.authored.asura_palace_quest",
        hash(b"cfee58077cc792a8e8360d96a51c5fae025a539009d3f82689121dfc20290ede"),
        &[
            hash(b"a807363e56c5966b6e3bba458b4e29151bbf31230de20d129e0864e13c795a53"),
            hash(b"f46458b071254e14908bea9b5f40d7924f6a3ca818e27c8961e01b35632d0518"),
            hash(b"7d2ff46451588dd3ff2352c5db743480e1511730cce8c4ffcdf10f716a1d7caa"),
        ],
    ),
    (
        "oteryn:quest.chayenne_realm",
        hash(b"cf99ff0b1b68f740343bc53faeff3928a1dd510b10b69f28a17c58c26f669a31"),
        &[hash(
            b"200ff087735610eda7ae1d765c08491c3508fbff97967865b6cba1a1045b9d10",
        )],
    ),
    (
        "oteryn:quest.authored.braindeath_quest",
        hash(b"8d8ea321374b01ddba4e073076959274c1d7d5bf03bb32d73240a50ec871c227"),
        &[hash(
            b"4e9b33f69585e0ce52302cfb0f018fd66c2864a6aec3259b2fc80757cf818940",
        )],
    ),
    (
        "oteryn:quest.authored.citizen_of_issavi_outfits_quest",
        hash(b"7a5d8ac354c809d1406bd86f2bb2738764c9d0643a19adfb0cde06f1da329783"),
        &[hash(
            b"312cde5287a319cb4039095c61013abaadfa04c2fc34016c634baa4270160222",
        )],
    ),
    (
        "oteryn:quest.barbarian_test",
        hash(b"18aa3b81e3b3462295a80b435696a7e9e66a739b0c26c199e6c0f1abbe6f89cb"),
        &[hash(
            b"036f08fefe3ff5f327c3801288be0dfae04987f4badfd00678a1db653490797b",
        )],
    ),
    (
        "oteryn:quest.battle_mage_outfits_quest",
        hash(b"a19162baec7e41dd0e4c5b3332819dce4dd6b8f4a2bb300cd5f14ab58c4aff90"),
        &[hash(
            b"5ad0bfbad1d580750ff170af788693571ddf9dcd7cd544a4a001c315e74f38b9",
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
    (
        "oteryn:quest.the_ice_islands_quest",
        hash(b"b31bab1af5526262b5b0590cf13b3b99947d18fe0d40ff1fbbbf24189199fdea"),
        &[
            hash(b"825a74759b817bb6d3bf39bfb339748ef62a5e30d96d5d0e2b93022734a71a6f"),
            hash(b"299b92c459921a7f30b4128518dc869fc7e7f6de3d50dbcf3be09ee05dcb44c0"),
        ],
    ),
    (
        "oteryn:quest.the_secret_library_quest",
        hash(b"22db0b0b39a6a299def5ea0466fd0def68f98d9ccba6e209ae2108ecefec0a6b"),
        &[hash(
            b"8d54b990bfdaf4e00123a18a7132517508dd920b997598f996419bbbb25ca008",
        )],
    ),
];

pub(super) const DRAGON_CORPSE_PREDECESSOR: (&str, DefinitionHash) = (
    "oteryn:quest.dragon_corpse_quest",
    hash(b"ba645250c7b9827e710e57059c40e56ba56c1950c76433932125927c670b2f06"),
);

// Main at 3e09916423 before the donor routes attached new tracks and transitions.
const SOURCE_ROUTE_PREDECESSORS: &[(&str, DefinitionHash)] = &[
    (
        "oteryn:quest.captain_iglues_treasure_quest",
        hash(b"fb2d072d598f8543e7007e182645ef920900cb1b413e99aa2f114bf43a300477"),
    ),
    (
        "oteryn:quest.cave_explorer_outfits_quest",
        hash(b"9f18e63d91bb37882efd544b61e2d14dde49dc6b04422a006049aba3dc3f3c6c"),
    ),
    (
        "oteryn:quest.chayenne_realm",
        hash(b"245b1686c81a2abef73c1525e7ed55e6fc628ea056d9da81963b962f59f69a08"),
    ),
    (
        "oteryn:quest.children_of_the_revolution_quest",
        hash(b"e91b65cb6e0a69e9b4473bb83db970a115bd631acbfae34b7a7e5f8d740e52c2"),
    ),
    (
        "oteryn:quest.crystal_wand_quest",
        hash(b"84012a12a60d5371070c5fea63caf66af7b05807b9f3d36a6c8d77a7cfeea557"),
    ),
    DRAGON_CORPSE_PREDECESSOR,
    (
        "oteryn:quest.spike_task",
        hash(b"056a2e30ee5ff71b52331a31967a02804df035b2f8b0befe321d8869809829bb"),
    ),
    (
        "oteryn:quest.spike_task",
        hash(b"3f03edd9b13bd48938ee76a9644e0c1f0acbe2528199ca6d0f3811202177ac7f"),
    ),
];

pub(super) fn retain_source_route_predecessors(catalogue: &mut QuestStateCatalogue) {
    for &(quest, saved) in SOURCE_ROUTE_PREDECESSORS {
        if catalogue
            .definition_hash(quest)
            .is_some_and(|current| current != saved)
        {
            catalogue
                .predecessors
                .entry(quest.into())
                .or_default()
                .push(NpcDefinitionPredecessor {
                    hash: saved,
                    defaults: Default::default(),
                });
        }
    }
}

pub(super) fn published_hash(quest: &str, current: DefinitionHash) -> DefinitionHash {
    CITATION_REFRESHES
        .iter()
        .find(|(owner, refreshed, _)| *owner == quest && refreshed.contains(&current))
        .map_or(current, |(_, _, published)| *published)
}

pub(super) fn retain_predecessors(catalogue: &mut QuestStateCatalogue) {
    for &(quest, current, previous) in GAMEPLAY_PREDECESSORS {
        if catalogue.definition_hash(quest) == Some(current) {
            let accepted = catalogue.predecessors.entry(quest.into()).or_default();
            for &hash in previous {
                if accepted.iter().any(|predecessor| predecessor.hash == hash) {
                    continue;
                }
                accepted.push(NpcDefinitionPredecessor {
                    hash,
                    defaults: Default::default(),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quest::{loader, npc_rules};

    #[test]
    fn dragon_corpse_accepts_the_published_cave_pin() -> Result<(), String> {
        let loaded =
            loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        let (quest, saved) = DRAGON_CORPSE_PREDECESSOR;
        assert_ne!(catalogue.definition_hash(quest), Some(saved));
        assert!(npc_rules::legacy_defaults(catalogue, quest, &saved).is_some());
        Ok(())
    }

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
    fn source_routes_keep_exact_main_quest_pins() -> Result<(), String> {
        let loaded =
            loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        for &(quest, saved) in SOURCE_ROUTE_PREDECESSORS {
            let current = catalogue.definition_hash(quest).ok_or(quest)?;
            if current != saved {
                assert!(
                    npc_rules::legacy_defaults(catalogue, quest, &saved).is_some(),
                    "{quest}"
                );
            }
            let mut unrelated = saved;
            unrelated[0] ^= 1;
            assert!(
                npc_rules::legacy_defaults(catalogue, quest, &unrelated).is_none(),
                "{quest}"
            );
        }
        Ok(())
    }

    #[test]
    fn reviewed_package_keeps_exact_published_definition_pins() -> Result<(), String> {
        let loaded =
            loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();

        for &(quest, refreshed, published) in CITATION_REFRESHES {
            if quest == "oteryn:quest.grave_danger_quest" {
                // Zelos credit changes the live definition. Exact citation
                // variants still resolve to the published predecessor.
                assert_ne!(catalogue.definition_hash(quest), Some(published));
                assert!(npc_rules::legacy_defaults(catalogue, quest, &published).is_some());
            } else {
                assert_eq!(catalogue.definition_hash(quest), Some(published), "{quest}");
            }
            for &citation in refreshed {
                assert_eq!(published_hash(quest, citation), published, "{quest}");
                let mut unrelated_revision = citation;
                unrelated_revision[0] ^= 1;
                assert_eq!(
                    published_hash(quest, unrelated_revision),
                    unrelated_revision
                );
            }
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
