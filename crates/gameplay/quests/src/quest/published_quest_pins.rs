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
        hash(b"5c6e9bc04b7dc5881f879e073bf3165d452e599d3633b64416cf1f611ac9523d"),
        hash(b"9f3e5b1cb514d0aa4c8b475fd041fc61910791a53da2ce3ebd1cc80c9d31d2ab"),
    ),
    (
        "oteryn:quest.grimvale_quest",
        hash(b"be926e9f2acf5908483637cc2f83c79dbeb7b3418c4051cb43a2ffa779233d65"),
        hash(b"8485475de7287c1d16beffe6e14612fac04e2c8717c5c25ffb49012ea79d4de8"),
    ),
    // Shaman Treasure restores a UID in the shared Edron region citation.
    (
        "oteryn:quest.grimvale_quest",
        hash(b"1eef1d1432518c55c87ff99b0462d5d0f43ba2412d3aa9873156df53ec9509c1"),
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
    // Package 4 changes the placement inventory cited by these existing routes.
    (
        "oteryn:quest.ferumbras_ascension_quest",
        hash(b"90726ffb2298ce404833c2a08cee70eb699dd600256d2edf6763858d83697685"),
        hash(b"2c5453b476d04277fef5fcb9a0816805c039d3361eea70bf28ffda1846693c8f"),
    ),
    (
        "oteryn:quest.forgotten_knowledge_quest",
        hash(b"c5e817a79d863a9385dea01bfb4ab98f824a3a7d9be6656a1ee0ad1f19bbcb35"),
        hash(b"4c96930f43d73c762b1f0e546c0f71aa11a24a504fbde760e03327b4e0db74ce"),
    ),
    (
        "oteryn:quest.formogar_mine_hoist",
        hash(b"011e38be924e9587dc34a60e68ce55785a2813422ceeb40d35b82bd73a1857a0"),
        hash(b"17d1d5c6239e5a9b23e4fdaa6ee89231368e507ced80608c055b9f66ede55ebb"),
    ),
    (
        "oteryn:quest.giant_smithhammer_quest",
        hash(b"9501a6b0ebd1a3c91f01f62ce60193d4e60e963b89b35dd63874067e60b7a211"),
        hash(b"50bcf5c02dbbbd3a4ac6326dcd43ea563fa96fc8b1ca5d9274d3aa989a04d215"),
    ),
    (
        "oteryn:quest.goblin_merchant_quest",
        hash(b"e5374a03d334a81681c8124b29a64a9a256ab626e7b242b10be790684136fe46"),
        hash(b"6a7abf586d279cd807c0ebd03967cab2116fc3bb036e72072a9ac1fccf81129b"),
    ),
    (
        "oteryn:quest.grave_danger_quest",
        hash(b"4736dfb1c2085d6000f06bb8a7cb811d4f55714a01e53f5c4f35a79b01db902f"),
        hash(b"9f3e5b1cb514d0aa4c8b475fd041fc61910791a53da2ce3ebd1cc80c9d31d2ab"),
    ),
    (
        "oteryn:quest.grimvale_quest",
        hash(b"bcfa73062ad864018ae4788e1fb314e73c105d5e5c06429dccae36caaba8e558"),
        hash(b"8485475de7287c1d16beffe6e14612fac04e2c8717c5c25ffb49012ea79d4de8"),
    ),
    (
        "oteryn:quest.heart_of_destruction_quest",
        hash(b"31d2eda46fbc8ceb31b0f874d61ac3cd5fb4a75b764007cbfb26753a557e1042"),
        hash(b"4d4d919cff9c211b7996f838cf149b5ce8ea2437509eb323b65eaf8535d3cb5f"),
    ),
    (
        "oteryn:quest.hero_of_rathleton_quest",
        hash(b"3565804c6beed42b4e4f88d875db68388c3b6ec34df17e0d892881ebdd46ca14"),
        hash(b"a124f146976b28bfedba9bf73d43e425fed9267ba60e7c23a523faba0bd11b38"),
    ),
    (
        "oteryn:quest.hidden_threats_quest",
        hash(b"997c65cbbd982f65e0099b3071c1708962f1d46ad2e37bfbc24f90997281dfed"),
        hash(b"0863383d44e7c74131f8d5424156f50df17a5ceb675eeb1be02218b288ebeadc"),
    ),
    (
        "oteryn:quest.hydra_egg_quest",
        hash(b"9b29515bba98d0b790f0bfdaa746c0223b6ccccf328e369370bcd9993a5a5067"),
        hash(b"78c0d4800b0094afbdf9a91ad444c7b8adbad1b2c6c5d8b3b48602837c69ffca"),
    ),
];

// These definitions changed gameplay. Preserve their exact saved pins and keep the
// new rules bound to the new hash. Existing absent-track defaults did not change.
const GAMEPLAY_PREDECESSORS: &[(&str, DefinitionHash, &[DefinitionHash])] = &[
    // Captured by quest_definition_hashes on main 9295760b3 and the reviewed route definitions.
    (
        "oteryn:quest.authored.kingdom_of_kormarak_quest",
        hash(b"299da386ca452bd3715cefe2ff71405897dda80a5198e725f06596c4b0e1120c"),
        &[hash(
            b"2d7f93ccfa1ffea4e4e881645c2d031f52d4508348efd3ce863603c51f029553",
        )],
    ),
    (
        "oteryn:quest.authored.mastermind_potion_quest",
        hash(b"9de149d1bcedb6d2c7c7b06c20a7924e1f76646a913e680cce3962fbb8d7daa3"),
        &[hash(
            b"092155f83fe1b21c345c6f8a4b1eff439f42861a4eeaa0643a17f51cffd3a603",
        )],
    ),
    (
        "oteryn:quest.authored.mystery_of_the_valley_quest",
        hash(b"c2f0f0fdeb2d28b4a746e7117d844cb26dddc553f2c081b54df2b81815b040de"),
        &[hash(
            b"9262faea34889f6fa7f476319d0abf7fe0e51934d80cf77809d1b07ade66baa8",
        )],
    ),
    (
        "oteryn:quest.hunter_outfits_quest",
        hash(b"f1857cd86d557a97d58f8a180b0e8919ab24c5866bd61ab675f65780b435a42c"),
        &[hash(
            b"8fb20493f47ae0a1444ff5cb584e14d408b8b6435aabf0175c1118703ff43d6c",
        )],
    ),
    (
        "oteryn:quest.mad_mage_room_quest",
        hash(b"cfa1db835dd1b1ef55b513276870506dd0cf38587ee1667b32f8d062243486f6"),
        &[hash(
            b"bee52340c68d21b8b93951a9a6944572162e714538ca745766eb88414f5d630e",
        )],
    ),
    // Predecessors measured on main 3e0991642 with quest_definition_hashes.
    (
        "oteryn:quest.authored.venore_daily_tasks_quest",
        hash(b"4a3a65c5e76cb8e0b5cd57d6721cc5d1634d91bdeb7339a667485dc867c9ade7"),
        &[hash(
            b"ef8a4b6d2a00bac0fe4ac8db70d91b81ce31bc68c27924539079714bbdbe15e3",
        )],
    ),
    (
        "oteryn:quest.authored.voodoo_doll_quest",
        hash(b"ca164208efed0f55b22206e2fe29f6cf706c693f17d8c4f59d3b466df6f4faff"),
        &[hash(
            b"99642b43c08d8bba83b6eac982a1a0da4fd67c054dad406fd473c1645bb78721",
        )],
    ),
    (
        "oteryn:quest.authored.nomads_land_quest",
        hash(b"852db8606f6acf8a6d12e55c775a2f62a2eeac0594c6457af10ebb0f6b2e2325"),
        &[hash(
            b"84770d717573d6e6d48dc836dd6bea5d30330cc90b0e2e82e63ff29825735360",
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
        "oteryn:quest.outfit_and_addon_quests",
        hash(b"8ad5b2673382523cc269396f2c2016226e18ac9f894ac2049264993ae5982d19"),
        &[
            hash(b"ce65daca1c0a3c499c3f23fe60ae01aeb517e1aaea4689022c055971de786e6f"),
            hash(b"bd44ba2f572315de58bc53958ee1bf398a7039ae5badf3e796017965d051f6c8"),
        ],
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
        hash(b"fdaedce30df4cf940d0022e341e5c0e9be4025a17564ae8f0fed960e350cfe4e"),
        &[hash(
            b"299b92c459921a7f30b4128518dc869fc7e7f6de3d50dbcf3be09ee05dcb44c0",
        )],
    ),
    (
        "oteryn:quest.the_new_frontier_quest",
        hash(b"61034f9e3365c8817d659f8ffa59a0f21e404d2a41dfc6099a48a5cedd71018f"),
        &[hash(
            b"e947f2485cdf2c3094ba301e71369a208d98b017d08aef8e727705a992b6f0be",
        )],
    ),
    (
        "oteryn:quest.wayfarer_outfits_quest",
        hash(b"04f1928fb6d02943927358fc1e3947695680bc704e44afcbbbe5ac6548de194e"),
        &[hash(
            b"68a6045ea88cf241d8ec6f881245131a9076d8d423722544573e9c3b45dae42c",
        )],
    ),
    // Captured from the complete loader at 3e09916423181c795c829eca5e5414dde3f29a3e.
    (
        "oteryn:quest.the_paradox_tower_quest",
        hash(b"1d305dfe05863eea4575a95366cb15dda92d00d22b6c9dff915468932f165cde"),
        &[hash(
            b"4fc6ffa2b309728ed9ee0e76381c1e6d20d618389b2e74bdd5b773cf108bdec5",
        )],
    ),
    (
        "oteryn:quest.the_queen_of_the_banshees_quest",
        hash(b"32cdd6f504fc316133423cac07489eb9164f00ea9cd93e0c550eeb5efc66f488"),
        &[hash(
            b"678a96577e5535714661dd42a36a1fe44de1eafa146415a9d644d9aa96ede1b3",
        )],
    ),
    (
        "oteryn:quest.tibia_tales",
        hash(b"4bc3fe57d6e18a10a467d3944a6cbebcf592554d1ac35c7722db35973ddedd92"),
        &[
            hash(b"a8a3f3a72ac62a5e522cde7872023af874dd43e98dd02ea3f03edb11446d8835"),
            hash(b"8d75517e41359a25abea653c8fd44fa5b4d2768c1470d9acaa9442b73f543065"),
            hash(b"26da6d39754ac853af2a65436856b5b6f8b860dbdd5a5db4553b16fe2009ce4e"),
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
            let predecessors = catalogue.predecessors.entry(quest.into()).or_default();
            for &hash in previous {
                if !predecessors
                    .iter()
                    .any(|predecessor| predecessor.hash == hash)
                {
                    predecessors.push(NpcDefinitionPredecessor {
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
    fn donor_quest_saved_voodoo_and_venore_pins_remain_admitted() -> Result<(), String> {
        let loaded =
            loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        for (quest, predecessor) in [
            (
                "oteryn:quest.authored.venore_daily_tasks_quest",
                hash(b"ef8a4b6d2a00bac0fe4ac8db70d91b81ce31bc68c27924539079714bbdbe15e3"),
            ),
            (
                "oteryn:quest.authored.voodoo_doll_quest",
                hash(b"99642b43c08d8bba83b6eac982a1a0da4fd67c054dad406fd473c1645bb78721"),
            ),
        ] {
            assert!(
                npc_rules::legacy_defaults(catalogue, quest, &predecessor).is_some(),
                "source-era progress at the requested base must remain admitted: {quest}"
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
