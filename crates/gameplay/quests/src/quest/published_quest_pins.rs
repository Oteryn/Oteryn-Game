//! Definition pins captured from the published 871c1dec7 catalogue and the reviewed 0f605da3 catalogue.
use super::{NpcDefinitionPredecessor, QuestStateCatalogue};

type DefinitionHash = [u8; 32];

pub(super) const fn hash(hex: &[u8; 64]) -> DefinitionHash {
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
        hash(b"94a749650b719e1d0221aa6ae103980b293e217efbc39da5e294e07a3f60987d"),
        hash(b"17d1d5c6239e5a9b23e4fdaa6ee89231368e507ced80608c055b9f66ede55ebb"),
    ),
    (
        "oteryn:quest.giant_smithhammer_quest",
        hash(b"00820d29d902f9dd224a82ec10210292e273d8d2d98a758280451d3a54f338ca"),
        hash(b"50bcf5c02dbbbd3a4ac6326dcd43ea563fa96fc8b1ca5d9274d3aa989a04d215"),
    ),
    (
        "oteryn:quest.goblin_merchant_quest",
        hash(b"0441ba70dc3784573be31f1cff59e7e6954a4d552652959a19c2786abd305b3d"),
        hash(b"6a7abf586d279cd807c0ebd03967cab2116fc3bb036e72072a9ac1fccf81129b"),
    ),
    (
        "oteryn:quest.grave_danger_quest",
        hash(b"52b062dee5106bfe6c5dd58bbb018ed6e5a952f458b65a89d719da7db1424527"),
        hash(b"9f3e5b1cb514d0aa4c8b475fd041fc61910791a53da2ce3ebd1cc80c9d31d2ab"),
    ),
    (
        "oteryn:quest.heart_of_destruction_quest",
        hash(b"d562a2963056485a18d5066552f7ece897ef7642bc2d4c703baebea8cfd45c88"),
        hash(b"4d4d919cff9c211b7996f838cf149b5ce8ea2437509eb323b65eaf8535d3cb5f"),
    ),
    (
        "oteryn:quest.hero_of_rathleton_quest",
        hash(b"8395212c1ba468cb27802cf62a6235028b8b4500f9b4bcb053711bfda65fef1d"),
        hash(b"a124f146976b28bfedba9bf73d43e425fed9267ba60e7c23a523faba0bd11b38"),
    ),
    (
        "oteryn:quest.hidden_threats_quest",
        hash(b"c605417d3767ce5d646d6fd8f78eb64fc7c742ad26307249aa7205a49625e338"),
        hash(b"0863383d44e7c74131f8d5424156f50df17a5ceb675eeb1be02218b288ebeadc"),
    ),
];

// These definitions changed gameplay. Preserve their exact saved pins and keep the
// new rules bound to the new hash. Existing absent-track defaults did not change.
const GAMEPLAY_PREDECESSORS: &[(&str, DefinitionHash, &[DefinitionHash])] = &[
    // Active save pins captured with the production loader at d48146720 (the merge base).
    (
        "oteryn:quest.authored.hive_born_world_change",
        hash(b"8a8b041a8ac06324c70f4f7b197253ce64d3c8b6bbd46a0daaf4af9083ae480e"),
        &[hash(
            b"9db34a9d74b69bc46f47b39bfe3665cb009826d55cc02df1c5f9fee078fd12b6",
        )],
    ),
    (
        "oteryn:quest.dreamer_s_challenge_quest",
        hash(b"6419507bfce77447ba549c62db2375b230d24ccb5677828e4c6eb2ce058d232f"),
        &[hash(
            b"96bb73ad6a5a0e319707484af264d9d6ac48b3d7b5abdd767e7cdae115dc8d8c",
        )],
    ),
    (
        "oteryn:quest.ferumbras_ascension_quest",
        hash(b"904af0470d70e7f7de11c50d476be4eb899041a8b397a67a79e9c41f00d9036c"),
        &[
            hash(b"2c5453b476d04277fef5fcb9a0816805c039d3361eea70bf28ffda1846693c8f"),
            hash(b"e86e12f668231e6ff93181caf6bd34a4dde5b7d53b9bacff6bc4a44bd12fb464"),
        ],
    ),
    (
        "oteryn:quest.forgotten_knowledge_quest",
        hash(b"c3f10d04b28f58a30f98fa7df4082712661ed21f638e12318e9f1d323e386650"),
        &[
            hash(b"4c96930f43d73c762b1f0e546c0f71aa11a24a504fbde760e03327b4e0db74ce"),
            hash(b"298abe628683287d3aa5939bb4769361cccfeb586946e8956daa886f9b37a51a"),
        ],
    ),
    (
        "oteryn:quest.grimvale_quest",
        hash(b"0e190e8ec58f1d4b5600a3743a45fe185371815e9fc20c0b1971316e986fc2fc"),
        &[
            hash(b"8485475de7287c1d16beffe6e14612fac04e2c8717c5c25ffb49012ea79d4de8"),
            hash(b"4fb22937877e3de361938b3dc7358b9bb98f799341987579ec56e4f54063a819"),
        ],
    ),
    (
        "oteryn:quest.nightmare_outfits_quest",
        hash(b"dc94a505cae469502aacecb2886cf794f483a0a5082583c9c9fdf505f592bca2"),
        &[hash(
            b"34b34208aeb0321bb2a60bb20de7521a3485153d0338c1e8b71269c22e836d9d",
        )],
    ),
    (
        "oteryn:quest.the_ape_city_quest",
        hash(b"11bdc83b25d2857d49be51cceb845bca1b00d14ec6e4e140e846c42cb0fb17d0"),
        &[hash(
            b"18884601e04fc41671d5b24abc67f63cc0cf234813bc1f967ab24297871d1e05",
        )],
    ),
    (
        "oteryn:quest.the_beginning_quest",
        hash(b"e7262552558e41f8213c5f4a8dd38f9b976f772606b06df3ba258b3fd545e0ec"),
        &[hash(
            b"2b72c6adf88ff455b53f305bc0f8f35912a8de6fd520222a26bfc0e574e93852",
        )],
    ),
    // Active quest definitions captured from main at 3e09916423181c795c829eca5e5414dde3f29a3e.
    (
        "oteryn:quest.dream_warrior_outfits_quest",
        hash(b"89294b60df026bb41c7f506035840b13ad6fc2d71e3a510d73242caf385bb9a0"),
        &[hash(
            b"549d02b6c708850ac156f8c1bda5211cae5ecf4b1cc4a12cf9e512278d4c191f",
        )],
    ),
    (
        "oteryn:quest.the_first_dragon_quest",
        hash(b"4b8694d760896a03fd774cac416fb65bc82f3277affb5a2c208b6742789d431f"),
        &[hash(
            b"2000974093d7ce811040df0d025f3e725f0a624e5bb6f8c8fdccbda8c1e3dd66",
        )],
    ),
    // Saved definitions from main at 3e099164 precede the Simon and Demon reward additions.
    (
        "oteryn:quest.deeper_fibula_quest",
        hash(b"525b4c15a0f707d9201bedf573ea32e6b17b335a9328b2db89b088c9ab84411d"),
        &[hash(
            b"cf4bbe71022ae339892baec90149fb287837d1ae9736cc70f4b27fdd7cb22088",
        )],
    ),
    (
        "oteryn:quest.demon_helmet_quest",
        hash(b"1b75daf7bd1219fec3fff46027cef7f6099d78b22782ac9b84cb54575c733cd4"),
        &[hash(
            b"d8dc7ff3be9461581bca7600168cf4ee5de64d90e1ff622e52cabd7309a06028",
        )],
    ),
    (
        "oteryn:quest.demon_oak",
        hash(b"4a22d6fc365d36435dd7805abddc6c90fd88ac273cd39fb3d2f4aa98e5aa7201"),
        &[hash(
            b"057ad49a3c234eed32bb2eec994e78225d67424ba67796d099844eaa3d454a8e",
        )],
    ),
    (
        "oteryn:quest.steal_from_thieves_quest",
        hash(b"dbbdbf24cc3bcfe2caefed06b1287254d035e4642998150e2bf909bb3784f94d"),
        &[hash(
            b"ba0a6aa9a34a16c8e56ae191d6af5fbe3ce07928fafbeb50fffeeaf97b4f2672",
        )],
    ),
    (
        "oteryn:quest.hot_cuisine_quest",
        hash(b"dd018f81fad8d5b44cf1731e9fb30e9d352e91f54fc9a1c73537e3e86963dc0d"),
        &[hash(
            b"d34cbfa3109adfc1d61e4ad871a88c097329b1510ed414d85dfbe8d590b5f4f2",
        )],
    ),
    (
        "oteryn:quest.hunting_for_tokens",
        hash(b"2d9c2d53f7ad233d907fd87905dd9686aa5ba0c7b170e9a365b9ec056cec4a4a"),
        &[hash(
            b"f68593b143c5de45970b3a2375e9264d9781bc7968f59cc67c218b4e66cd05b3",
        )],
    ),
    (
        "oteryn:quest.hydra_egg_quest",
        hash(b"ce23f90445efb9cd4020ec098c0d6fa3466bec2454c82c667548f7f0c6a4f448"),
        &[hash(
            b"78c0d4800b0094afbdf9a91ad444c7b8adbad1b2c6c5d8b3b48602837c69ffca",
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

// Published by the production loader on main at 95e63dc1c, before Dread Lord.
pub(super) const BROTHERHOOD_BEFORE_DREAD_LORD: DefinitionHash =
    hash(b"4269816e523e2daa58821cadf770d45b98da24e8d149800499f151904dee2905");

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
    fn brotherhood_preserves_the_definition_for_existing_death_dealers() -> Result<(), String> {
        let loaded =
            loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        let quest = "oteryn:quest.brotherhood_outfits_quest";
        let previous = BROTHERHOOD_BEFORE_DREAD_LORD;
        assert_ne!(loaded.catalogue().definition_hash(quest), Some(previous));
        let defaults = npc_rules::legacy_defaults(loaded.catalogue(), quest, &previous)
            .ok_or("published Brotherhood definition is no longer admitted")?;
        assert_eq!(
            defaults.get("oteryn:quest-progress/quest/u7_9/brotherhood_outfits/door"),
            Some(&0)
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

    #[test]
    fn accepts_every_changed_active_merge_base_pin() -> Result<(), String> {
        // Captured using load_embedded_quest_state at d4814672050acda4458f7d8129f351183df02b65.
        let saved = [
            (
                "oteryn:quest.authored.hive_born_world_change",
                hash(b"9db34a9d74b69bc46f47b39bfe3665cb009826d55cc02df1c5f9fee078fd12b6"),
            ),
            (
                "oteryn:quest.brotherhood_outfits_quest",
                hash(b"4269816e523e2daa58821cadf770d45b98da24e8d149800499f151904dee2905"),
            ),
            (
                "oteryn:quest.deeper_fibula_quest",
                hash(b"cf4bbe71022ae339892baec90149fb287837d1ae9736cc70f4b27fdd7cb22088"),
            ),
            (
                "oteryn:quest.demon_helmet_quest",
                hash(b"d8dc7ff3be9461581bca7600168cf4ee5de64d90e1ff622e52cabd7309a06028"),
            ),
            (
                "oteryn:quest.demon_oak",
                hash(b"057ad49a3c234eed32bb2eec994e78225d67424ba67796d099844eaa3d454a8e"),
            ),
            (
                "oteryn:quest.dream_warrior_outfits_quest",
                hash(b"549d02b6c708850ac156f8c1bda5211cae5ecf4b1cc4a12cf9e512278d4c191f"),
            ),
            (
                "oteryn:quest.dreamer_s_challenge_quest",
                hash(b"96bb73ad6a5a0e319707484af264d9d6ac48b3d7b5abdd767e7cdae115dc8d8c"),
            ),
            (
                "oteryn:quest.ferumbras_ascension_quest",
                hash(b"2c5453b476d04277fef5fcb9a0816805c039d3361eea70bf28ffda1846693c8f"),
            ),
            (
                "oteryn:quest.forgotten_knowledge_quest",
                hash(b"4c96930f43d73c762b1f0e546c0f71aa11a24a504fbde760e03327b4e0db74ce"),
            ),
            (
                "oteryn:quest.grimvale_quest",
                hash(b"8485475de7287c1d16beffe6e14612fac04e2c8717c5c25ffb49012ea79d4de8"),
            ),
            (
                "oteryn:quest.hot_cuisine_quest",
                hash(b"d34cbfa3109adfc1d61e4ad871a88c097329b1510ed414d85dfbe8d590b5f4f2"),
            ),
            (
                "oteryn:quest.hunting_for_tokens",
                hash(b"f68593b143c5de45970b3a2375e9264d9781bc7968f59cc67c218b4e66cd05b3"),
            ),
            (
                "oteryn:quest.hydra_egg_quest",
                hash(b"78c0d4800b0094afbdf9a91ad444c7b8adbad1b2c6c5d8b3b48602837c69ffca"),
            ),
            (
                "oteryn:quest.nightmare_outfits_quest",
                hash(b"34b34208aeb0321bb2a60bb20de7521a3485153d0338c1e8b71269c22e836d9d"),
            ),
            (
                "oteryn:quest.outfit_and_addon_quests",
                hash(b"ce65daca1c0a3c499c3f23fe60ae01aeb517e1aaea4689022c055971de786e6f"),
            ),
            (
                "oteryn:quest.sam_s_old_backpack_quest",
                hash(b"e618fcebed1f65d871a0228c3f508c8fa74abae80f6c162585dda4dcff2e0364"),
            ),
            (
                "oteryn:quest.steal_from_thieves_quest",
                hash(b"ba0a6aa9a34a16c8e56ae191d6af5fbe3ce07928fafbeb50fffeeaf97b4f2672"),
            ),
            (
                "oteryn:quest.the_ape_city_quest",
                hash(b"18884601e04fc41671d5b24abc67f63cc0cf234813bc1f967ab24297871d1e05"),
            ),
            (
                "oteryn:quest.the_beginning_quest",
                hash(b"2b72c6adf88ff455b53f305bc0f8f35912a8de6fd520222a26bfc0e574e93852"),
            ),
            (
                "oteryn:quest.the_first_dragon_quest",
                hash(b"2000974093d7ce811040df0d025f3e725f0a624e5bb6f8c8fdccbda8c1e3dd66"),
            ),
        ];
        let loaded =
            loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        let mut missing = Vec::new();
        for (quest, pin) in saved {
            if catalogue.definition_hash(quest) != Some(pin)
                && npc_rules::legacy_defaults(catalogue, quest, &pin).is_none()
            {
                missing.push(quest);
            }
        }
        assert!(
            missing.is_empty(),
            "unrecognized merge-base saves: {missing:?}"
        );
        Ok(())
    }
}
