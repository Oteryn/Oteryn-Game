//! Exact definition pins captured from published and reviewed quest catalogues.
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
const CITATION_REFRESHES: &[(&str, &[DefinitionHash], DefinitionHash)] = &[
    (
        "oteryn:quest.factions",
        &[hash(
            b"443998012475d3e1daf6d0f0decc60f51d56fc03408583c6fd5c4927f86f5106",
        )],
        hash(b"16a0ebe1810d54cdaa467424e4e3f0ab02fa4a4b8a728d7aea33706c9957d738"),
    ),
    (
        "oteryn:quest.formogar_mine_hoist",
        &[
            hash(b"94a749650b719e1d0221aa6ae103980b293e217efbc39da5e294e07a3f60987d"),
            hash(b"4dd6fb32f7883de4278a7096fa5a36ae077efa682522ae94abf97d8f02c1242c"),
            hash(b"573f5ae9cf08a05018a99b7b773797326f93a2fd2e62025f5541bbcafb8795e7"),
            hash(b"011e38be924e9587dc34a60e68ce55785a2813422ceeb40d35b82bd73a1857a0"),
            hash(b"27004a9af9cc1bb7b59f061b9bbbd8b01a00e0f320a9d1e3682e91a41549ebdf"),
            hash(b"2534afd2b057d81dce6c819eb3287d0a3e5c2f45de5e566278dd4960a1db1911"),
            hash(b"23b3dd6ef1bb877686122ca1d97412221ec735bab162680a08b94743d8016bf9"),
            hash(b"d91460774be98a4b174ae1c83d7d2c7e7efcf5fc5ac115f7cd60ae12094840fa"),
            hash(b"617bff2ce025747e4a7747fce2009d039c79c0416d2ba226ed735d0ea5d76ede"),
            hash(b"8e6dd324609ad288a973fc96293b0d8230bb51f8093f6e676199e7c173b6a191"),
            hash(b"717ed235f59e2a87f75331f18199877f1324995ab8c3cf2d80cf52a1593d74ab"),
        ],
        hash(b"17d1d5c6239e5a9b23e4fdaa6ee89231368e507ced80608c055b9f66ede55ebb"),
    ),
    (
        "oteryn:quest.giant_smithhammer_quest",
        &[
            hash(b"00820d29d902f9dd224a82ec10210292e273d8d2d98a758280451d3a54f338ca"),
            hash(b"4c294712b959c58cfc957e27be05b89278dedecda374e8f8a242169c81319722"),
            hash(b"aee6849e771f86af9ac983344207ba160da0f0889441c0ae148b8c0a4d4cf4cc"),
            hash(b"9501a6b0ebd1a3c91f01f62ce60193d4e60e963b89b35dd63874067e60b7a211"),
            hash(b"5ce048870a692127563f83b2766a0b89034d2f74b09bb75c67826608d3f6ed64"),
            hash(b"a5a9171709a883b61fe485bf7f11f04cdd7cf3bad7c3af22c2a977627ac0c3c6"),
            hash(b"4db65fa625c6c170dd62e21fac749c3052ff2d98c9ef6257ea64693373647b04"),
            hash(b"8c6b244adca3ac947bee105824abdc268648729a62add19adc897cc47845b1f1"),
            hash(b"ae411e4ba985d12638e50fc7497cca6ca52ce87e02e2cb7b1382b9097a89ea5b"),
        ],
        hash(b"50bcf5c02dbbbd3a4ac6326dcd43ea563fa96fc8b1ca5d9274d3aa989a04d215"),
    ),
    (
        "oteryn:quest.goblin_merchant_quest",
        &[
            hash(b"0441ba70dc3784573be31f1cff59e7e6954a4d552652959a19c2786abd305b3d"),
            hash(b"681ebe711c3e9c9624881d6ccd9ed49f4045563e6429329a516ebc0cb496f5f8"),
            hash(b"26d7d2303a2ccd81de839f3a3653db3613762e14fcfec7573cb10c860db3bff0"),
            hash(b"e5374a03d334a81681c8124b29a64a9a256ab626e7b242b10be790684136fe46"),
            hash(b"cdaf632abd1feb4f61b76a2e03eb6c1bcd604b7e5f1bdff692c1289aea567b63"),
            hash(b"3b0c0bf62b5e6e48922f569a845c4362af96e45e7ed1d5861a61bdc95eead228"),
            hash(b"30c5f41357098fd2eda299fc3e35bb5f1d69e8469872f1d3368fc4d7792041da"),
            hash(b"fe96732992ad3b207b75433d64569fffc0792eacc0eb6e2a16e80a636d5046ad"),
            hash(b"c47908e169bd3d739136df750c3d5d9a227e73e579457abeb054329c0c3d3b5b"),
        ],
        hash(b"6a7abf586d279cd807c0ebd03967cab2116fc3bb036e72072a9ac1fccf81129b"),
    ),
    (
        "oteryn:quest.grave_danger_quest",
        &[
            hash(b"5c6e9bc04b7dc5881f879e073bf3165d452e599d3633b64416cf1f611ac9523d"),
            hash(b"540cd73ad49e78251491ea0266f3930f000b57c26007a5fcb0dcaf2bea4a4113"),
            hash(b"676537a62b92f7b97f27da4c44aab3fbb5dc1bbcd684c5173d99905a0927416a"),
            hash(b"4736dfb1c2085d6000f06bb8a7cb811d4f55714a01e53f5c4f35a79b01db902f"),
            hash(b"b29cb579067c778ba2fa8d4fba288bb8451dfb19486e87499877107d4f160789"),
            hash(b"daec3ae8d16f98a0793bb3fdee1d20baacb36333f8f28bfa06b9f810d7a5c1be"),
        ],
        GRAVE_DANGER_PRE_ZELOS,
    ),
    (
        "oteryn:quest.grimvale_quest",
        &[
            hash(b"8d5144f88d2889bd95d34648f36383a90b0bda712e4bd2f2b40d0256f0f854cd"),
            hash(b"ffc0064a21640b08fd5cddd020cac005af21aed2fc98c8a0c76369e3ddbd33bf"),
            hash(b"1eef1d1432518c55c87ff99b0462d5d0f43ba2412d3aa9873156df53ec9509c1"),
            hash(b"bf28ee0fb34e8cb739fcaa7c0f6717f1eafdb599e54afffc4f94ff04042a2ef5"),
            hash(b"bcfa73062ad864018ae4788e1fb314e73c105d5e5c06429dccae36caaba8e558"),
            hash(b"6b3088673629c1cd58dbbd984e4d1d43eb18ba15e571584859f92ea46b81eb2f"),
            hash(b"efd7667dbc0334f5c4213ce9bff5f53edd9d090007eb9f4c6a0f998fd9e5fcfc"),
            hash(b"845f231a52f3b61457c2bfd659756cc076c73948215b0c1e4e5c686873dad697"),
            hash(b"be926e9f2acf5908483637cc2f83c79dbeb7b3418c4051cb43a2ffa779233d65"),
            hash(b"efe0803a67f5d52dd56ad28b9cece21419f5b7109b0f57752ebfc6eb2f978d17"),
        ],
        hash(b"8485475de7287c1d16beffe6e14612fac04e2c8717c5c25ffb49012ea79d4de8"),
    ),
    (
        "oteryn:quest.grimvale_quest",
        &[
            hash(b"8d739e34de08f00342139183123982adb253d99776829429bcccaa471136702d"),
            hash(b"87207e32015277e46a85905acf5e59ce524c22817b920255eeb62a7310f68923"),
        ],
        hash(b"a12ba535924a59362bc81815385cc2164cd29b0a666f71c94340ff98a5a3970d"),
    ),
    (
        "oteryn:quest.heart_of_destruction_quest",
        &[
            hash(b"d562a2963056485a18d5066552f7ece897ef7642bc2d4c703baebea8cfd45c88"),
            hash(b"ca161893db0debcd005d1359f804106925c73f3e9fe397abb2fb710425dbb157"),
            hash(b"b49004b3ac6547cf7f916ddfb0b343e2fd4a0315bcdf1d46c71f1a66def59154"),
            hash(b"31d2eda46fbc8ceb31b0f874d61ac3cd5fb4a75b764007cbfb26753a557e1042"),
            hash(b"e82d27abd4cf3c63aec484c243af15969b50b182ba5fd53e615a89b2682b0f73"),
            hash(b"2848fd802cce5c88b001f01289c0dac7af6b769c3529b9ac3fd3278ef056a405"),
            hash(b"939300991b2aeba03d7caea13110e0c65acc2ba02d6edeca688d7bb010e49f80"),
            hash(b"24fc148f94a14a65c654cb3c583396ecd78928dfe925a3e02d96d4b939f3f0bb"),
            hash(b"5fd58496a9bbec4ab0feda817d182f73fc9b562cfeee6c48ccec84df16fe064d"),
            hash(b"d2b693bda2cb942c371056f6267c980bedff85882b81dbb1e3661732ba4b6e5d"),
            hash(b"6932cd6660ef753b3d772eb61d25630e8885af55542c486a1f5e22be6d0c4abd"),
        ],
        hash(b"4d4d919cff9c211b7996f838cf149b5ce8ea2437509eb323b65eaf8535d3cb5f"),
    ),
    (
        "oteryn:quest.hero_of_rathleton_quest",
        &[
            hash(b"8395212c1ba468cb27802cf62a6235028b8b4500f9b4bcb053711bfda65fef1d"),
            hash(b"36e6a39b51ab64edf9e285c81cb6248266c201b428f61841c490d136db4464e0"),
            hash(b"13841887d2cda6862f0898037cc6441d42b704872ded826dfbecb3261a7727a7"),
            hash(b"3565804c6beed42b4e4f88d875db68388c3b6ec34df17e0d892881ebdd46ca14"),
            hash(b"d7d6e50db66e0c7aafb8d00355f40bd531d470a3ca8a7c04da4f9255d515acfb"),
            hash(b"1338afbd82ba65494e5565ecdcf420ee35945e6bb146e54bccbda05d2e039791"),
            hash(b"ad269982a9bdcc24960f74f743fe239aa043e4a10e123272b0f6949027847954"),
            hash(b"7b5d449d50f32ca6d1a72ff58aa3c557f882358d32cc42259ea8614f7934bd05"),
            hash(b"45f1e1cfde073c475a485ef92d29e9b6177b6e3fa1de7df67e4e2652aa3c462c"),
            hash(b"59828756c8cdfdfcef9853acde04b5b420d372f1ca7b021213c2423ffa1469dd"),
            hash(b"bcb632c2d8f98ea72960400637dcead1438c7267366415c3836be9a71e8963fd"),
        ],
        hash(b"a124f146976b28bfedba9bf73d43e425fed9267ba60e7c23a523faba0bd11b38"),
    ),
    (
        "oteryn:quest.hidden_threats_quest",
        &[
            hash(b"c605417d3767ce5d646d6fd8f78eb64fc7c742ad26307249aa7205a49625e338"),
            hash(b"c00b18f773d09c9c372771d066dc470d85365aa27bfd2a1014583812cbccbd9b"),
            hash(b"0a77d4d929a6c9719f3f39eb88a631595cf515da121f64fe84b4dc5e63826823"),
            hash(b"997c65cbbd982f65e0099b3071c1708962f1d46ad2e37bfbc24f90997281dfed"),
            hash(b"fc793d42c84350dbba73b31753ca10b1b94ee114b9c8d67c4cac9b2ad848b0e7"),
            hash(b"1350f740c2780c11c22786cd4e0449c79f22fd245a47803bfaff85c44ecae2ae"),
            hash(b"53345631c518ee8fd246468ff795ad043eb77d207ad03daaf2b368d5789015f7"),
            hash(b"b9fd699bb82723a99a039107862d2920348cc47e38ab1dd99d8df5b3769965d9"),
            hash(b"5710efb6a47e898a65a81660b74ee74e469d1ea3e4ceb14b5e3a0b5029ed4215"),
            hash(b"8cbf1b4b88b94598939d9f352e1a4a69a2ba5ff13d0f5486d1f59eff8ccddc2d"),
            hash(b"824fcc3e8d29376814c064b0c6343db5319456c1c186318caa16f8449a453f12"),
        ],
        hash(b"0863383d44e7c74131f8d5424156f50df17a5ceb675eeb1be02218b288ebeadc"),
    ),
    (
        "oteryn:quest.hydra_egg_quest",
        &[
            hash(b"bf21c8ce62fd697df94d9125c787278372f8869088542a1a718ec7ba814f7dad"),
            hash(b"93f799015394a36b5896f0a862a860befd3f55a871778c5cfa2cbb6aeb1e35a7"),
            hash(b"9b29515bba98d0b790f0bfdaa746c0223b6ccccf328e369370bcd9993a5a5067"),
            hash(b"9e0e1064ccff5db2a9a3c7272e783136889ce9fe8721aacdae663e62f3423c99"),
            hash(b"4410ec5c32a8a9991447e39703d77bcfb0145cb09c909765ff571789938ac534"),
            hash(b"165adeeae7115720f419bc2aa99d6a942b24dd8b5a11af44016929c566335ada"),
            hash(b"afabcb21148c24b6a84813da6046861ff35ea512193f4dd2e775234193336049"),
        ],
        hash(b"78c0d4800b0094afbdf9a91ad444c7b8adbad1b2c6c5d8b3b48602837c69ffca"),
    ),
    (
        "oteryn:quest.ferumbras_ascension_quest",
        &[
            hash(b"06686721ac1b4e2b9b8055018119ac9ac486bcc30976644dcc2bc1cd021f9811"),
            hash(b"90726ffb2298ce404833c2a08cee70eb699dd600256d2edf6763858d83697685"),
            hash(b"05cdf255318dcd268826279656a9d779b53b88b8030efb2dc13600cb756d4b64"),
            hash(b"87fac0d81843121fb8e0555c18f0497b81c9ab311058ab0547683c50200d602c"),
            hash(b"046bc3914cc79a5d5f746a6dd56bab3ec6d0c77db6282615992fdb29cb3f3348"),
            hash(b"b78d634978fcca046d0a4a9863448f6bdce8768754d80fa14fd040260a7834dd"),
        ],
        hash(b"2c5453b476d04277fef5fcb9a0816805c039d3361eea70bf28ffda1846693c8f"),
    ),
    (
        "oteryn:quest.forgotten_knowledge_quest",
        &[
            hash(b"2a0e84e7f04fe4eafff4c3824de035250c01f8c9317feb2773ba11391678e00b"),
            hash(b"c5e817a79d863a9385dea01bfb4ab98f824a3a7d9be6656a1ee0ad1f19bbcb35"),
            hash(b"372df6f08b7588a01455b9f2ffef138d1bb52c70ee7b6ddde117babdd098b874"),
            hash(b"e3b513a35624575283574370f7be62eb6b4edd46954d77fd99d8338fa437e569"),
            hash(b"b8de92cb0d194f955969ae0e85b6ce47e18849b664742cebd656780cf6afad38"),
            hash(b"1f2491941e888347f73919d391ecfbedbea6f1fb03433ed7e7edb5b008176d23"),
        ],
        hash(b"4c96930f43d73c762b1f0e546c0f71aa11a24a504fbde760e03327b4e0db74ce"),
    ),
    (
        "oteryn:quest.grave_danger_quest",
        &[
            hash(b"5379f875a00f92429abeaed968037bf4e2778416c0b62a7b4f31cc2ac7c4d0aa"),
            hash(b"622fd073a0be3b84c8c78c515db66bf8d09f18ba6fadb009b469572269c7c096"),
        ],
        hash(b"286f3e0a4fdaff0ae2a14494889fe2d3492c5bbd4d8668a4f9b2cdaa48e37e7e"),
    ),
    (
        "oteryn:quest.grave_danger_quest",
        &[hash(
            b"69f766380778b7206750fdb0c61124d6b2efba92d7001502012d7fa1d040b1d0",
        )],
        PUBLISHED_GRAVE_DANGER_LIVE,
    ),
    (
        "oteryn:quest.grave_danger_quest",
        &[
            hash(b"5605ff5538ecb520e1a22c62ec5d64f8e6b7ffef27712b0c1492174247b9ee74"),
            hash(b"de3fd7c3b92165f4cc71c57effa8beaa5a90aa8a305d365f22fc51a82baad1e5"),
            hash(b"5e996375f282ec9d2d3b9d0fb17f46e3b19e8ae4443f6bf336299dedb010ad7d"),
        ],
        hash(b"9f3e5b1cb514d0aa4c8b475fd041fc61910791a53da2ce3ebd1cc80c9d31d2ab"),
    ),
    (
        "oteryn:quest.the_ice_islands_quest",
        &[hash(
            b"2466334ebc8e1fe1c21a06e2eaec92d0048920c28ed8cf1abf27ba2a0038167d",
        )],
        hash(b"b31bab1af5526262b5b0590cf13b3b99947d18fe0d40ff1fbbbf24189199fdea"),
    ),
    // Exact citation refreshes measured across the integrated content train.
    (
        "oteryn:quest.ferumbras_ascension_quest",
        &[hash(
            b"b5c362e28e85b935de1d7013a6cac14af4762101b7180dfa624a8199284ce206",
        )],
        hash(b"e612ad0f379b601b86b10d7c2b46d8e8ac7ce64c828cc12c5e4d686b9bf49620"),
    ),
    (
        "oteryn:quest.forgotten_knowledge_quest",
        &[hash(
            b"6aa8ccd292372accd01854fad5cc220d5af321308631bc1658172474dc005c20",
        )],
        hash(b"2c3b037dd13d724ea7b0371afc5756318c94d91285bfc6620aa710bb3005899f"),
    ),
    (
        "oteryn:quest.giant_smithhammer_quest",
        &[hash(
            b"2e4114efbc89f46d333713805e8ab81bc8c9205e5f213efbf96e71ab9e7aaaa3",
        )],
        hash(b"4242553063eec3ce16dd533c848c3f6a5cd716943b713c53a713ff42d8c80ce1"),
    ),
    (
        "oteryn:quest.goblin_merchant_quest",
        &[hash(
            b"649dd6d0d108b695fb8c2c6fa3f0f7bbefeb6228a572603ac1ddb32ecfb1e8bf",
        )],
        hash(b"52f34079861497c5687b92fc1b02cc48bfd5072af466ca308c112c946c25f81f"),
    ),
    (
        "oteryn:quest.grave_danger_quest",
        &[hash(
            b"5cbcc415adb5bb8ed3b77d0729ecff0950da236c0d0daa2aa03d4d1310241d18",
        )],
        hash(b"d1cdf6655a94744e239e485b9051f8e7d073be20d228a64e2fd1e19eda40840a"),
    ),
    (
        "oteryn:quest.hydra_egg_quest",
        &[hash(
            b"4ccce72f5f05d84b3b7d08d6700eeb4870b3eb1fe51035c9a365a5eb8ce157ef",
        )],
        hash(b"ae002be99d3196b93a23087bd929766e6160482da9fc3f36e054d91e5acc32e6"),
    ),
];

type GameplayPredecessors = (&'static str, DefinitionHash, &'static [DefinitionHash]);

const NEW_FRONTIER_SOURCE_PIN: DefinitionHash =
    hash(b"e947f2485cdf2c3094ba301e71369a208d98b017d08aef8e727705a992b6f0be");
const NEW_FRONTIER_AUTHORED_PIN: DefinitionHash =
    hash(b"e781a83be954e7111b4aec158c5d0f17e101441b1c242380cd8d432f55d019ef");
// The native route is also loaded without authored bindings. Keep its exact predecessor.
const NEW_FRONTIER_NATIVE_PREDECESSORS: GameplayPredecessors = (
    "oteryn:quest.the_new_frontier_quest",
    hash(b"61034f9e3365c8817d659f8ffa59a0f21e404d2a41dfc6099a48a5cedd71018f"),
    &[NEW_FRONTIER_SOURCE_PIN],
);

// These definitions changed gameplay. Preserve their exact saved pins and keep the
// new rules bound to the new hash. Existing absent-track defaults did not change.
const GAMEPLAY_PREDECESSORS: &[GameplayPredecessors] = &[
    (
        "oteryn:quest.adventurers_guild",
        hash(b"1ccc6851351d21c2d18943b259dbb017060607f9ab1421c4c36be4aadb4e9978"),
        &[hash(
            b"ae3b30a75e15f42fef0701e744bb27e91596aec92d794c9b969244d4668e7eae",
        )],
    ),
    (
        "oteryn:quest.adventures_of_galthen_quest",
        hash(b"97f814314ddbe2aa7ca889e426f88d64c8fc38f83ae1bb753ddd9c23f18923bf"),
        &[hash(
            b"610d37ee99a353c9a5cb00020f3d425e1e8fe72df4b585878968abfb559a6445",
        )],
    ),
    // Captured by quest_definition_hashes on main 9295760b3 and the reviewed route definitions.
    (
        "oteryn:quest.factions",
        hash(b"16a0ebe1810d54cdaa467424e4e3f0ab02fa4a4b8a728d7aea33706c9957d738"),
        &[hash(
            b"2bcb10953ae9889273762889d224e7f8f6d2d91fcb100c6a74a033227134c210",
        )],
    ),
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
    // Keep the Eclipse and Vampire Hunt saves, including saves from before the dust shortcut was removed.
    (
        "oteryn:quest.the_inquisition_quest",
        hash(b"072963573a04f538f1746de44e9e5f812079490e7dfbaab32ada96ab9a5717d8"),
        &[
            hash(b"6da59313e0183b50f65cb662b8b4b4eabb0b2a87cae044242c7c9587c1115ea8"),
            hash(b"8f43a82febdcd9b7fd7c9cd453b53a9b0a6b6de05273abd1d46c2414d562f48d"),
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
    (
        "oteryn:quest.tibia_tales",
        TIBIA_TALES_WITHOUT_ARITO_REFERRAL,
        &[TIBIA_TALES_TRAIN_SUCCESSOR],
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
        &[
            hash(b"292c00bd7c415835a998f3f249239c9b0b78d2f712bd8a347cb0759bfbdc81e8"),
            hash(b"5ad0bfbad1d580750ff170af788693571ddf9dcd7cd544a4a001c315e74f38b9"),
        ],
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

// Published by the production loader on main at 95e63dc1c, before Dread Lord.
pub(super) const BROTHERHOOD_BEFORE_DREAD_LORD: DefinitionHash =
    hash(b"4269816e523e2daa58821cadf770d45b98da24e8d149800499f151904dee2905");

// The pkq2 Tibia Tales and Cursed Crystal source cohorts extend this successor.
const TIBIA_TALES_TRAIN_SUCCESSOR: DefinitionHash =
    hash(b"08ddd97de427afb52741eacdef7781de95f438f5a86a88915d484b447e885db7");
const TIBIA_TALES_WITHOUT_ARITO_REFERRAL: DefinitionHash =
    hash(b"3e47096ea96ec29008c3666724672aeea3489bcd8e02ce3131c5be0a2837f05f");

const TIBIA_TALES_WITH_LIONS_ROCK: DefinitionHash =
    hash(b"97df564833d439d17ea75888b3cafc53c7551b489aa52f58b2dc4640f8c38362");

// Exact successors captured from the integrated production loader. The unit
// hashes remain valid predecessors for their own quest.
const CONTENT_TRAIN_SUCCESSORS: &[(&str, DefinitionHash, DefinitionHash)] = &[
    (
        "oteryn:quest.the_explorer_society_quest",
        hash(b"f51489994ea4f58e5ed3bd18de0a2120cb2852b8005a63eaee08ff86b4128fb1"),
        hash(b"f303676654b1838969e71f6327afe356168855d601e5fb1c46a1605f9c62ce8a"),
    ),
    (
        "oteryn:quest.authored.nomads_land_quest",
        hash(b"852db8606f6acf8a6d12e55c775a2f62a2eeac0594c6457af10ebb0f6b2e2325"),
        hash(b"fca492eb36c2318f4633a8aae3d68b6e32e5943b2717ddfee7933fe6857dfd4f"),
    ),
    // Umar's passage extends the published Factions definition.
    (
        "oteryn:quest.factions",
        hash(b"16a0ebe1810d54cdaa467424e4e3f0ab02fa4a4b8a728d7aea33706c9957d738"),
        hash(b"e0fd8a81ef902c0c37a31b2ad4e1cf4cd011d2e95bd45c1f4f0844c154b341b6"),
    ),
    (
        "oteryn:quest.ferumbras_ascension_quest",
        hash(b"69a1b3cfaa0d7f3b921da0ea7bb3bafd1de62a22e484e4c333541ba29c9b2a79"),
        hash(b"e612ad0f379b601b86b10d7c2b46d8e8ac7ce64c828cc12c5e4d686b9bf49620"),
    ),
    (
        "oteryn:quest.forgotten_knowledge_quest",
        hash(b"cbc0d3c2dfe847aa97daad45f4d322331ea90416ed774d5dddfc4cc7b4567281"),
        hash(b"2c3b037dd13d724ea7b0371afc5756318c94d91285bfc6620aa710bb3005899f"),
    ),
    (
        "oteryn:quest.giant_smithhammer_quest",
        hash(b"02d322ad993f1194f9b16fab996570c08b8d23b29d85d1cc39c19f1473e5c3e4"),
        hash(b"4242553063eec3ce16dd533c848c3f6a5cd716943b713c53a713ff42d8c80ce1"),
    ),
    (
        "oteryn:quest.goblin_merchant_quest",
        hash(b"75ed65085049b76c051ac4d92a8af8d59a3d015fa4cf1c0a973d0c5287f661f2"),
        hash(b"52f34079861497c5687b92fc1b02cc48bfd5072af466ca308c112c946c25f81f"),
    ),
    (
        "oteryn:quest.grave_danger_quest",
        hash(b"9f8957002452ecfe907a4be7430a5910db697ac88b74efe8dfaeab0dce568531"),
        hash(b"d1cdf6655a94744e239e485b9051f8e7d073be20d228a64e2fd1e19eda40840a"),
    ),
    (
        "oteryn:quest.grimvale_quest",
        hash(b"ec073be1d536c213bf81c914202b663a7f862ae7f78316fd97053173e3127823"),
        hash(b"a12ba535924a59362bc81815385cc2164cd29b0a666f71c94340ff98a5a3970d"),
    ),
    (
        "oteryn:quest.hydra_egg_quest",
        hash(b"c2247451fc15ba29360c581a59805205a6b8d3a606392342f1c99660cfdea6fa"),
        hash(b"ae002be99d3196b93a23087bd929766e6160482da9fc3f36e054d91e5acc32e6"),
    ),
    (
        "oteryn:quest.outfit_and_addon_quests",
        hash(b"f9785be106728cc026d6515c8e092eb08965d3d8e65bcb3a54f0816041a4d9dc"),
        hash(b"a9f70bb9fb2d498c4b82ff905d5b51dc774eb99e51d4701d651971dc15fb5831"),
    ),
    (
        "oteryn:quest.the_ape_city_quest",
        hash(b"3e8a82c701c48fb9e42db74edc6a0c3821b2f1e71dddf805b8d3553043dae44d"),
        hash(b"36a42b73db21b6e58303c3f613656d1c9d78f8bf741152d68dc14f9d71ecaa31"),
    ),
    (
        "oteryn:quest.the_new_frontier_quest",
        hash(b"61034f9e3365c8817d659f8ffa59a0f21e404d2a41dfc6099a48a5cedd71018f"),
        hash(b"6aba631a049bcb9213d7192bcc98fcaf7c94ef7ce643c4c741c7f738f3e0a233"),
    ),
    (
        "oteryn:quest.the_new_frontier_quest",
        hash(b"6aba631a049bcb9213d7192bcc98fcaf7c94ef7ce643c4c741c7f738f3e0a233"),
        NEW_FRONTIER_AUTHORED_PIN,
    ),
    (
        "oteryn:quest.barbarian_test",
        hash(b"18aa3b81e3b3462295a80b435696a7e9e66a739b0c26c199e6c0f1abbe6f89cb"),
        hash(b"b093eed3bfc25b81c1aff81ab93a2316ba426353ec608ff705f21b3bd410f1be"),
    ),
    (
        "oteryn:quest.battle_mage_outfits_quest",
        hash(b"a19162baec7e41dd0e4c5b3332819dce4dd6b8f4a2bb300cd5f14ab58c4aff90"),
        hash(b"7e24a445ca8fc5bc5e3443dd38c23bafa6f01b4815b666a652291f36305a1edf"),
    ),
    (
        "oteryn:quest.ferumbras_ascension_quest",
        hash(b"2c5453b476d04277fef5fcb9a0816805c039d3361eea70bf28ffda1846693c8f"),
        hash(b"e612ad0f379b601b86b10d7c2b46d8e8ac7ce64c828cc12c5e4d686b9bf49620"),
    ),
    (
        "oteryn:quest.forgotten_knowledge_quest",
        hash(b"4c96930f43d73c762b1f0e546c0f71aa11a24a504fbde760e03327b4e0db74ce"),
        hash(b"2c3b037dd13d724ea7b0371afc5756318c94d91285bfc6620aa710bb3005899f"),
    ),
    (
        "oteryn:quest.nightmare_outfits_quest",
        hash(b"dc94a505cae469502aacecb2886cf794f483a0a5082583c9c9fdf505f592bca2"),
        hash(b"63d6c648bcbc087f4277361c686926870b2df2c73d6570d3dd8ea13fbe7cabc1"),
    ),
    (
        "oteryn:quest.the_ice_islands_quest",
        hash(b"fdaedce30df4cf940d0022e341e5c0e9be4025a17564ae8f0fed960e350cfe4e"),
        hash(b"026ddacf9e11904f68123efbfaf0f8b19337ad6fd717809e2bf18e426d30a0ae"),
    ),
    (
        "oteryn:quest.the_ice_islands_quest",
        hash(b"b31bab1af5526262b5b0590cf13b3b99947d18fe0d40ff1fbbbf24189199fdea"),
        hash(b"026ddacf9e11904f68123efbfaf0f8b19337ad6fd717809e2bf18e426d30a0ae"),
    ),
    (
        "oteryn:quest.the_secret_library_quest",
        hash(b"22db0b0b39a6a299def5ea0466fd0def68f98d9ccba6e209ae2108ecefec0a6b"),
        hash(b"e5f4033817cca4c9f267379f185fa7c6ed2f2ec755544abc132221dc9b49b0a2"),
    ),
    (
        "oteryn:quest.tibia_tales",
        TIBIA_TALES_WITHOUT_ARITO_REFERRAL,
        TIBIA_TALES_WITH_LIONS_ROCK,
    ),
];

fn is_content_train_successor(
    quest: &str,
    current: DefinitionHash,
    active: Option<DefinitionHash>,
) -> bool {
    CONTENT_TRAIN_SUCCESSORS
        .iter()
        .any(|&(owner, unit, train)| owner == quest && unit == current && active == Some(train))
}

fn is_successor(quest: &str, current: DefinitionHash, active: Option<DefinitionHash>) -> bool {
    active == Some(current)
        || (quest == "oteryn:quest.tibia_tales"
            && (active == Some(TIBIA_TALES_WITHOUT_ARITO_REFERRAL)
                || active == Some(TIBIA_TALES_WITH_LIONS_ROCK)
                || (current != TIBIA_TALES_WITHOUT_ARITO_REFERRAL
                    && active == Some(TIBIA_TALES_TRAIN_SUCCESSOR))))
        || is_content_train_successor(quest, current, active)
}

// Canonical loader output for the stable Farmine topic and its prior routes.
const AUTHORED_GAMEPLAY_PREDECESSORS: &[(&str, DefinitionHash, &[DefinitionHash])] = &[(
    "oteryn:quest.the_new_frontier_quest",
    NEW_FRONTIER_AUTHORED_PIN,
    &[
        hash(b"f419510de4813df96a8270dcd2f648101981daa98ef1fe46fade5dab4d636984"),
        hash(b"78161561d1fda5955ac5783020a521447d1924558bd69fad889b1bf8ba39a4d6"),
        hash(b"af6dcf481c46b3e06577bd6ba47cb37fa06aca9e37b48b21c36b6159b3b4d462"),
        hash(b"cb5d7d7008e10821fc65677dec2475d278235b7f5c82ac1f99a2e615873b9e1c"),
    ],
)];

pub(super) fn published_hash(quest: &str, current: DefinitionHash) -> DefinitionHash {
    CITATION_REFRESHES
        .iter()
        .find(|(owner, refreshed, _)| *owner == quest && refreshed.contains(&current))
        .map_or(current, |(_, _, published)| *published)
}

// The exact replacement NPC snapshots have the same absent-track defaults as the
// earlier live snapshots. Preserve both under the measured active definition.
const NPC_SNAPSHOT_PREDECESSORS: &[(&str, DefinitionHash, DefinitionHash, DefinitionHash)] = &[
    (
        "oteryn:quest.grave_danger_quest",
        hash(b"d1cdf6655a94744e239e485b9051f8e7d073be20d228a64e2fd1e19eda40840a"),
        GRAVE_DANGER_PRE_ZELOS,
        hash(b"62b1a9c525aea095845afd89a9bfa0d478b729b851dc86d006eacb3b5b405e2f"),
    ),
    (
        "oteryn:quest.hero_of_rathleton_quest",
        hash(b"a124f146976b28bfedba9bf73d43e425fed9267ba60e7c23a523faba0bd11b38"),
        hash(b"cab96523f101dda9dea22ea0b0c6b3a88054c5c5f71949e5f93f205fac1495f6"),
        hash(b"aec1fbb66d6cbb19a6d3d9fa5c3a51999d1f159c8d1cee8d427ad3075adea0a5"),
    ),
    (
        "oteryn:quest.tibia_tales",
        TIBIA_TALES_WITH_LIONS_ROCK,
        hash(b"26e770c01a6deddd293cde040494caabee10def864f616bac7e8afa01b7d5dde"),
        hash(b"52cc0637ab01ee36326fbbd0f9f89aa45579e78d280cfea01ca44481eb27d64c"),
    ),
];

fn retain_npc_snapshot_predecessors(catalogue: &mut QuestStateCatalogue) {
    for &(quest, active, saved, replacement) in NPC_SNAPSHOT_PREDECESSORS {
        if catalogue.definition_hash(quest) != Some(active) {
            continue;
        }
        let Some(defaults) =
            super::npc_rules::legacy_defaults(catalogue, quest, &replacement).cloned()
        else {
            continue;
        };
        let predecessors = catalogue.predecessors.entry(quest.into()).or_default();
        if let Some(previous) = predecessors
            .iter_mut()
            .find(|previous| previous.hash == saved)
        {
            previous.defaults = defaults;
        } else {
            predecessors.push(NpcDefinitionPredecessor {
                hash: saved,
                defaults,
            });
        }
    }
}

pub(super) fn retain_predecessors(catalogue: &mut QuestStateCatalogue) {
    retain(
        catalogue,
        GAMEPLAY_PREDECESSORS
            .iter()
            .chain([&NEW_FRONTIER_NATIVE_PREDECESSORS]),
    );
    retain_npc_snapshot_predecessors(catalogue);
}

pub(super) fn retain_authored_predecessors(catalogue: &mut QuestStateCatalogue) {
    retain(catalogue, AUTHORED_GAMEPLAY_PREDECESSORS);
}

fn retain<'a>(
    catalogue: &mut QuestStateCatalogue,
    pins: impl IntoIterator<Item = &'a GameplayPredecessors>,
) {
    for &(quest, current, previous) in pins {
        let active = catalogue.definition_hash(quest);
        if is_successor(quest, current, active) {
            let unit_pin = is_content_train_successor(quest, current, active).then_some(current);
            let predecessors = catalogue.predecessors.entry(quest.into()).or_default();
            for hash in previous.iter().copied().chain(unit_pin) {
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
    fn nomads_land_accepts_main_save_under_the_composed_tibia_tales_pin() -> Result<(), String> {
        let loaded =
            loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        let mut catalogue = loaded.catalogue().clone();
        let quest = "oteryn:quest.tibia_tales";
        let main = hash(b"8d75517e41359a25abea653c8fd44fa5b4d2768c1470d9acaa9442b73f543065");
        assert_eq!(
            catalogue.definition_hash(quest),
            Some(TIBIA_TALES_WITH_LIONS_ROCK)
        );
        for (successor, accepted) in [
            (TIBIA_TALES_TRAIN_SUCCESSOR, true),
            (TIBIA_TALES_WITHOUT_ARITO_REFERRAL, true),
            (TIBIA_TALES_WITH_LIONS_ROCK, true),
            ([0x5a; 32], false),
        ] {
            catalogue.predecessors.remove(quest);
            catalogue.hashes.insert(quest.into(), successor);
            retain_predecessors(&mut catalogue);
            assert_eq!(
                npc_rules::legacy_defaults(&catalogue, quest, &main).is_some(),
                accepted
            );
            assert_eq!(
                npc_rules::legacy_defaults(&catalogue, quest, &TIBIA_TALES_TRAIN_SUCCESSOR)
                    .is_some(),
                matches!(
                    successor,
                    TIBIA_TALES_WITHOUT_ARITO_REFERRAL | TIBIA_TALES_WITH_LIONS_ROCK
                )
            );
            assert_eq!(
                npc_rules::legacy_defaults(&catalogue, quest, &TIBIA_TALES_WITHOUT_ARITO_REFERRAL)
                    .is_some(),
                successor == TIBIA_TALES_WITH_LIONS_ROCK
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
            let active = catalogue.definition_hash(quest);
            if active != Some(published)
                && (quest == "oteryn:quest.grave_danger_quest"
                    || CONTENT_TRAIN_SUCCESSORS
                        .iter()
                        .any(|&(owner, _, train)| owner == quest && active == Some(train)))
            {
                // These quests have since gained gameplay changes. Each citation
                // variant still resolves to the exact published predecessor.
                assert!(
                    npc_rules::legacy_defaults(catalogue, quest, &published).is_some(),
                    "{quest}"
                );
            } else {
                assert_eq!(active, Some(published), "{quest}");
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
        for &(quest, current, previous) in GAMEPLAY_PREDECESSORS
            .iter()
            .chain(AUTHORED_GAMEPLAY_PREDECESSORS)
        {
            let active = catalogue.definition_hash(quest);
            if AUTHORED_GAMEPLAY_PREDECESSORS
                .iter()
                .any(|&(owner, pin, _)| owner == quest && pin == current)
            {
                assert_eq!(active, Some(current), "{quest}");
            } else {
                assert!(is_successor(quest, current, active), "{quest}: {active:?}");
            }
            let unit_pin = is_content_train_successor(quest, current, active).then_some(current);
            for pin in previous.iter().copied().chain(unit_pin) {
                let actual = npc_rules::legacy_defaults(catalogue, quest, &pin);
                let defaults = if pin == GRAVE_DANGER_PRE_ZELOS {
                    catalogue
                        .declared_tracks()
                        .filter(|track| track.quest == quest)
                        .map(|track| (track.key.clone(), track.initial))
                        .collect()
                } else {
                    Default::default()
                };
                if pin == super::super::reward_callbacks::OUTFIT_REWARDS_BEFORE_CITIZEN {
                    let historical = actual.ok_or("published Outfit defaults")?;
                    assert_eq!(historical.len(), 52);
                    assert_eq!(historical.get("oteryn:quest-progress/navigator"), Some(&-1));
                } else {
                    assert_eq!(actual, Some(&defaults), "{quest}");
                }
                let mut foreign = pin;
                foreign[0] ^= 1;
                assert!(npc_rules::legacy_defaults(catalogue, quest, &foreign).is_none());
            }
        }
        for &(quest, active, saved, replacement) in NPC_SNAPSHOT_PREDECESSORS {
            assert_eq!(catalogue.definition_hash(quest), Some(active), "{quest}");
            let defaults = npc_rules::legacy_defaults(catalogue, quest, &replacement)
                .ok_or("replacement NPC snapshot defaults")?;
            assert!(!defaults.is_empty(), "{quest}");
            assert_eq!(
                npc_rules::legacy_defaults(catalogue, quest, &saved),
                Some(defaults),
                "{quest}"
            );
            let mut foreign = saved;
            foreign[0] ^= 1;
            assert!(npc_rules::legacy_defaults(catalogue, quest, &foreign).is_none());
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
