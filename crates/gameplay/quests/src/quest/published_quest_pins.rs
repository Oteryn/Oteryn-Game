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

const GRAVE_DANGER_PRE_ZELOS: DefinitionHash =
    hash(b"9f3e5b1cb514d0aa4c8b475fd041fc61910791a53da2ce3ebd1cc80c9d31d2ab");

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
        GRAVE_DANGER_PRE_ZELOS,
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
        GRAVE_DANGER_PRE_ZELOS,
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
        GRAVE_DANGER_PRE_ZELOS,
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
    // Generated from the merged definitions and the published c3ca2e8dc loader.
    (
        "oteryn:quest.ferumbras_ascension_quest",
        hash(b"05cdf255318dcd268826279656a9d779b53b88b8030efb2dc13600cb756d4b64"),
        hash(b"2c5453b476d04277fef5fcb9a0816805c039d3361eea70bf28ffda1846693c8f"),
    ),
    (
        "oteryn:quest.forgotten_knowledge_quest",
        hash(b"372df6f08b7588a01455b9f2ffef138d1bb52c70ee7b6ddde117babdd098b874"),
        hash(b"4c96930f43d73c762b1f0e546c0f71aa11a24a504fbde760e03327b4e0db74ce"),
    ),
    (
        "oteryn:quest.formogar_mine_hoist",
        hash(b"27004a9af9cc1bb7b59f061b9bbbd8b01a00e0f320a9d1e3682e91a41549ebdf"),
        hash(b"17d1d5c6239e5a9b23e4fdaa6ee89231368e507ced80608c055b9f66ede55ebb"),
    ),
    (
        "oteryn:quest.giant_smithhammer_quest",
        hash(b"5ce048870a692127563f83b2766a0b89034d2f74b09bb75c67826608d3f6ed64"),
        hash(b"50bcf5c02dbbbd3a4ac6326dcd43ea563fa96fc8b1ca5d9274d3aa989a04d215"),
    ),
    (
        "oteryn:quest.goblin_merchant_quest",
        hash(b"cdaf632abd1feb4f61b76a2e03eb6c1bcd604b7e5f1bdff692c1289aea567b63"),
        hash(b"6a7abf586d279cd807c0ebd03967cab2116fc3bb036e72072a9ac1fccf81129b"),
    ),
    (
        "oteryn:quest.grave_danger_quest",
        hash(b"622fd073a0be3b84c8c78c515db66bf8d09f18ba6fadb009b469572269c7c096"),
        hash(b"286f3e0a4fdaff0ae2a14494889fe2d3492c5bbd4d8668a4f9b2cdaa48e37e7e"),
    ),
    (
        "oteryn:quest.grave_danger_quest",
        hash(b"b29cb579067c778ba2fa8d4fba288bb8451dfb19486e87499877107d4f160789"),
        GRAVE_DANGER_PRE_ZELOS,
    ),
    (
        "oteryn:quest.grimvale_quest",
        hash(b"6b3088673629c1cd58dbbd984e4d1d43eb18ba15e571584859f92ea46b81eb2f"),
        hash(b"8485475de7287c1d16beffe6e14612fac04e2c8717c5c25ffb49012ea79d4de8"),
    ),
    (
        "oteryn:quest.heart_of_destruction_quest",
        hash(b"e82d27abd4cf3c63aec484c243af15969b50b182ba5fd53e615a89b2682b0f73"),
        hash(b"4d4d919cff9c211b7996f838cf149b5ce8ea2437509eb323b65eaf8535d3cb5f"),
    ),
    (
        "oteryn:quest.hero_of_rathleton_quest",
        hash(b"d7d6e50db66e0c7aafb8d00355f40bd531d470a3ca8a7c04da4f9255d515acfb"),
        hash(b"a124f146976b28bfedba9bf73d43e425fed9267ba60e7c23a523faba0bd11b38"),
    ),
    (
        "oteryn:quest.hidden_threats_quest",
        hash(b"fc793d42c84350dbba73b31753ca10b1b94ee114b9c8d67c4cac9b2ad848b0e7"),
        hash(b"0863383d44e7c74131f8d5424156f50df17a5ceb675eeb1be02218b288ebeadc"),
    ),
    (
        "oteryn:quest.hydra_egg_quest",
        hash(b"9e0e1064ccff5db2a9a3c7272e783136889ce9fe8721aacdae663e62f3423c99"),
        hash(b"78c0d4800b0094afbdf9a91ad444c7b8adbad1b2c6c5d8b3b48602837c69ffca"),
    ),
];

type GameplayPredecessors = (&'static str, DefinitionHash, &'static [DefinitionHash]);

const NEW_FRONTIER_SOURCE_PIN: DefinitionHash =
    hash(b"e947f2485cdf2c3094ba301e71369a208d98b017d08aef8e727705a992b6f0be");
// The native route is also loaded without authored bindings. Keep its exact predecessor.
const NEW_FRONTIER_NATIVE_PREDECESSORS: GameplayPredecessors = (
    "oteryn:quest.the_new_frontier_quest",
    hash(b"61034f9e3365c8817d659f8ffa59a0f21e404d2a41dfc6099a48a5cedd71018f"),
    &[NEW_FRONTIER_SOURCE_PIN],
);

// These definitions changed gameplay. Preserve their exact saved pins and keep the
// new rules bound to the new hash. Existing absent-track defaults did not change.
const GAMEPLAY_PREDECESSORS: &[GameplayPredecessors] = &[
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
        hash(b"f9785be106728cc026d6515c8e092eb08965d3d8e65bcb3a54f0816041a4d9dc"),
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
        hash(b"6aba631a049bcb9213d7192bcc98fcaf7c94ef7ce643c4c741c7f738f3e0a233"),
        &[
            NEW_FRONTIER_SOURCE_PIN,
            hash(b"cb5d7d7008e10821fc65677dec2475d278235b7f5c82ac1f99a2e615873b9e1c"),
        ],
    ),
    (
        "oteryn:quest.wayfarer_outfits_quest",
        hash(b"04f1928fb6d02943927358fc1e3947695680bc704e44afcbbbe5ac6548de194e"),
        &[hash(
            b"68a6045ea88cf241d8ec6f881245131a9076d8d423722544573e9c3b45dae42c",
        )],
    ),
    (
        "oteryn:quest.the_explorer_society_quest",
        hash(b"f51489994ea4f58e5ed3bd18de0a2120cb2852b8005a63eaee08ff86b4128fb1"),
        &[hash(
            b"eef6a0c9dfb1196d65489ccf8478febf6fe02a32cc3696cbe3b2f12a7eb63740",
        )],
    ),
    // Captured from the complete loader at 3e09916423181c795c829eca5e5414dde3f29a3e.
    (
        "oteryn:quest.the_paradox_tower_quest",
        hash(b"0d6ee6f5e6bf5651b9d3442f9e393f31d549bd6450c861bd94caf6fbbb3f5962"),
        &[
            hash(b"4fc6ffa2b309728ed9ee0e76381c1e6d20d618389b2e74bdd5b773cf108bdec5"),
            hash(b"3176623ca229a7e3d443e82077c9d162a7990607f78532e5601f801e1b26f3ff"),
        ],
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

// The pkq2 Tibia Tales and Cursed Crystal source cohorts extend this successor.
const TIBIA_TALES_TRAIN_SUCCESSOR: DefinitionHash =
    hash(b"08ddd97de427afb52741eacdef7781de95f438f5a86a88915d484b447e885db7");

fn is_successor(quest: &str, current: DefinitionHash, active: Option<DefinitionHash>) -> bool {
    active == Some(current)
        || (quest == "oteryn:quest.tibia_tales" && active == Some(TIBIA_TALES_TRAIN_SUCCESSOR))
}

pub(super) fn published_hash(quest: &str, current: DefinitionHash) -> DefinitionHash {
    CITATION_REFRESHES
        .iter()
        .find(|(owner, refreshed, _)| *owner == quest && *refreshed == current)
        .map_or(current, |(_, _, published)| *published)
}

pub(super) fn retain_predecessors(catalogue: &mut QuestStateCatalogue) {
    for &(quest, current, previous) in GAMEPLAY_PREDECESSORS
        .iter()
        .chain([&NEW_FRONTIER_NATIVE_PREDECESSORS])
    {
        if is_successor(quest, current, catalogue.definition_hash(quest)) {
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
    fn nomads_land_accepts_main_save_under_the_composed_tibia_tales_pin() -> Result<(), String> {
        let loaded =
            loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        let mut catalogue = loaded.catalogue().clone();
        let quest = "oteryn:quest.tibia_tales";
        let main = hash(b"8d75517e41359a25abea653c8fd44fa5b4d2768c1470d9acaa9442b73f543065");
        for (successor, accepted) in [(TIBIA_TALES_TRAIN_SUCCESSOR, true), ([0x5a; 32], false)] {
            catalogue.predecessors.remove(quest);
            catalogue.hashes.insert(quest.into(), successor);
            retain_predecessors(&mut catalogue);
            assert_eq!(
                npc_rules::legacy_defaults(&catalogue, quest, &main).is_some(),
                accepted
            );
        }
        Ok(())
    }

    #[test]
    fn package_accepts_main_grave_danger_pin_as_predecessor() -> Result<(), String> {
        let loaded =
            loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        let grave = "oteryn:quest.grave_danger_quest";
        let main = GRAVE_DANGER_PRE_ZELOS;
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
            if published == GRAVE_DANGER_PRE_ZELOS {
                // Zelos credit changes the live definition. The pre-Zelos citation
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
            let active = catalogue.definition_hash(quest);
            assert!(is_successor(quest, current, active), "{quest}: {active:?}");
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
