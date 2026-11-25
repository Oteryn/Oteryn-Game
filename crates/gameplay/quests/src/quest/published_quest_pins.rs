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

const GRAVE_DANGER_PRE_ZELOS: DefinitionHash =
    hash(b"9f3e5b1cb514d0aa4c8b475fd041fc61910791a53da2ce3ebd1cc80c9d31d2ab");

const PUBLISHED_GRAVE_DANGER_LIVE: DefinitionHash =
    hash(b"286f3e0a4fdaff0ae2a14494889fe2d3492c5bbd4d8668a4f9b2cdaa48e37e7e");

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
    // Merged citation digests captured with quest_definition_hashes.
    (
        "oteryn:quest.formogar_mine_hoist",
        hash(b"573f5ae9cf08a05018a99b7b773797326f93a2fd2e62025f5541bbcafb8795e7"),
        hash(b"17d1d5c6239e5a9b23e4fdaa6ee89231368e507ced80608c055b9f66ede55ebb"),
    ),
    (
        "oteryn:quest.giant_smithhammer_quest",
        hash(b"aee6849e771f86af9ac983344207ba160da0f0889441c0ae148b8c0a4d4cf4cc"),
        hash(b"50bcf5c02dbbbd3a4ac6326dcd43ea563fa96fc8b1ca5d9274d3aa989a04d215"),
    ),
    (
        "oteryn:quest.goblin_merchant_quest",
        hash(b"26d7d2303a2ccd81de839f3a3653db3613762e14fcfec7573cb10c860db3bff0"),
        hash(b"6a7abf586d279cd807c0ebd03967cab2116fc3bb036e72072a9ac1fccf81129b"),
    ),
    (
        "oteryn:quest.heart_of_destruction_quest",
        hash(b"b49004b3ac6547cf7f916ddfb0b343e2fd4a0315bcdf1d46c71f1a66def59154"),
        hash(b"4d4d919cff9c211b7996f838cf149b5ce8ea2437509eb323b65eaf8535d3cb5f"),
    ),
    (
        "oteryn:quest.hero_of_rathleton_quest",
        hash(b"13841887d2cda6862f0898037cc6441d42b704872ded826dfbecb3261a7727a7"),
        hash(b"a124f146976b28bfedba9bf73d43e425fed9267ba60e7c23a523faba0bd11b38"),
    ),
    (
        "oteryn:quest.hidden_threats_quest",
        hash(b"0a77d4d929a6c9719f3f39eb88a631595cf515da121f64fe84b4dc5e63826823"),
        hash(b"0863383d44e7c74131f8d5424156f50df17a5ceb675eeb1be02218b288ebeadc"),
    ),
    (
        "oteryn:quest.grave_danger_quest",
        hash(b"5379f875a00f92429abeaed968037bf4e2778416c0b62a7b4f31cc2ac7c4d0aa"),
        hash(b"286f3e0a4fdaff0ae2a14494889fe2d3492c5bbd4d8668a4f9b2cdaa48e37e7e"),
    ),
    (
        // The derived pre-Zelos definition keeps its exact saved defaults.
        "oteryn:quest.grave_danger_quest",
        hash(b"676537a62b92f7b97f27da4c44aab3fbb5dc1bbcd684c5173d99905a0927416a"),
        GRAVE_DANGER_PRE_ZELOS,
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
    // Canonical Quest shard citations refreshed by the combined Crystal NPC content.
    (
        "oteryn:quest.ferumbras_ascension_quest",
        hash(b"87fac0d81843121fb8e0555c18f0497b81c9ab311058ab0547683c50200d602c"),
        hash(b"2c5453b476d04277fef5fcb9a0816805c039d3361eea70bf28ffda1846693c8f"),
    ),
    (
        "oteryn:quest.forgotten_knowledge_quest",
        hash(b"e3b513a35624575283574370f7be62eb6b4edd46954d77fd99d8338fa437e569"),
        hash(b"4c96930f43d73c762b1f0e546c0f71aa11a24a504fbde760e03327b4e0db74ce"),
    ),
    (
        "oteryn:quest.formogar_mine_hoist",
        hash(b"2534afd2b057d81dce6c819eb3287d0a3e5c2f45de5e566278dd4960a1db1911"),
        hash(b"17d1d5c6239e5a9b23e4fdaa6ee89231368e507ced80608c055b9f66ede55ebb"),
    ),
    (
        "oteryn:quest.giant_smithhammer_quest",
        hash(b"a5a9171709a883b61fe485bf7f11f04cdd7cf3bad7c3af22c2a977627ac0c3c6"),
        hash(b"50bcf5c02dbbbd3a4ac6326dcd43ea563fa96fc8b1ca5d9274d3aa989a04d215"),
    ),
    (
        "oteryn:quest.goblin_merchant_quest",
        hash(b"3b0c0bf62b5e6e48922f569a845c4362af96e45e7ed1d5861a61bdc95eead228"),
        hash(b"6a7abf586d279cd807c0ebd03967cab2116fc3bb036e72072a9ac1fccf81129b"),
    ),
    (
        "oteryn:quest.grave_danger_quest",
        hash(b"69f766380778b7206750fdb0c61124d6b2efba92d7001502012d7fa1d040b1d0"),
        PUBLISHED_GRAVE_DANGER_LIVE,
    ),
    (
        "oteryn:quest.grimvale_quest",
        hash(b"efd7667dbc0334f5c4213ce9bff5f53edd9d090007eb9f4c6a0f998fd9e5fcfc"),
        hash(b"8485475de7287c1d16beffe6e14612fac04e2c8717c5c25ffb49012ea79d4de8"),
    ),
    (
        "oteryn:quest.heart_of_destruction_quest",
        hash(b"2848fd802cce5c88b001f01289c0dac7af6b769c3529b9ac3fd3278ef056a405"),
        hash(b"4d4d919cff9c211b7996f838cf149b5ce8ea2437509eb323b65eaf8535d3cb5f"),
    ),
    (
        "oteryn:quest.hero_of_rathleton_quest",
        hash(b"1338afbd82ba65494e5565ecdcf420ee35945e6bb146e54bccbda05d2e039791"),
        hash(b"a124f146976b28bfedba9bf73d43e425fed9267ba60e7c23a523faba0bd11b38"),
    ),
    (
        "oteryn:quest.hidden_threats_quest",
        hash(b"1350f740c2780c11c22786cd4e0449c79f22fd245a47803bfaff85c44ecae2ae"),
        hash(b"0863383d44e7c74131f8d5424156f50df17a5ceb675eeb1be02218b288ebeadc"),
    ),
    (
        "oteryn:quest.hydra_egg_quest",
        hash(b"4410ec5c32a8a9991447e39703d77bcfb0145cb09c909765ff571789938ac534"),
        hash(b"78c0d4800b0094afbdf9a91ad444c7b8adbad1b2c6c5d8b3b48602837c69ffca"),
    ),
    // The same citation refresh before the Zelos questline effect.
    (
        "oteryn:quest.grave_danger_quest",
        hash(b"5605ff5538ecb520e1a22c62ec5d64f8e6b7ffef27712b0c1492174247b9ee74"),
        hash(b"9f3e5b1cb514d0aa4c8b475fd041fc61910791a53da2ce3ebd1cc80c9d31d2ab"),
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
        hash(b"69a1b3cfaa0d7f3b921da0ea7bb3bafd1de62a22e484e4c333541ba29c9b2a79"),
        &[
            hash(b"2c5453b476d04277fef5fcb9a0816805c039d3361eea70bf28ffda1846693c8f"),
            hash(b"e86e12f668231e6ff93181caf6bd34a4dde5b7d53b9bacff6bc4a44bd12fb464"),
        ],
    ),
    (
        "oteryn:quest.forgotten_knowledge_quest",
        hash(b"cbc0d3c2dfe847aa97daad45f4d322331ea90416ed774d5dddfc4cc7b4567281"),
        &[
            hash(b"4c96930f43d73c762b1f0e546c0f71aa11a24a504fbde760e03327b4e0db74ce"),
            hash(b"298abe628683287d3aa5939bb4769361cccfeb586946e8956daa886f9b37a51a"),
        ],
    ),
    (
        "oteryn:quest.grimvale_quest",
        hash(b"ec073be1d536c213bf81c914202b663a7f862ae7f78316fd97053173e3127823"),
        &[
            hash(b"8485475de7287c1d16beffe6e14612fac04e2c8717c5c25ffb49012ea79d4de8"),
            hash(b"4fb22937877e3de361938b3dc7358b9bb98f799341987579ec56e4f54063a819"),
        ],
    ),
    (
        "oteryn:quest.nightmare_outfits_quest",
        hash(b"dc94a505cae469502aacecb2886cf794f483a0a5082583c9c9fdf505f592bca2"),
        &[
            hash(b"34b34208aeb0321bb2a60bb20de7521a3485153d0338c1e8b71269c22e836d9d"),
            hash(b"a124f146976b28bfedba9bf73d43e425fed9267ba60e7c23a523faba0bd11b38"),
        ],
    ),
    (
        "oteryn:quest.the_ape_city_quest",
        hash(b"3e8a82c701c48fb9e42db74edc6a0c3821b2f1e71dddf805b8d3553043dae44d"),
        &[
            hash(b"18884601e04fc41671d5b24abc67f63cc0cf234813bc1f967ab24297871d1e05"),
            // The authored route on main (c3ca2e8dc) keeps the same track defaults.
            hash(b"353191f4da997af50b1548bdd15276bfabf5f5f0a188f9720465d07ffd56c72f"),
        ],
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
        hash(b"c2247451fc15ba29360c581a59805205a6b8d3a606392342f1c99660cfdea6fa"),
        &[hash(
            b"78c0d4800b0094afbdf9a91ad444c7b8adbad1b2c6c5d8b3b48602837c69ffca",
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
        "oteryn:quest.giant_smithhammer_quest",
        hash(b"02d322ad993f1194f9b16fab996570c08b8d23b29d85d1cc39c19f1473e5c3e4"),
        &[
            hash(b"50bcf5c02dbbbd3a4ac6326dcd43ea563fa96fc8b1ca5d9274d3aa989a04d215"),
            hash(b"bc9a387b5eba43a87478e8016be9c36a86f4cf63a555ade6e4eefae403ca8ba8"),
        ],
    ),
    (
        "oteryn:quest.goblin_merchant_quest",
        hash(b"75ed65085049b76c051ac4d92a8af8d59a3d015fa4cf1c0a973d0c5287f661f2"),
        &[
            hash(b"6a7abf586d279cd807c0ebd03967cab2116fc3bb036e72072a9ac1fccf81129b"),
            hash(b"950746d5cd80d9fd1dd8844d03187fd0527f9c69b276f4aa2da4c70ae06e522f"),
        ],
    ),
    (
        "oteryn:quest.grave_danger_quest",
        hash(b"9f8957002452ecfe907a4be7430a5910db697ac88b74efe8dfaeab0dce568531"),
        &[
            hash(b"286f3e0a4fdaff0ae2a14494889fe2d3492c5bbd4d8668a4f9b2cdaa48e37e7e"),
            hash(b"6121b5603a681798d7e21ba2d8fddfb3a3479f7133f5672c824f6ea72cce87fe"),
            hash(b"9f3e5b1cb514d0aa4c8b475fd041fc61910791a53da2ce3ebd1cc80c9d31d2ab"),
            hash(b"43161afcc986695e8999938e3395de140ec57134c00b1e89bcbe0e640775b5a0"),
            hash(b"4fe40f437a6acad51a574510d5d0bcef6dd7476ff7b1bed9438cc6b88c222ffe"),
        ],
    ),
    (
        "oteryn:quest.hand_of_the_inquisition_outfits_quest",
        hash(b"2524a17ebf71ea5d741672793c0152ef2c7cde8a0e0d858b8c0a424a13426441"),
        &[
            hash(b"ee6c3e406da9bb800d0f73cd6fe71da5fd8064f3f0e66c3c6133de1e045ed529"),
            hash(b"6ce138c8d2d98d0869359e3a93c25e2053fe55fcd729a5dc9887fb0e76cd073e"),
            hash(b"e73c8c57b74a19a1da0b5e2172458951140b1774dc0b526222c8742a156245fc"),
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

// Published by the production loader on main at 95e63dc1c, before Dread Lord.
pub(super) const BROTHERHOOD_BEFORE_DREAD_LORD: DefinitionHash =
    hash(b"4269816e523e2daa58821cadf770d45b98da24e8d149800499f151904dee2905");

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
        let main = hash(b"9f3e5b1cb514d0aa4c8b475fd041fc61910791a53da2ce3ebd1cc80c9d31d2ab");
        assert_ne!(catalogue.definition_hash(grave), Some(main));
        let defaults = catalogue
            .declared_tracks()
            .filter(|track| track.quest == grave)
            .map(|track| (track.key.clone(), track.initial))
            .collect();
        assert_eq!(
            npc_rules::legacy_defaults(catalogue, grave, &main),
            Some(&defaults)
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
            if matches!(
                quest,
                "oteryn:quest.ferumbras_ascension_quest"
                    | "oteryn:quest.forgotten_knowledge_quest"
                    | "oteryn:quest.grimvale_quest"
                    | "oteryn:quest.hydra_egg_quest"
            ) || (quest == "oteryn:quest.grave_danger_quest"
                && published == GRAVE_DANGER_PRE_ZELOS)
            {
                // These quests have since gained gameplay changes. Each citation
                // variant still resolves to the exact published predecessor.
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
