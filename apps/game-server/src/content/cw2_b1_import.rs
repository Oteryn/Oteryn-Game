//! Exact adapters for owner-approved CW2-B1 native Item bindings.
//!
//! This adapter accepts only the protected B1 catalogue bytes. The source observations remain
//! import provenance and explicit losses; the native Item values are Oteryn-authored and use the
//! existing project Item record and Reference lowering path.

use super::{
    CandidateDisposition, CandidateValue, DefinitionIdentityDocument, ImportBatch, ImportCandidate,
    ImportCandidateFamily, ImportCandidateOperation, ItemStackDocument, NamedCandidateField,
    NativeItemBindingDisposition, NativeItemBindingDocument, ProjectReferenceRecord,
    ProjectionDocument, ReimportDecision, ReimportFieldState, world_project_sha256,
};
use std::{
    collections::BTreeSet,
    fmt::{self, Display, Formatter},
};

pub const PROTECTED_CW2_B1_EVIDENCE_BYTES: usize = 16_877_870;
pub const PROTECTED_CW2_B1_EVIDENCE_BLOB: &str = "2f0121f3ea6586477b4535840b9a1f1bc28c677c";
pub const PROTECTED_CW2_B1_EVIDENCE_SHA256: &str =
    "7836c78cad130a5c404f648e76e0823f53ae6a34c6952b9b88c8bed2e50d96a7";
pub const PROTECTED_CW2_B1_PRODUCT_SHA256: &str =
    "d773076b576599b6ced7eb53e262cdc6363515d842da3e8518b610e2106db0fc";
pub const PROTECTED_CW2_B1_MAPPER_BLOB: &str = "904d62e1277ceae76434f75bca104686a7303ff2";
pub const PROTECTED_CW2_B1_MAPPER_SHA256: &str =
    "320ce69f516de2a6e3cec669493ec59a6f3103f405e624478cf2a76acd3d01b6";
pub const PROTECTED_CW2_B1_NODE_SHA256: &str =
    "b7c5c457cdccf047b251313e27cf283442a7346ddc556eaece8c2fdc30d655c3";
pub const PROTECTED_CW2_B1_FIELD_PROFILE_SHA256: &str =
    "11da415530a0c8678cdb3d05c3205f21f3cc708f13a7342674538f97b911472e";

pub const CW2_B1_SOURCE_REPOSITORY: &str = "zimbadev/crystalserver";
pub const CW2_B1_SOURCE_REVISION: &str = "ff7ede593c69d4c658b382c97443e8155926924a";
pub const CW2_B1_SOURCE_PATH: &str = "data/items/items.xml";
pub const CW2_B1_SOURCE_BLOB: &str = "0b1dc3ba1a49094d9c83b90ab399bd2a9dd7a17f";
pub const CW2_B1_SOURCE_SHA256: &str =
    "c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb";
pub const CW2_B1_SOURCE_BYTES: i64 = 3_819_874;
pub const CW2_B1_SOURCE_ITEM_ID: u64 = 2_876;

pub const CW2_B1_VASE_KEY: &str = "oteryn:item.decor.vase";
pub const CW2_B1_VASE_REVISION: &str = "definition-r1";
pub const CW2_B1_NATIVE_ITEM_BATCH_REVISION: &str = "definition-r1";
pub const CW2_B1_NATIVE_ITEM_BATCH_COUNT: usize = 64;
pub const CW2_B1_VASE_B3_ROW: &str = "definition:monster:0108:loot:0006";
pub const CW2_B1_VASE_B3_ROW_SHA256: &str =
    "f5d87a09806776c70a799abb5b9eb657ed66b93346d943053ba3fad6500b2712";

const CATALOG_SCHEMA: &str = "OTERYN_CW2_ITEM_IDENTITY_CATALOG_SOURCE_BATCH/v1";
const MAPPER_PROFILE: &str = "OTERYN_CW2_ITEM_IDENTITY_CATALOG_MAPPER/v1";
const SOURCE_CANDIDATE_ID: &str = "crystal:item:2876";

#[derive(Debug, Clone, Copy)]
struct NativeItemSpec {
    source_item_id: u64,
    source_label: &'static str,
    item_class: &'static str,
    native_key: &'static str,
    stack_capable: bool,
    node_sha256: &'static str,
    field_profile_sha256: &'static str,
}

const NATIVE_ITEM_BATCH: [NativeItemSpec; CW2_B1_NATIVE_ITEM_BATCH_COUNT] = [
    NativeItemSpec {
        source_item_id: 3035,
        source_label: "platinum coin",
        item_class: "currency_stackable",
        native_key: "oteryn:item.currency.platinum_coin",
        stack_capable: true,
        node_sha256: "2a2400eb87de8067677d3d00b82e973135b12fe3e29348040dbc1724d3da12d8",
        field_profile_sha256: "74d31a24125469ba363fdb5833bf1097bf5280900531d7657549ddc5c281b2e0",
    },
    NativeItemSpec {
        source_item_id: 3043,
        source_label: "crystal coin",
        item_class: "currency_stackable",
        native_key: "oteryn:item.currency.crystal_coin",
        stack_capable: true,
        node_sha256: "a5538fe905de344f0df8e9994859ab7d2cdc0cdc40df51b3f584a95fe42501c1",
        field_profile_sha256: "74d31a24125469ba363fdb5833bf1097bf5280900531d7657549ddc5c281b2e0",
    },
    NativeItemSpec {
        source_item_id: 3492,
        source_label: "worm",
        item_class: "currency_stackable",
        native_key: "oteryn:item.material.worm",
        stack_capable: true,
        node_sha256: "85c932f8c74f5147c4d834ccb0811a0c959035749a8ef1b706c8de6347419ba9",
        field_profile_sha256: "74d31a24125469ba363fdb5833bf1097bf5280900531d7657549ddc5c281b2e0",
    },
    NativeItemSpec {
        source_item_id: 675,
        source_label: "small enchanted sapphire",
        item_class: "currency_stackable",
        native_key: "oteryn:item.material.gem.small_enchanted_sapphire",
        stack_capable: true,
        node_sha256: "2fa3ffba601d8a3dad55a6cd545d403ff295d5a5c693d2b722c95f5387507765",
        field_profile_sha256: "74d31a24125469ba363fdb5833bf1097bf5280900531d7657549ddc5c281b2e0",
    },
    NativeItemSpec {
        source_item_id: 676,
        source_label: "small enchanted ruby",
        item_class: "currency_stackable",
        native_key: "oteryn:item.material.gem.small_enchanted_ruby",
        stack_capable: true,
        node_sha256: "36112a05b9c2be382ba24e97234e888bb54c4cf0a6ac1a414bc0885aa45a89d5",
        field_profile_sha256: "74d31a24125469ba363fdb5833bf1097bf5280900531d7657549ddc5c281b2e0",
    },
    NativeItemSpec {
        source_item_id: 677,
        source_label: "small enchanted emerald",
        item_class: "currency_stackable",
        native_key: "oteryn:item.material.gem.small_enchanted_emerald",
        stack_capable: true,
        node_sha256: "34428b6f050f90adb424e4bd6898317f0f58c449ea971184bb42b59dfb335ea8",
        field_profile_sha256: "74d31a24125469ba363fdb5833bf1097bf5280900531d7657549ddc5c281b2e0",
    },
    NativeItemSpec {
        source_item_id: 678,
        source_label: "small enchanted amethyst",
        item_class: "currency_stackable",
        native_key: "oteryn:item.material.gem.small_enchanted_amethyst",
        stack_capable: true,
        node_sha256: "342a7ebfd1e74252c3e1f5132cce2909dd85c5a5ff057beec41fb7a56841a97c",
        field_profile_sha256: "74d31a24125469ba363fdb5833bf1097bf5280900531d7657549ddc5c281b2e0",
    },
    NativeItemSpec {
        source_item_id: 3447,
        source_label: "arrow",
        item_class: "currency_stackable",
        native_key: "oteryn:item.ammunition.arrow",
        stack_capable: true,
        node_sha256: "f2b3c59423df63a433fda13ca945ee7510f1664b03c9f4d45de75136b72030bd",
        field_profile_sha256: "f58edc73b19db6864b4d88c89ead00d0dc3b842368746a3e0bf310d9686cb299",
    },
    NativeItemSpec {
        source_item_id: 3267,
        source_label: "dagger",
        item_class: "weapon",
        native_key: "oteryn:item.weapon.blade.dagger",
        stack_capable: false,
        node_sha256: "755ce623d930a572152886a8393df358995b608d6e51c9cb75ef5e9a93702710",
        field_profile_sha256: "880a3995c315e61a2d0c199d16a8372fed5bc739c39d009227cd13fcf14cb48c",
    },
    NativeItemSpec {
        source_item_id: 3280,
        source_label: "fire sword",
        item_class: "weapon",
        native_key: "oteryn:item.weapon.sword.fire",
        stack_capable: false,
        node_sha256: "9a0c6c7b8b82d0c1ddb02d70da8a63db9aa25f5bf1063a157b3a491ce72e55fa",
        field_profile_sha256: "00c9f780f6e879239cf4c0639d3f9cd768a5b3255a857e78a1e9a688ab22abf9",
    },
    NativeItemSpec {
        source_item_id: 3295,
        source_label: "bright sword",
        item_class: "weapon",
        native_key: "oteryn:item.weapon.sword.bright",
        stack_capable: false,
        node_sha256: "8e7bfd90afcc484cdd01f54ea36440308b428bd258ba0eb166143c663068f5a3",
        field_profile_sha256: "73f000129a1cc9400b4947bb8fb4e74f9ab5d1b98654122b99b013e11680a5a4",
    },
    NativeItemSpec {
        source_item_id: 3268,
        source_label: "hand axe",
        item_class: "weapon",
        native_key: "oteryn:item.weapon.axe.hand",
        stack_capable: false,
        node_sha256: "242953c6602a914269736e904dd1822b9537a4cd32f62fafa94eccf930705b94",
        field_profile_sha256: "880a3995c315e61a2d0c199d16a8372fed5bc739c39d009227cd13fcf14cb48c",
    },
    NativeItemSpec {
        source_item_id: 3320,
        source_label: "fire axe",
        item_class: "weapon",
        native_key: "oteryn:item.weapon.axe.fire",
        stack_capable: false,
        node_sha256: "6312557dbf49bac7366925126cb6b331655456df9aa41ce8dee4043707969fb8",
        field_profile_sha256: "00c9f780f6e879239cf4c0639d3f9cd768a5b3255a857e78a1e9a688ab22abf9",
    },
    NativeItemSpec {
        source_item_id: 3318,
        source_label: "knight axe",
        item_class: "weapon",
        native_key: "oteryn:item.weapon.axe.knight",
        stack_capable: false,
        node_sha256: "5579e21fef40b215aeb5438d6bef0d950cb8098e4015afd852f0be647f710b07",
        field_profile_sha256: "9ddbf315c0145fa263a931c85d03d708245953795a41731c0d82b191e874b593",
    },
    NativeItemSpec {
        source_item_id: 3286,
        source_label: "mace",
        item_class: "weapon",
        native_key: "oteryn:item.weapon.club.mace",
        stack_capable: false,
        node_sha256: "3819ae74118f58055cf925ff0a5616ae4e971c1fccfff96093a18c4ed0514eb8",
        field_profile_sha256: "880a3995c315e61a2d0c199d16a8372fed5bc739c39d009227cd13fcf14cb48c",
    },
    NativeItemSpec {
        source_item_id: 3324,
        source_label: "skull staff",
        item_class: "weapon",
        native_key: "oteryn:item.weapon.club.skull_staff",
        stack_capable: false,
        node_sha256: "14e4fa7532a9afbc2b8ef04a7176c5f740cd14c4ed4143baa4cc2140a0120b5c",
        field_profile_sha256: "73f000129a1cc9400b4947bb8fb4e74f9ab5d1b98654122b99b013e11680a5a4",
    },
    NativeItemSpec {
        source_item_id: 3305,
        source_label: "battle hammer",
        item_class: "weapon",
        native_key: "oteryn:item.weapon.club.battle_hammer",
        stack_capable: false,
        node_sha256: "d587b5b105cdc67bcfe9d40e92f8423893e68ca5aad52089bf3d78f5921fe65f",
        field_profile_sha256: "880a3995c315e61a2d0c199d16a8372fed5bc739c39d009227cd13fcf14cb48c",
    },
    NativeItemSpec {
        source_item_id: 3349,
        source_label: "crossbow",
        item_class: "weapon",
        native_key: "oteryn:item.weapon.ranged.crossbow",
        stack_capable: false,
        node_sha256: "f2415dbc73fdb717f07636e0806686bf7ce7f3aba82de720b917742993a8f760",
        field_profile_sha256: "225f761f198c6bdfd74a6cf483861e276dec4e4d3bd8feebdf017160318250cf",
    },
    NativeItemSpec {
        source_item_id: 3350,
        source_label: "bow",
        item_class: "weapon",
        native_key: "oteryn:item.weapon.ranged.bow",
        stack_capable: false,
        node_sha256: "04cf9514fa182b56f2240f557444b46b9ec71028283174caec102f333b18e076",
        field_profile_sha256: "3f213c9d451c38cdd984ae9e3f3790cafd835a7ab7e3aad0fa7b9d1f3e99ba12",
    },
    NativeItemSpec {
        source_item_id: 3066,
        source_label: "snakebite rod",
        item_class: "weapon",
        native_key: "oteryn:item.weapon.wand.snakebite_rod",
        stack_capable: false,
        node_sha256: "9345fbf7ad0a54f912e097136375a957e9291365355836eb2952bf8d600078e1",
        field_profile_sha256: "f4fc7ac67bb34b3b6c222cb738286258efb6113cf49da3d412ff1ab211617452",
    },
    NativeItemSpec {
        source_item_id: 3351,
        source_label: "steel helmet",
        item_class: "equipment",
        native_key: "oteryn:item.equipment.steel_helmet",
        stack_capable: false,
        node_sha256: "b7ac518e8a97c0efb4e52ec4c1daf77dd38944da841ae50eddadb45119825dda",
        field_profile_sha256: "83c2e166d10682deea3f6eda02904f6a9ced4da21b0486545360aa76d0e48823",
    },
    NativeItemSpec {
        source_item_id: 3357,
        source_label: "plate armor",
        item_class: "equipment",
        native_key: "oteryn:item.armor.plate_armor",
        stack_capable: false,
        node_sha256: "006dd5aac4ea304f1cfbede93b9b757e9697b6d89bafc533fab5fe5d397a6793",
        field_profile_sha256: "83c2e166d10682deea3f6eda02904f6a9ced4da21b0486545360aa76d0e48823",
    },
    NativeItemSpec {
        source_item_id: 3557,
        source_label: "plate legs",
        item_class: "equipment",
        native_key: "oteryn:item.equipment.plate_legs",
        stack_capable: false,
        node_sha256: "63249035017fa13d9ac598bc41babf466e0bc45b9c185f173f36131099423ee1",
        field_profile_sha256: "83c2e166d10682deea3f6eda02904f6a9ced4da21b0486545360aa76d0e48823",
    },
    NativeItemSpec {
        source_item_id: 3409,
        source_label: "steel shield",
        item_class: "equipment",
        native_key: "oteryn:item.equipment.steel_shield",
        stack_capable: false,
        node_sha256: "6dd9ed793bc00b52b980402ad0ae7c76fe37a761592c4325128714914c368082",
        field_profile_sha256: "1151295383ab15474f226cac81b4d6f5242094e371fcb06b6a3402fc0006b04d",
    },
    NativeItemSpec {
        source_item_id: 3412,
        source_label: "wooden shield",
        item_class: "equipment",
        native_key: "oteryn:item.equipment.wooden_shield",
        stack_capable: false,
        node_sha256: "a7fcf4b6cbd73783256bd0d58767af814668ac4138d36b6e083d9b8b971fd371",
        field_profile_sha256: "127dca5e2e2ecb8b26898d5f19e4957156ed8268423288c429e690ae69bc312d",
    },
    NativeItemSpec {
        source_item_id: 3055,
        source_label: "platinum amulet",
        item_class: "equipment",
        native_key: "oteryn:item.equipment.platinum_amulet",
        stack_capable: false,
        node_sha256: "8e22605db18521528f2aff0e289ebfd7350a7b5ac676e0b09f9ccc8fb8a4e6a4",
        field_profile_sha256: "ff3a4d3b8cdb492ccba844425f2a28a9e9cc361417170d9986af4be07e691834",
    },
    NativeItemSpec {
        source_item_id: 812,
        source_label: "terra legs",
        item_class: "equipment",
        native_key: "oteryn:item.equipment.terra_legs",
        stack_capable: false,
        node_sha256: "9f3f20696e67671ed262557fd3dce4d6c82b5d5a5f9d72561174fb247b2f3b21",
        field_profile_sha256: "8c7df30bd6dee7644067aee2b2090162c428dbd84fb03ab59eef9ddc135566ed",
    },
    NativeItemSpec {
        source_item_id: 824,
        source_label: "glacier robe",
        item_class: "equipment",
        native_key: "oteryn:item.equipment.glacier_robe",
        stack_capable: false,
        node_sha256: "4a1d051d07a643a181e3ddf411d2b8488daf6c7f65b909eb1f0e3c739c84e4ae",
        field_profile_sha256: "e856903cf287582b2a100f4e9bbc75be584b243ffd45e42e342718339c882286",
    },
    NativeItemSpec {
        source_item_id: 827,
        source_label: "magma monocle",
        item_class: "equipment",
        native_key: "oteryn:item.equipment.magma_monocle",
        stack_capable: false,
        node_sha256: "c9a4ddc01bf25a279e3c533c5d31f96617eb9a8003531bf4ad0816fcc88065e5",
        field_profile_sha256: "13c9be2c1b84f9b7199d50c4f43b8597c1cc9a35fd58a8182ff5cab02b6bded3",
    },
    NativeItemSpec {
        source_item_id: 3391,
        source_label: "crusader helmet",
        item_class: "equipment",
        native_key: "oteryn:item.equipment.crusader_helmet",
        stack_capable: false,
        node_sha256: "c5d4953ee231b39adf03d2340c4cd9a2c37fd1b930c5ed80e983c1a87944654c",
        field_profile_sha256: "f59e1b9f71ca5347cca5d5aa9c18ca05c1e31d6ecc8c2197e1a596a212613a72",
    },
    NativeItemSpec {
        source_item_id: 3370,
        source_label: "knight armor",
        item_class: "equipment",
        native_key: "oteryn:item.equipment.knight_armor",
        stack_capable: false,
        node_sha256: "a087ba076bcbc6b3f967d339b9d00ddcf99d352214bb78f1d7daec8e9cfff0da",
        field_profile_sha256: "f59e1b9f71ca5347cca5d5aa9c18ca05c1e31d6ecc8c2197e1a596a212613a72",
    },
    NativeItemSpec {
        source_item_id: 3385,
        source_label: "crown helmet",
        item_class: "equipment",
        native_key: "oteryn:item.equipment.crown_helmet",
        stack_capable: false,
        node_sha256: "3f773e34581386ee337a89049a586d49a5135c8c835572d4465abe85820e1c4f",
        field_profile_sha256: "f59e1b9f71ca5347cca5d5aa9c18ca05c1e31d6ecc8c2197e1a596a212613a72",
    },
    NativeItemSpec {
        source_item_id: 2871,
        source_label: "golden backpack",
        item_class: "container",
        native_key: "oteryn:item.container.golden_backpack",
        stack_capable: false,
        node_sha256: "d8fe87b4bd22a5244ba83289b406ca2a6930d2482c963d9e6cc6bc6778534d2f",
        field_profile_sha256: "612572db278dc67cc5abedaadef3826f40701bb8a0e409030b01adac6e09c7fa",
    },
    NativeItemSpec {
        source_item_id: 5926,
        source_label: "pirate backpack",
        item_class: "container",
        native_key: "oteryn:item.container.pirate_backpack",
        stack_capable: false,
        node_sha256: "508b2a6740ce10e609c6452676c011931366fd278659b89208642f61e026011d",
        field_profile_sha256: "612572db278dc67cc5abedaadef3826f40701bb8a0e409030b01adac6e09c7fa",
    },
    NativeItemSpec {
        source_item_id: 5927,
        source_label: "pirate bag",
        item_class: "container",
        native_key: "oteryn:item.container.pirate_bag",
        stack_capable: false,
        node_sha256: "133ea978d3ea2b922a77f4f6255a2c058570e550c45e9219a8cd540740250dc4",
        field_profile_sha256: "93cd457c45acf4b2a6d5dc4044e42c2094715f09db1e484629d011fe5aca5172",
    },
    NativeItemSpec {
        source_item_id: 7343,
        source_label: "fur bag",
        item_class: "container",
        native_key: "oteryn:item.container.fur_bag",
        stack_capable: false,
        node_sha256: "da1c5482ade8e41fe0efbe3a884de0e6ff792863efc0ae06f87f8be55ab15306",
        field_profile_sha256: "93cd457c45acf4b2a6d5dc4044e42c2094715f09db1e484629d011fe5aca5172",
    },
    NativeItemSpec {
        source_item_id: 9604,
        source_label: "moon backpack",
        item_class: "container",
        native_key: "oteryn:item.container.moon_backpack",
        stack_capable: false,
        node_sha256: "8dffabf589c66a3cf8cda4cac92e5b23c3ed91cc1d50ec4dd646c5f2df8d8afc",
        field_profile_sha256: "612572db278dc67cc5abedaadef3826f40701bb8a0e409030b01adac6e09c7fa",
    },
    NativeItemSpec {
        source_item_id: 14248,
        source_label: "deepling backpack",
        item_class: "container",
        native_key: "oteryn:item.container.deepling_backpack",
        stack_capable: false,
        node_sha256: "7a496de66726bf3e6e469372cfed2506bf08a7c8ac4d0958e6d95343a737f968",
        field_profile_sha256: "612572db278dc67cc5abedaadef3826f40701bb8a0e409030b01adac6e09c7fa",
    },
    NativeItemSpec {
        source_item_id: 24393,
        source_label: "pillow backpack",
        item_class: "container",
        native_key: "oteryn:item.container.pillow_backpack",
        stack_capable: false,
        node_sha256: "733323428cf4d2461b64136380355b719cee9e46d14acf960b82fedb20587754",
        field_profile_sha256: "612572db278dc67cc5abedaadef3826f40701bb8a0e409030b01adac6e09c7fa",
    },
    NativeItemSpec {
        source_item_id: 28571,
        source_label: "book backpack",
        item_class: "container",
        native_key: "oteryn:item.container.book_backpack",
        stack_capable: false,
        node_sha256: "235d4b3daaac7b30759bb8108c49fbcd163a1c2d134fe036e68f16c17b0a63cf",
        field_profile_sha256: "612572db278dc67cc5abedaadef3826f40701bb8a0e409030b01adac6e09c7fa",
    },
    NativeItemSpec {
        source_item_id: 237,
        source_label: "strong mana potion",
        item_class: "consumable_charges",
        native_key: "oteryn:item.consumable.potion.strong_mana",
        stack_capable: true,
        node_sha256: "8107df439e0b1965dc6a7ba8b8590581fa4e87c30d6b1a9dd40a0048b17f1853",
        field_profile_sha256: "4f644e9be432a4e94d1bccebb1c3e4ee16850a78a22170597f910f2a0b6c4248",
    },
    NativeItemSpec {
        source_item_id: 239,
        source_label: "great health potion",
        item_class: "consumable_charges",
        native_key: "oteryn:item.consumable.potion.great_health",
        stack_capable: true,
        node_sha256: "1676d400a1f7e4a37efa63f06e8bf1ad012ef4732b74fa816f78b4211824b7f8",
        field_profile_sha256: "4f644e9be432a4e94d1bccebb1c3e4ee16850a78a22170597f910f2a0b6c4248",
    },
    NativeItemSpec {
        source_item_id: 266,
        source_label: "health potion",
        item_class: "consumable_charges",
        native_key: "oteryn:item.consumable.potion.health",
        stack_capable: true,
        node_sha256: "29d71d0e7dbd3f57adabafdef63b6224e0d0eb97aca0820a2ca5c1fc9b8036ed",
        field_profile_sha256: "74d31a24125469ba363fdb5833bf1097bf5280900531d7657549ddc5c281b2e0",
    },
    NativeItemSpec {
        source_item_id: 268,
        source_label: "mana potion",
        item_class: "consumable_charges",
        native_key: "oteryn:item.consumable.potion.mana",
        stack_capable: true,
        node_sha256: "4f3104c0a2a4cfc7ee1eb50f21cc39bae73a31c697d160a82d3c8e8905cc5b2c",
        field_profile_sha256: "74d31a24125469ba363fdb5833bf1097bf5280900531d7657549ddc5c281b2e0",
    },
    NativeItemSpec {
        source_item_id: 3048,
        source_label: "might ring",
        item_class: "consumable_charges",
        native_key: "oteryn:item.charged.might_ring",
        stack_capable: false,
        node_sha256: "a52e35b19f031bc94f44c71c402eb29a97928b91c649efb655e7c68aa8c759a0",
        field_profile_sha256: "ab35b1698f5a53bb4b4f29991e7e23aaa7cc0af72540f070e17b87b053b68782",
    },
    NativeItemSpec {
        source_item_id: 3081,
        source_label: "stone skin amulet",
        item_class: "consumable_charges",
        native_key: "oteryn:item.charged.stone_skin_amulet",
        stack_capable: false,
        node_sha256: "5c6c64e234b76e1b79e5e2d1d2228cbe5ab13bd854482cb455a12f5a6743ef7b",
        field_profile_sha256: "02b64132c553e683d58fa5910a5ae55fc96b91e203fe30cf20aeaa2fbd87da37",
    },
    NativeItemSpec {
        source_item_id: 3155,
        source_label: "sudden death rune",
        item_class: "consumable_charges",
        native_key: "oteryn:item.consumable.sudden_death_rune",
        stack_capable: true,
        node_sha256: "574e1ee5c03676abdcbdc3a5120db3d85ba693279a05a59ffd62b0341a0e223f",
        field_profile_sha256: "1d153b0d9524f79947e3d3a5adb4719798354e011c5d2ec977b4abf561479dbc",
    },
    NativeItemSpec {
        source_item_id: 9302,
        source_label: "sacred tree amulet",
        item_class: "consumable_charges",
        native_key: "oteryn:item.charged.sacred_tree_amulet",
        stack_capable: false,
        node_sha256: "7e0f1cf96e72d0b1cacb220d195b00c86f29da2acfa4a2107712d20ef463037b",
        field_profile_sha256: "ed3d4a2dd94bf1ff705516c36454de2f39091d9392078521915bde405bc0ce9a",
    },
    NativeItemSpec {
        source_item_id: 2389,
        source_label: "small blue pillow",
        item_class: "physical_decor",
        native_key: "oteryn:item.decor.pillow.small_blue",
        stack_capable: false,
        node_sha256: "9326478595c91bdc0cd132c264f3072cd182885f000f35f5d42db0b5ef6cc233",
        field_profile_sha256: "74d31a24125469ba363fdb5833bf1097bf5280900531d7657549ddc5c281b2e0",
    },
    NativeItemSpec {
        source_item_id: 2848,
        source_label: "purple tome",
        item_class: "physical_decor",
        native_key: "oteryn:item.decor.tome.purple",
        stack_capable: false,
        node_sha256: "f7a5a3f46ed282e8c3b13d61712a0484f4b6f6b82ad8de01f63a5b863b6b474a",
        field_profile_sha256: "4f644e9be432a4e94d1bccebb1c3e4ee16850a78a22170597f910f2a0b6c4248",
    },
    NativeItemSpec {
        source_item_id: 2850,
        source_label: "blue tome",
        item_class: "physical_decor",
        native_key: "oteryn:item.decor.tome.blue",
        stack_capable: false,
        node_sha256: "9557feb1f960911e48aa89b3c066d4ba0525bf9e23fa219be7eef98b14b16335",
        field_profile_sha256: "4f644e9be432a4e94d1bccebb1c3e4ee16850a78a22170597f910f2a0b6c4248",
    },
    NativeItemSpec {
        source_item_id: 2852,
        source_label: "red tome",
        item_class: "physical_decor",
        native_key: "oteryn:item.decor.tome.red",
        stack_capable: false,
        node_sha256: "e22b2abbef6842cd5de6bb8597e0fa9f3cd9ce5d0e4896783a6c47b6b8c22b31",
        field_profile_sha256: "4f644e9be432a4e94d1bccebb1c3e4ee16850a78a22170597f910f2a0b6c4248",
    },
    NativeItemSpec {
        source_item_id: 2876,
        source_label: "vase",
        item_class: "physical_decor",
        native_key: "oteryn:item.decor.vase",
        stack_capable: false,
        node_sha256: "b7c5c457cdccf047b251313e27cf283442a7346ddc556eaece8c2fdc30d655c3",
        field_profile_sha256: "11da415530a0c8678cdb3d05c3205f21f3cc708f13a7342674538f97b911472e",
    },
    NativeItemSpec {
        source_item_id: 2885,
        source_label: "brown flask",
        item_class: "physical_decor",
        native_key: "oteryn:item.decor.flask.brown",
        stack_capable: false,
        node_sha256: "876efad2b78488af6aa09fd7dde055a460a0b2f9316b632e3ccc9b361ad05bd3",
        field_profile_sha256: "11da415530a0c8678cdb3d05c3205f21f3cc708f13a7342674538f97b911472e",
    },
    NativeItemSpec {
        source_item_id: 2917,
        source_label: "candlestick",
        item_class: "physical_decor",
        native_key: "oteryn:item.decor.candlestick",
        stack_capable: false,
        node_sha256: "34410387d5be6973f665aa2f038c32311763e4a7ba02e25018e6191ad96a21e6",
        field_profile_sha256: "5974cb399f5abd02cee50220bb2d5ab4cbb97af42ac30c6b4605ddf9e70b5ed0",
    },
    NativeItemSpec {
        source_item_id: 2933,
        source_label: "small oil lamp",
        item_class: "physical_decor",
        native_key: "oteryn:item.decor.oil_lamp.small",
        stack_capable: false,
        node_sha256: "2d168d5565f0c7fe94a9d10f6f31c55d2203af837c443ea5cfdad3378a9e0926",
        field_profile_sha256: "74d31a24125469ba363fdb5833bf1097bf5280900531d7657549ddc5c281b2e0",
    },
    NativeItemSpec {
        source_item_id: 2953,
        source_label: "panpipes",
        item_class: "physical_decor",
        native_key: "oteryn:item.decor.panpipes",
        stack_capable: false,
        node_sha256: "638ecc4852144c304f962dfb44b4dfc319485bd69e7293439013f0b4f1f2e5d7",
        field_profile_sha256: "74d31a24125469ba363fdb5833bf1097bf5280900531d7657549ddc5c281b2e0",
    },
    NativeItemSpec {
        source_item_id: 2993,
        source_label: "teddy bear",
        item_class: "physical_decor",
        native_key: "oteryn:item.decor.teddy_bear",
        stack_capable: false,
        node_sha256: "339845df118885389556307c190d38f504edebd8baebb14b0714aed59a5fa88b",
        field_profile_sha256: "74d31a24125469ba363fdb5833bf1097bf5280900531d7657549ddc5c281b2e0",
    },
    NativeItemSpec {
        source_item_id: 2995,
        source_label: "piggy bank",
        item_class: "physical_decor",
        native_key: "oteryn:item.decor.piggy_bank",
        stack_capable: false,
        node_sha256: "5414b44e1d5e986d5c883e38dcad0dfd7378d99e490fd66bff55265801b20b95",
        field_profile_sha256: "74d31a24125469ba363fdb5833bf1097bf5280900531d7657549ddc5c281b2e0",
    },
    NativeItemSpec {
        source_item_id: 953,
        source_label: "nail",
        item_class: "physical_decor",
        native_key: "oteryn:item.physical.nail",
        stack_capable: false,
        node_sha256: "bc7dc34b75c7b262d6ba0368f803393cc9ca417a7a9c2dd7c941e1929b4c6e12",
        field_profile_sha256: "74d31a24125469ba363fdb5833bf1097bf5280900531d7657549ddc5c281b2e0",
    },
    NativeItemSpec {
        source_item_id: 940,
        source_label: "natural soil",
        item_class: "physical_decor",
        native_key: "oteryn:item.physical.soil.natural",
        stack_capable: false,
        node_sha256: "7d030e8ccc016d95cda9eabe533e57032531c5abd9458f978de3a688c45ebd56",
        field_profile_sha256: "7aaf75e89a491139e016bfa720eefffec0b7c659655db25d1cd13a9e97237d88",
    },
    NativeItemSpec {
        source_item_id: 941,
        source_label: "glimmering soil",
        item_class: "physical_decor",
        native_key: "oteryn:item.physical.soil.glimmering",
        stack_capable: false,
        node_sha256: "a0429e1e89785d0207f50eaf2043fec97171c9fd939c6f6056f5867cee945882",
        field_profile_sha256: "7aaf75e89a491139e016bfa720eefffec0b7c659655db25d1cd13a9e97237d88",
    },
    NativeItemSpec {
        source_item_id: 942,
        source_label: "flawless ice crystal",
        item_class: "physical_decor",
        native_key: "oteryn:item.physical.crystal.ice.flawless",
        stack_capable: false,
        node_sha256: "7fc23279085c279d190bdbb0d50b106d53b6d8003c28bb22c906d7a89431f5ab",
        field_profile_sha256: "74d31a24125469ba363fdb5833bf1097bf5280900531d7657549ddc5c281b2e0",
    },
    NativeItemSpec {
        source_item_id: 901,
        source_label: "marlin",
        item_class: "physical_decor",
        native_key: "oteryn:item.physical.marlin",
        stack_capable: false,
        node_sha256: "251b4ee4dc7c3b36ec6514f4a5829fc8c90254bd33b03b669c74ca5ca87eff01",
        field_profile_sha256: "4f644e9be432a4e94d1bccebb1c3e4ee16850a78a22170597f910f2a0b6c4248",
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtectedCw2B1ImportError {
    InputLimitExceeded { actual: usize, limit: usize },
    EvidenceMismatch(&'static str),
}

impl Display for ProtectedCw2B1ImportError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputLimitExceeded { actual, limit } => {
                write!(
                    formatter,
                    "CW2-B1 evidence bytes exceed limit: {actual} > {limit}"
                )
            }
            Self::EvidenceMismatch(field) => {
                write!(formatter, "protected CW2-B1 evidence mismatch: {field}")
            }
        }
    }
}

impl std::error::Error for ProtectedCw2B1ImportError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtectedCw2B1VaseImport {
    pub record: ProjectReferenceRecord,
    pub batch: ImportBatch,
}

/// Produce the one approved native Item record and its bound import provenance.
///
/// The exact protected catalogue digest is checked before any catalogue decoding. No broad
/// catalogue family is imported, and none of the source pickup, weight, name, primary-type or B3
/// observations select native gameplay semantics.
pub fn protected_cw2_b1_vase_import(
    evidence_bytes: &[u8],
) -> Result<ProtectedCw2B1VaseImport, ProtectedCw2B1ImportError> {
    validate_protected_evidence(evidence_bytes)?;

    let identity = DefinitionIdentityDocument {
        family: "Item".to_owned(),
        key: CW2_B1_VASE_KEY.to_owned(),
        revision: CW2_B1_VASE_REVISION.to_owned(),
    };
    let record = ProjectReferenceRecord::Item {
        identity: identity.clone(),
        client_projection: ProjectionDocument::ClientSafe,
        materializable: true,
        stack_class: ItemStackDocument::NonStackable,
    };

    let normalized_fields = vec![
        field(
            "binding.native-item",
            CandidateValue::NativeItemBinding(NativeItemBindingDocument {
                identity,
                disposition: NativeItemBindingDisposition::LocalNonProduction,
            }),
        ),
        text_field("evidence.catalog-blob", PROTECTED_CW2_B1_EVIDENCE_BLOB),
        text_field(
            "evidence.catalog-product-sha256",
            PROTECTED_CW2_B1_PRODUCT_SHA256,
        ),
        text_field("evidence.catalog-sha256", PROTECTED_CW2_B1_EVIDENCE_SHA256),
        text_field(
            "evidence.field-profile-sha256",
            PROTECTED_CW2_B1_FIELD_PROFILE_SHA256,
        ),
        text_field("evidence.mapper-blob", PROTECTED_CW2_B1_MAPPER_BLOB),
        text_field("evidence.mapper-sha256", PROTECTED_CW2_B1_MAPPER_SHA256),
        text_field("evidence.node-sha256", PROTECTED_CW2_B1_NODE_SHA256),
        text_field("loss.b3-loot-semantics", "NOT_PROMOTED"),
        text_field("loss.redistribution-grant", "NONE"),
        text_field("loss.weight-native-field", "UNREPRESENTED"),
        text_field("loss.weight-unit", "UNKNOWN"),
        text_field("source.b3-loot-row-digest", CW2_B1_VASE_B3_ROW_SHA256),
        text_field("source.b3-loot-row-identity", CW2_B1_VASE_B3_ROW),
        field(
            "source.item-id",
            CandidateValue::SourceId(CW2_B1_SOURCE_ITEM_ID),
        ),
        text_field("source.items-xml-blob", CW2_B1_SOURCE_BLOB),
        field(
            "source.items-xml-byte-length",
            CandidateValue::Integer(CW2_B1_SOURCE_BYTES),
        ),
        text_field("source.items-xml-path", CW2_B1_SOURCE_PATH),
        text_field("source.items-xml-sha256", CW2_B1_SOURCE_SHA256),
        text_field("source.license-locator", "LICENSE"),
        text_field("source.license-observation", "GNU GPL v2"),
        field("source.pickup-eligibility", CandidateValue::Integer(1)),
        text_field("source.primary-type-disposition", "PROVENANCE_ONLY"),
        field("source.weight-raw", CandidateValue::Integer(940)),
    ];

    let reimport_states = [
        ("source.pickup-eligibility", CandidateValue::Integer(1)),
        ("source.weight-raw", CandidateValue::Integer(940)),
    ]
    .into_iter()
    .map(|(field_path, value)| ReimportFieldState {
        stable_identity: CW2_B1_VASE_KEY.to_owned(),
        field_path: field_path.to_owned(),
        baseline: Some(value.clone()),
        upstream: Some(value.clone()),
        local: Some(value),
        decision: ReimportDecision::Unchanged,
    })
    .collect();

    Ok(ProtectedCw2B1VaseImport {
        record,
        batch: ImportBatch {
            batch_id: "cw2-b1-vase-native-item".to_owned(),
            source_repository: CW2_B1_SOURCE_REPOSITORY.to_owned(),
            source_revision: CW2_B1_SOURCE_REVISION.to_owned(),
            source_artifact_sha256: PROTECTED_CW2_B1_EVIDENCE_SHA256.to_owned(),
            access_disposition: "PENDING".to_owned(),
            source_generation_profile: CATALOG_SCHEMA.to_owned(),
            importer: "repository-protected-cw2-b1-evidence".to_owned(),
            mapper: MAPPER_PROFILE.to_owned(),
            mapper_revision: PROTECTED_CW2_B1_MAPPER_BLOB.to_owned(),
            mapper_sha256: PROTECTED_CW2_B1_MAPPER_SHA256.to_owned(),
            candidates: vec![ImportCandidate {
                source_candidate_id: SOURCE_CANDIDATE_ID.to_owned(),
                source_label: "vase".to_owned(),
                source_numeric_id: Some(CW2_B1_SOURCE_ITEM_ID),
                candidate_family: ImportCandidateFamily::Item,
                candidate_operation: ImportCandidateOperation::BindNativeItem,
                candidate_target: format!("{CW2_B1_VASE_KEY}@{CW2_B1_VASE_REVISION}"),
                candidate_formula: "NOT_APPLICABLE".to_owned(),
                evidence_class: "OTS_HYPOTHESIS_ONLY".to_owned(),
                closure_disposition: CandidateDisposition::LocalNonProduction,
                disposition_reason: "OWNER_APPROVED_LOCAL_NON_PRODUCTION_BINDING".to_owned(),
                normalized_fields,
            }],
            reimport_states,
        },
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtectedCw2B1NativeItemBatchImport {
    pub records: Vec<ProjectReferenceRecord>,
    pub batch: ImportBatch,
}

/// Produce the first bounded multi-class native Item binding batch.
///
/// Stable keys and minimal stack shapes are Oteryn-authored decisions. Source labels, numeric IDs,
/// candidate field profiles, B3 occurrence evidence and all unsupported semantics remain
/// provenance-only. They never generate or modify a native identity or gameplay value.
pub fn protected_cw2_b1_native_item_batch_import(
    evidence_bytes: &[u8],
) -> Result<ProtectedCw2B1NativeItemBatchImport, ProtectedCw2B1ImportError> {
    validate_protected_evidence(evidence_bytes)?;

    let mut source_ids = BTreeSet::new();
    let mut native_keys = BTreeSet::new();
    let mut records = Vec::with_capacity(CW2_B1_NATIVE_ITEM_BATCH_COUNT);
    let mut candidates = Vec::with_capacity(CW2_B1_NATIVE_ITEM_BATCH_COUNT);
    let mut reimport_states = Vec::with_capacity(CW2_B1_NATIVE_ITEM_BATCH_COUNT);

    let mut specs = NATIVE_ITEM_BATCH;
    specs.sort_by(|left, right| left.native_key.cmp(right.native_key));
    for spec in specs {
        if !source_ids.insert(spec.source_item_id) || !native_keys.insert(spec.native_key) {
            return Err(ProtectedCw2B1ImportError::EvidenceMismatch(
                "native item batch uniqueness",
            ));
        }

        let identity = DefinitionIdentityDocument {
            family: "Item".to_owned(),
            key: spec.native_key.to_owned(),
            revision: CW2_B1_NATIVE_ITEM_BATCH_REVISION.to_owned(),
        };
        records.push(ProjectReferenceRecord::Item {
            identity: identity.clone(),
            client_projection: ProjectionDocument::ClientSafe,
            materializable: true,
            stack_class: if spec.stack_capable {
                ItemStackDocument::StackCapable
            } else {
                ItemStackDocument::NonStackable
            },
        });

        let source_candidate_id = format!("crystal:item:{}", spec.source_item_id);
        let normalized_fields = vec![
            field(
                "binding.native-item",
                CandidateValue::NativeItemBinding(NativeItemBindingDocument {
                    identity,
                    disposition: NativeItemBindingDisposition::LocalNonProduction,
                }),
            ),
            text_field("evidence.catalog-blob", PROTECTED_CW2_B1_EVIDENCE_BLOB),
            text_field(
                "evidence.catalog-product-sha256",
                PROTECTED_CW2_B1_PRODUCT_SHA256,
            ),
            text_field("evidence.catalog-sha256", PROTECTED_CW2_B1_EVIDENCE_SHA256),
            text_field("evidence.field-profile-sha256", spec.field_profile_sha256),
            text_field("evidence.mapper-blob", PROTECTED_CW2_B1_MAPPER_BLOB),
            text_field("evidence.mapper-sha256", PROTECTED_CW2_B1_MAPPER_SHA256),
            text_field("evidence.node-sha256", spec.node_sha256),
            text_field("loss.b3-loot-semantics", "NOT_PROMOTED"),
            text_field(
                "loss.reference-item-definition",
                reference_item_definition_gap(spec.item_class),
            ),
            text_field("loss.redistribution-grant", "NONE"),
            text_field("loss.source-values-as-gameplay-truth", "REJECTED"),
            field(
                "source.item-id",
                CandidateValue::SourceId(spec.source_item_id),
            ),
            text_field("source.item-identity", &source_candidate_id),
            text_field("source.item-label", spec.source_label),
            text_field("source.item-label-disposition", "PROVENANCE_ONLY"),
            text_field("source.items-xml-blob", CW2_B1_SOURCE_BLOB),
            field(
                "source.items-xml-byte-length",
                CandidateValue::Integer(CW2_B1_SOURCE_BYTES),
            ),
            text_field("source.items-xml-path", CW2_B1_SOURCE_PATH),
            text_field("source.items-xml-sha256", CW2_B1_SOURCE_SHA256),
            text_field("source.license-locator", "LICENSE"),
            text_field("source.license-observation", "GNU GPL v2"),
            text_field(
                "source.native-key-authorship",
                "OTERYN_EDITORIAL_SELECTION_NOT_SOURCE_DERIVED",
            ),
            text_field("source.representative-class", spec.item_class),
        ];
        candidates.push(ImportCandidate {
            source_candidate_id,
            source_label: spec.source_label.to_owned(),
            source_numeric_id: Some(spec.source_item_id),
            candidate_family: ImportCandidateFamily::Item,
            candidate_operation: ImportCandidateOperation::BindNativeItem,
            candidate_target: format!("{}@{}", spec.native_key, CW2_B1_NATIVE_ITEM_BATCH_REVISION),
            candidate_formula: "NOT_APPLICABLE".to_owned(),
            evidence_class: "OTS_HYPOTHESIS_ONLY".to_owned(),
            closure_disposition: CandidateDisposition::LocalNonProduction,
            disposition_reason: "OWNER_APPROVED_CW2_NATIVE_ITEM_BATCH".to_owned(),
            normalized_fields,
        });
        let source_id_value = CandidateValue::SourceId(spec.source_item_id);
        reimport_states.push(ReimportFieldState {
            stable_identity: spec.native_key.to_owned(),
            field_path: "source.item-id".to_owned(),
            baseline: Some(source_id_value.clone()),
            upstream: Some(source_id_value.clone()),
            local: Some(source_id_value),
            decision: ReimportDecision::Unchanged,
        });
    }

    candidates.sort_by(|left, right| left.source_candidate_id.cmp(&right.source_candidate_id));
    for candidate in &mut candidates {
        candidate
            .normalized_fields
            .sort_by(|left, right| left.field_path.cmp(&right.field_path));
    }
    reimport_states.sort_by(|left, right| {
        left.stable_identity
            .cmp(&right.stable_identity)
            .then_with(|| left.field_path.cmp(&right.field_path))
    });

    Ok(ProtectedCw2B1NativeItemBatchImport {
        records,
        batch: ImportBatch {
            batch_id: "cw2-b1-native-item-batch-64".to_owned(),
            source_repository: CW2_B1_SOURCE_REPOSITORY.to_owned(),
            source_revision: CW2_B1_SOURCE_REVISION.to_owned(),
            source_artifact_sha256: PROTECTED_CW2_B1_EVIDENCE_SHA256.to_owned(),
            access_disposition: "PENDING".to_owned(),
            source_generation_profile: CATALOG_SCHEMA.to_owned(),
            importer: "repository-protected-cw2-b1-evidence".to_owned(),
            mapper: MAPPER_PROFILE.to_owned(),
            mapper_revision: PROTECTED_CW2_B1_MAPPER_BLOB.to_owned(),
            mapper_sha256: PROTECTED_CW2_B1_MAPPER_SHA256.to_owned(),
            candidates,
            reimport_states,
        },
    })
}

fn validate_protected_evidence(evidence_bytes: &[u8]) -> Result<(), ProtectedCw2B1ImportError> {
    if evidence_bytes.len() > PROTECTED_CW2_B1_EVIDENCE_BYTES {
        return Err(ProtectedCw2B1ImportError::InputLimitExceeded {
            actual: evidence_bytes.len(),
            limit: PROTECTED_CW2_B1_EVIDENCE_BYTES,
        });
    }
    if evidence_bytes.len() != PROTECTED_CW2_B1_EVIDENCE_BYTES {
        return Err(ProtectedCw2B1ImportError::EvidenceMismatch(
            "evidence byte length",
        ));
    }
    if world_project_sha256(evidence_bytes) != PROTECTED_CW2_B1_EVIDENCE_SHA256 {
        return Err(ProtectedCw2B1ImportError::EvidenceMismatch(
            "evidence byte digest",
        ));
    }
    Ok(())
}

fn reference_item_definition_gap(item_class: &str) -> &'static str {
    match item_class {
        "currency_stackable" => {
            "STACK_MAX;TYPED_WEIGHT_UNIT;VALUE_OR_CURRENCY_SEMANTICS;PRESENTATION"
        }
        "weapon" => {
            "WEAPON_USE;FORMULA;PROTECTION;MODIFIERS;IMBUEMENT;TYPED_WEIGHT_UNIT;PRESENTATION"
        }
        "equipment" => {
            "EQUIP_PATTERN;SLOT_SEMANTICS;REQUIREMENTS;PROTECTION;MODIFIERS;RESISTANCE;IMBUEMENT;TYPED_WEIGHT_UNIT;PRESENTATION"
        }
        "container" => {
            "CONTAINER_CAPACITY;CONTAINMENT_POLICY;NESTING_BOUNDS;TYPED_WEIGHT_UNIT;PRESENTATION"
        }
        "consumable_charges" => {
            "CHARGES;USE_SEMANTICS;DECAY_OR_TIMING;MODIFIERS;TYPED_WEIGHT_UNIT;PRESENTATION"
        }
        "physical_decor" => {
            "PRESENTATION;TYPED_WEIGHT_UNIT;LIGHT_READABLE_FLUID_TEMPORAL_OR_INTERACTION_FIELDS"
        }
        _ => "UNKNOWN_UNSUPPORTED_ITEM_CLASS",
    }
}

fn text_field(path: &str, value: &str) -> NamedCandidateField {
    field(path, CandidateValue::Text(value.to_owned()))
}

fn field(path: &str, value: CandidateValue) -> NamedCandidateField {
    NamedCandidateField {
        field_path: path.to_owned(),
        value,
    }
}
