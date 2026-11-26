//! Source-qualified NPC contacts in the six regional placement batches.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::path::Path;

use oteryn_world_bundle_compiler::bundle::{BuildClass, Identity};
use oteryn_world_bundle_compiler::compile::{Input, realize_npcs, realize_spawns};
use oteryn_world_bundle_compiler::source::{project_regions, registry, world_extent};

use oteryn_world_bundle_compiler::spawn::Direction;

type Contact = (&'static str, u16, u16, u8, Direction);

// Crystal 201189f8 world-npc.xml, with the retained Wiki cells for Wiki-only placements.
const CONTACTS: &[Contact] = &[
    // Edron
    (
        "oteryn:npc.a_dark_priestess",
        33258,
        31938,
        15,
        Direction::North,
    ),
    (
        "oteryn:npc.a_majestic_warwolf",
        33355,
        31991,
        8,
        Direction::North,
    ),
    ("oteryn:npc.albert", 33312, 31761, 7, Direction::North),
    (
        "oteryn:npc.an_ominous_bat",
        33264,
        31785,
        7,
        Direction::North,
    ),
    (
        "oteryn:npc.anaztassja_moroia_init",
        33268,
        31787,
        8,
        Direction::North,
    ),
    ("oteryn:npc.arkulius", 33280, 31845, 3, Direction::North),
    ("oteryn:npc.beatrice", 33215, 31802, 6, Direction::North),
    ("oteryn:npc.bonifacius", 33167, 31801, 7, Direction::North),
    (
        "oteryn:npc.captain_seahorse",
        33175,
        31763,
        6,
        Direction::North,
    ),
    (
        "oteryn:npc.chief_grarkharok",
        33231,
        31754,
        1,
        Direction::North,
    ),
    ("oteryn:npc.chrystal", 33175, 31811, 7, Direction::North),
    ("oteryn:npc.cornell", 33342, 31691, 7, Direction::North),
    ("oteryn:npc.cornell", 33304, 31720, 7, Direction::North),
    (
        "oteryn:npc.daniel_steelsoul",
        33192,
        31796,
        7,
        Direction::North,
    ),
    ("oteryn:npc.ebenizer", 33173, 31801, 7, Direction::North),
    (
        "oteryn:npc.edron_guardsman",
        33189,
        31788,
        7,
        Direction::North,
    ),
    (
        "oteryn:npc.edron_guardsman",
        33193,
        31788,
        7,
        Direction::North,
    ),
    ("oteryn:npc.edvard", 33229, 31832, 7, Direction::North),
    ("oteryn:npc.eliza", 33268, 31847, 6, Direction::North),
    ("oteryn:npc.falonzo", 33292, 31792, 13, Direction::North),
    ("oteryn:npc.fernlor", 33279, 31846, 8, Direction::North),
    ("oteryn:npc.fiona", 33281, 31846, 5, Direction::North),
    ("oteryn:npc.ghouliver", 33269, 31786, 8, Direction::North),
    ("oteryn:npc.gladys", 33348, 31687, 7, Direction::North),
    ("oteryn:npc.gnomadness", 33527, 32865, 14, Direction::North),
    ("oteryn:npc.gnomfurry", 33529, 32902, 14, Direction::North),
    ("oteryn:npc.gnominer", 33520, 32881, 14, Direction::North),
    (
        "oteryn:npc.gnomish_operative_catching",
        33608,
        32837,
        15,
        Direction::North,
    ),
    (
        "oteryn:npc.gnomish_operative_gt_m_01",
        33704,
        32947,
        15,
        Direction::North,
    ),
    (
        "oteryn:npc.gnomish_operative_raiders",
        33587,
        32966,
        15,
        Direction::North,
    ),
    (
        "oteryn:npc.gnomish_operative_resonating",
        33703,
        32869,
        15,
        Direction::North,
    ),
    (
        "oteryn:npc.gnomish_operative_volcano",
        33670,
        32923,
        15,
        Direction::North,
    ),
    ("oteryn:npc.gundralph", 33267, 31848, 5, Direction::North),
    ("oteryn:npc.gunther", 33171, 31778, 7, Direction::North),
    ("oteryn:npc.gurbasch", 33311, 31988, 15, Direction::North),
    ("oteryn:npc.hawkhurst", 33356, 31982, 7, Direction::North),
    ("oteryn:npc.ikassis", 33123, 31713, 7, Direction::North),
    ("oteryn:npc.iwan", 33216, 31832, 7, Direction::North),
    ("oteryn:npc.jack", 33277, 31756, 7, Direction::North),
    ("oteryn:npc.james", 33279, 31770, 7, Direction::North),
    ("oteryn:npc.jerom", 33240, 31757, 7, Direction::North),
    ("oteryn:npc.lailene", 33282, 31845, 4, Direction::North),
    (
        "oteryn:npc.lesser_messenger_of_heaven",
        33194,
        31644,
        7,
        Direction::North,
    ),
    ("oteryn:npc.luna", 33254, 31840, 5, Direction::North),
    ("oteryn:npc.maeryn", 33330, 31686, 7, Direction::North),
    ("oteryn:npc.milos", 33258, 31823, 3, Direction::North),
    (
        "oteryn:npc.mother_of_jack",
        33274,
        31759,
        6,
        Direction::North,
    ),
    ("oteryn:npc.myzzi", 33272, 31722, 7, Direction::North),
    ("oteryn:npc.pemaret", 33286, 31955, 6, Direction::North),
    ("oteryn:npc.puffels", 33267, 31848, 8, Direction::North),
    ("oteryn:npc.rudolph", 33213, 31812, 6, Direction::North),
    ("oteryn:npc.sandra", 33257, 31839, 7, Direction::North),
    ("oteryn:npc.sane_mage", 33299, 31811, 7, Direction::North),
    ("oteryn:npc.scrutinon", 33189, 31982, 7, Direction::North),
    (
        "oteryn:npc.servant_sentry",
        33312,
        31836,
        7,
        Direction::North,
    ),
    (
        "oteryn:npc.shoddy_beggar",
        33157,
        31815,
        7,
        Direction::North,
    ),
    ("oteryn:npc.sinclair", 33268, 31835, 2, Direction::North),
    (
        "oteryn:npc.sister_of_jack",
        33277,
        31756,
        5,
        Direction::North,
    ),
    ("oteryn:npc.telas", 33276, 31800, 6, Direction::North),
    ("oteryn:npc.telas", 33273, 31788, 11, Direction::North),
    ("oteryn:npc.telas_golem", 33267, 31788, 11, Direction::North),
    ("oteryn:npc.tereban", 33170, 31807, 6, Direction::North),
    ("oteryn:npc.thomas", 33256, 31838, 4, Direction::North),
    (
        "oteryn:npc.urks_the_mute",
        33517,
        32856,
        14,
        Direction::North,
    ),
    ("oteryn:npc.ursula", 33267, 31848, 7, Direction::North),
    ("oteryn:npc.wyrdin", 33266, 31835, 6, Direction::North),
    ("oteryn:npc.yoem", 33304, 31965, 7, Direction::North),
    // Darashia
    ("oteryn:npc.angelo", 33283, 32273, 12, Direction::North),
    ("oteryn:npc.azil", 33217, 32431, 7, Direction::North),
    ("oteryn:npc.bo_ques", 33102, 32521, 5, Direction::North),
    ("oteryn:npc.bozarn", 33274, 32388, 7, Direction::North),
    ("oteryn:npc.chemar", 33269, 32440, 6, Direction::North),
    ("oteryn:npc.dabui", 33230, 32409, 7, Direction::North),
    ("oteryn:npc.djema", 33099, 32522, 3, Direction::North),
    ("oteryn:npc.edoch", 33232, 32430, 7, Direction::North),
    (
        "oteryn:npc.elyen_ravenlock",
        33012,
        32359,
        11,
        Direction::North,
    ),
    ("oteryn:npc.fa_hradin", 33102, 32540, 5, Direction::North),
    ("oteryn:npc.fadil", 33219, 32396, 7, Direction::North),
    ("oteryn:npc.gabel", 33101, 32520, 1, Direction::North),
    ("oteryn:npc.guide_behil", 33289, 32469, 7, Direction::North),
    ("oteryn:npc.halif", 33217, 32423, 7, Direction::North),
    ("oteryn:npc.haroun", 33109, 32525, 4, Direction::North),
    ("oteryn:npc.hofech", 33270, 32440, 7, Direction::North),
    ("oteryn:npc.kazzan", 33231, 32389, 6, Direction::North),
    ("oteryn:npc.mazarius", 33277, 32390, 8, Direction::North),
    ("oteryn:npc.mazarius", 33277, 32390, 9, Direction::North),
    ("oteryn:npc.morun", 33240, 32392, 6, Direction::North),
    ("oteryn:npc.mugluf", 33220, 32416, 7, Direction::North),
    ("oteryn:npc.muhad", 33279, 32526, 9, Direction::North),
    ("oteryn:npc.muzir", 33222, 32389, 7, Direction::North),
    ("oteryn:npc.nah_bob", 33103, 32520, 2, Direction::North),
    ("oteryn:npc.oblivion", 33023, 32313, 10, Direction::North),
    ("oteryn:npc.omrabas", 33020, 32454, 9, Direction::North),
    ("oteryn:npc.omur", 33228, 32416, 7, Direction::North),
    ("oteryn:npc.petros", 33290, 32481, 6, Direction::North),
    ("oteryn:npc.samir", 33027, 32417, 4, Direction::North),
    ("oteryn:npc.shalmar", 33220, 32409, 7, Direction::North),
    (
        "oteryn:npc.the_gate_keeper",
        32855,
        32426,
        10,
        Direction::North,
    ),
    (
        "oteryn:npc.tomruk_the_ruddy",
        32983,
        32418,
        11,
        Direction::North,
    ),
    ("oteryn:npc.torkada", 33033, 32425, 7, Direction::South),
    ("oteryn:npc.umar", 33102, 32531, 6, Direction::North),
    // Ankrahmun
    ("oteryn:npc.ahmet", 33126, 32810, 6, Direction::North),
    ("oteryn:npc.alesar", 33048, 32621, 5, Direction::North),
    (
        "oteryn:npc.an_ancient_priest",
        33028,
        32706,
        7,
        Direction::North,
    ),
    ("oteryn:npc.arito", 33070, 32885, 6, Direction::North),
    ("oteryn:npc.arkhothep", 33149, 32842, 4, Direction::North),
    ("oteryn:npc.baa_leal", 33044, 32623, 4, Direction::North),
    (
        "oteryn:npc.captain_sinbeard",
        33091,
        32883,
        6,
        Direction::North,
    ),
    ("oteryn:npc.dario", 33155, 32810, 4, Direction::North),
    ("oteryn:npc.feizuhl", 33066, 32886, 7, Direction::North),
    ("oteryn:npc.fenech", 33131, 32820, 5, Direction::North),
    (
        "oteryn:npc.guide_rahlkora",
        33085,
        32885,
        6,
        Direction::North,
    ),
    ("oteryn:npc.ishebad", 33158, 32849, 7, Direction::North),
    ("oteryn:npc.jakahr", 33066, 32876, 7, Direction::North),
    ("oteryn:npc.jean_pierre", 33066, 32530, 6, Direction::North),
    ("oteryn:npc.jezzara", 33126, 32821, 6, Direction::North),
    ("oteryn:npc.malor", 33044, 32622, 1, Direction::North),
    ("oteryn:npc.mehkesh", 33130, 32811, 5, Direction::North),
    ("oteryn:npc.melchior", 33142, 32817, 7, Direction::North),
    ("oteryn:npc.memech", 33135, 32810, 6, Direction::North),
    (
        "oteryn:npc.messenger_of_heaven",
        32157,
        31290,
        6,
        Direction::North,
    ),
    (
        "oteryn:npc.messenger_of_heaven",
        33195,
        31646,
        7,
        Direction::North,
    ),
    (
        "oteryn:npc.messenger_of_heaven",
        33172,
        32634,
        7,
        Direction::North,
    ),
    (
        "oteryn:npc.messenger_of_heaven",
        32207,
        31377,
        14,
        Direction::North,
    ),
    ("oteryn:npc.ormuhn", 33159, 32810, 5, Direction::North),
    ("oteryn:npc.rata_mari", 33037, 32626, 7, Direction::North),
    ("oteryn:npc.tesha", 33135, 32821, 6, Direction::North),
    ("oteryn:npc.the_crone", 33230, 32522, 14, Direction::North),
    ("oteryn:npc.tonar", 33181, 32883, 7, Direction::North),
    ("oteryn:npc.tothdral", 33149, 32867, 6, Direction::North),
    ("oteryn:npc.ubaid", 33047, 32622, 6, Direction::North),
    ("oteryn:npc.yaman", 33045, 32620, 2, Direction::North),
    // Port Hope
    ("oteryn:npc.andrew_lyze", 32759, 32632, 7, Direction::North),
    ("oteryn:npc.angus", 32670, 32730, 7, Direction::North),
    ("oteryn:npc.brengus", 32634, 32747, 6, Direction::North),
    ("oteryn:npc.clark", 32635, 32797, 7, Direction::North),
    ("oteryn:npc.clyde", 32574, 32752, 7, Direction::North),
    ("oteryn:npc.ferks", 32634, 32739, 6, Direction::North),
    (
        "oteryn:npc.ghost_of_a_priest",
        32732,
        32564,
        12,
        Direction::North,
    ),
    (
        "oteryn:npc.ghost_of_a_priest",
        32784,
        32528,
        13,
        Direction::North,
    ),
    (
        "oteryn:npc.ghost_of_a_priest",
        32809,
        32641,
        14,
        Direction::North,
    ),
    ("oteryn:npc.hairycles", 32826, 32574, 6, Direction::North),
    ("oteryn:npc.helor", 32572, 32753, 6, Direction::North),
    ("oteryn:npc.imbul", 32558, 32781, 7, Direction::North),
    ("oteryn:npc.kais", 32910, 32512, 8, Direction::North),
    ("oteryn:npc.lorek", 32679, 32775, 7, Direction::North),
    ("oteryn:npc.old_adall", 32629, 32771, 7, Direction::North),
    ("oteryn:npc.perod", 32634, 32740, 5, Direction::North),
    ("oteryn:npc.ray", 32620, 32746, 6, Direction::North),
    ("oteryn:npc.tandros", 32621, 32740, 5, Direction::North),
    (
        "oteryn:npc.the_blind_prophet",
        33021,
        32604,
        7,
        Direction::North,
    ),
    ("oteryn:npc.uso", 32579, 32755, 8, Direction::North),
    ("oteryn:npc.ustan", 32580, 32757, 6, Direction::North),
    ("oteryn:npc.vescu", 32565, 32651, 7, Direction::North),
    ("oteryn:npc.yawno", 32614, 32651, 7, Direction::North),
    ("oteryn:npc.zaidal", 32622, 32746, 5, Direction::North),
    // Liberty Bay
    (
        "oteryn:npc.admiral_wyrmslicer",
        32396,
        32821,
        3,
        Direction::North,
    ),
    ("oteryn:npc.an_idol", 32398, 32509, 7, Direction::North),
    ("oteryn:npc.atrad", 32077, 32533, 9, Direction::North),
    ("oteryn:npc.augustin", 32393, 32470, 4, Direction::North),
    ("oteryn:npc.barney", 32142, 32926, 7, Direction::North),
    ("oteryn:npc.berenice", 32359, 32812, 7, Direction::North),
    ("oteryn:npc.bertram", 32355, 32797, 7, Direction::North),
    ("oteryn:npc.bounac_guard", 32400, 32481, 4, Direction::North),
    ("oteryn:npc.braden", 32277, 32707, 7, Direction::North),
    ("oteryn:npc.bradford", 32299, 32837, 7, Direction::North),
    ("oteryn:npc.cameron", 32201, 32837, 7, Direction::North),
    ("oteryn:npc.captain_max", 32298, 32895, 6, Direction::North),
    (
        "oteryn:npc.captain_max_calassa",
        31915,
        32710,
        6,
        Direction::North,
    ),
    ("oteryn:npc.cedrik", 32292, 32818, 7, Direction::North),
    ("oteryn:npc.chantalle", 32363, 32792, 7, Direction::North),
    ("oteryn:npc.charlotta", 32268, 32841, 7, Direction::North),
    ("oteryn:npc.chondur", 32361, 32550, 7, Direction::North),
    ("oteryn:npc.danlon", 32190, 32950, 7, Direction::North),
    ("oteryn:npc.doug", 32201, 32756, 7, Direction::North),
    ("oteryn:npc.eleonore", 32331, 32763, 5, Direction::North),
    ("oteryn:npc.eranth", 32121, 32953, 7, Direction::North),
    ("oteryn:npc.erayo", 32518, 32911, 8, Direction::North),
    ("oteryn:npc.evan", 32259, 32881, 7, Direction::North),
    ("oteryn:npc.fergus", 32221, 32810, 7, Direction::North),
    ("oteryn:npc.frederik", 32345, 32808, 7, Direction::North),
    ("oteryn:npc.gordon", 32184, 32770, 7, Direction::North),
    ("oteryn:npc.guide_davina", 32275, 32894, 6, Direction::North),
    ("oteryn:npc.hamilton", 32200, 32792, 7, Direction::North),
    ("oteryn:npc.herbert", 32332, 32839, 7, Direction::North),
    ("oteryn:npc.irvin", 32185, 32736, 7, Direction::North),
    ("oteryn:npc.isolde", 32412, 32831, 4, Direction::North),
    ("oteryn:npc.jack_fate", 32286, 32891, 6, Direction::North),
    (
        "oteryn:npc.jack_fate_goroma",
        32161,
        32559,
        6,
        Direction::North,
    ),
    ("oteryn:npc.jason", 32157, 32940, 7, Direction::North),
    ("oteryn:npc.jefrey", 32338, 32834, 7, Direction::North),
    ("oteryn:npc.john", 32139, 32904, 7, Direction::North),
    ("oteryn:npc.john_bounac", 32442, 32486, 3, Direction::North),
    ("oteryn:npc.klaus", 31981, 32826, 2, Direction::North),
    ("oteryn:npc.larry", 32268, 32811, 11, Direction::North),
    (
        "oteryn:npc.lesh_the_seer",
        32406,
        32471,
        6,
        Direction::North,
    ),
    ("oteryn:npc.lyonel", 32298, 32832, 7, Direction::North),
    ("oteryn:npc.malunga", 32345, 32809, 5, Direction::North),
    ("oteryn:npc.marcus", 32316, 32861, 7, Direction::North),
    ("oteryn:npc.meraya", 32194, 32974, 7, Direction::North),
    ("oteryn:npc.norris", 32224, 32770, 7, Direction::North),
    ("oteryn:npc.ocelus", 32312, 32622, 4, Direction::North),
    ("oteryn:npc.one_eyed_joe", 32025, 32884, 6, Direction::North),
    ("oteryn:npc.paolo", 32159, 32965, 7, Direction::North),
    ("oteryn:npc.parlan", 32185, 32795, 7, Direction::North),
    ("oteryn:npc.peggy", 32294, 32807, 7, Direction::North),
    (
        "oteryn:npc.percy_silverhand",
        32330,
        32767,
        6,
        Direction::North,
    ),
    ("oteryn:npc.polly", 32116, 32968, 11, Direction::North),
    ("oteryn:npc.red_lilly", 32283, 32821, 7, Direction::North),
    ("oteryn:npc.ross", 32245, 32713, 7, Direction::North),
    ("oteryn:npc.sebastian", 32349, 32624, 7, Direction::North),
    (
        "oteryn:npc.sebastian_nargor",
        32027,
        32814,
        7,
        Direction::North,
    ),
    (
        "oteryn:npc.skeleton_guard",
        32131,
        32970,
        11,
        Direction::North,
    ),
    ("oteryn:npc.skip", 32122, 32968, 11, Direction::North),
    ("oteryn:npc.squeekquek", 32289, 32813, 10, Direction::South),
    (
        "oteryn:npc.theodore_loveless",
        32361,
        32787,
        6,
        Direction::North,
    ),
    ("oteryn:npc.torence", 32184, 32826, 7, Direction::North),
    ("oteryn:npc.tristan", 32397, 32825, 2, Direction::North),
    ("oteryn:npc.vulturenose", 31977, 32856, 3, Direction::North),
    // Svargrond
    (
        "oteryn:npc.a_dragon_mother",
        32303,
        31015,
        13,
        Direction::North,
    ),
    (
        "oteryn:npc.a_restless_soul",
        32256,
        31101,
        10,
        Direction::North,
    ),
    (
        "oteryn:npc.a_starving_dog",
        32122,
        31091,
        5,
        Direction::North,
    ),
    ("oteryn:npc.baltim", 32343, 31322, 6, Direction::North),
    ("oteryn:npc.bertha", 32319, 31113, 7, Direction::North),
    (
        "oteryn:npc.buddel_helheim",
        32462,
        31173,
        7,
        Direction::North,
    ),
    (
        "oteryn:npc.buddel_okolnir",
        32224,
        31381,
        7,
        Direction::North,
    ),
    (
        "oteryn:npc.buddel_raider_camp",
        32020,
        31294,
        7,
        Direction::North,
    ),
    (
        "oteryn:npc.buddel_tyrsung",
        32332,
        31227,
        7,
        Direction::North,
    ),
    (
        "oteryn:npc.captain_breezelda",
        32341,
        31109,
        6,
        Direction::North,
    ),
    ("oteryn:npc.captain_haba", 32353, 31124, 6, Direction::North),
    ("oteryn:npc.dankwart", 32202, 31161, 7, Direction::North),
    ("oteryn:npc.eirik", 32219, 31142, 7, Direction::North),
    ("oteryn:npc.freezhild", 32024, 31442, 5, Direction::North),
    ("oteryn:npc.frosty", 33888, 31016, 6, Direction::North),
    ("oteryn:npc.fynn", 32288, 31135, 7, Direction::North),
    ("oteryn:npc.halvar", 32231, 31097, 7, Direction::North),
    ("oteryn:npc.hawkyr", 32255, 31080, 6, Direction::North),
    ("oteryn:npc.iskan", 32302, 31082, 7, Direction::North),
    ("oteryn:npc.iyad", 32251, 31097, 4, Direction::North),
    ("oteryn:npc.janz", 32279, 31131, 7, Direction::North),
    ("oteryn:npc.jessica", 32299, 31136, 7, Direction::North),
    ("oteryn:npc.kjesse", 32211, 31132, 7, Direction::North),
    ("oteryn:npc.lurik", 32318, 31134, 7, Direction::North),
    ("oteryn:npc.nelly", 32307, 31134, 7, Direction::North),
    ("oteryn:npc.nilsor", 32330, 31046, 7, Direction::North),
    ("oteryn:npc.nomad", 31941, 31305, 6, Direction::North),
    ("oteryn:npc.nor", 32004, 31443, 7, Direction::North),
    ("oteryn:npc.robert", 32233, 31073, 6, Direction::North),
    ("oteryn:npc.romir", 32211, 31058, 5, Direction::North),
    ("oteryn:npc.siflind", 32360, 31030, 6, Direction::North),
    ("oteryn:npc.sirik", 32328, 31118, 7, Direction::North),
    ("oteryn:npc.sven", 32202, 31153, 7, Direction::North),
    ("oteryn:npc.thorwulf", 32264, 31090, 6, Direction::North),
    ("oteryn:npc.zora", 32364, 31324, 6, Direction::North),
];

#[test]
fn regional_npc_contacts_are_admitted_at_their_qualified_cells() -> Result<(), Box<dyn Error>> {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let regions: BTreeSet<_> = CONTACTS
        .iter()
        .map(|&(_, x, y, _, _)| (x / 256, y / 256))
        .collect();
    let project = project_regions(root, &regions.into_iter().collect::<Vec<_>>())?;
    let (resolver, palette) = registry(root, &project)?;
    let input = Input {
        regions: &project.regions,
        palette: &palette,
        identity: Identity::default(),
        world: world_extent(root)?,
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &project.families,
    };
    let (spawns, _) = realize_spawns(&input, &resolver)?;
    let (table, report) = realize_npcs(&input, &resolver, &spawns)?;
    let returning_service = "oteryn:service.travel.captain_max_calassa";
    assert!(project.families.routes.iter().any(|route| {
        route.service == returning_service
            && route.route == "liberty_bay"
            && route.destination == (32298, 32896, 6)
    }));
    assert!(
        !report
            .routes_held
            .iter()
            .any(|route| route.service == returning_service),
        "the Calassa return route must remain enterable"
    );
    let by_key: BTreeMap<_, _> = table
        .npcs
        .iter()
        .map(|npc| (npc.key.as_str(), npc.placements.as_slice()))
        .collect();

    let mut refused = Vec::new();
    let mut expected_counts = BTreeMap::new();
    for &(key, x, y, floor, facing) in CONTACTS {
        *expected_counts.entry(key).or_insert(0) += 1;
        let native_floor = -i8::try_from(floor)?;
        if !by_key.get(key).is_some_and(|placements| {
            placements
                .iter()
                .any(|p| (p.x, p.y, p.floor, p.direction) == (x, y, native_floor, facing))
        }) {
            let reason = report
                .held
                .iter()
                .find(|held| held.npc == key && held.cell == (x, y, floor));
            refused.push((key, x, y, floor, reason.map(|held| held.reason)));
        }
    }
    assert!(
        refused.is_empty(),
        "regional contacts refused: {refused:#?}"
    );
    Ok(())
}

#[test]
fn regional_contacts_leave_native_passage_arrivals_enterable() -> Result<(), Box<dyn Error>> {
    use oteryn_world_bundle_compiler::project::TravelRoute;

    #[derive(serde::Deserialize)]
    struct Overlay {
        key: String,
        x: u16,
        y: u16,
        floor: u8,
    }

    const PASSAGES: [&str; 2] = [
        "oteryn:service.travel.jack_fate/goroma",
        "oteryn:service.travel.dalbrect/isle_of_kings",
    ];
    let routes: Vec<Overlay> = serde_json::from_str(include_str!(
        "../../../crates/gameplay/content/src/content/npc_catalogue/travel_routes.json"
    ))?;
    let routes: Vec<_> = routes
        .into_iter()
        .filter(|row| PASSAGES.contains(&row.key.as_str()))
        .collect();
    assert_eq!(routes.len(), PASSAGES.len());
    let regions: Vec<_> = routes
        .iter()
        .map(|row| (row.x / 256, row.y / 256))
        .collect();
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let mut project = project_regions(root, &regions)?;
    for row in routes {
        let (service, route) = row.key.rsplit_once('/').ok_or("native passage key")?;
        project.families.routes.push(TravelRoute {
            service: service.into(),
            route: route.into(),
            destination: (row.x, row.y, row.floor),
        });
    }
    let (resolver, palette) = registry(root, &project)?;
    let input = Input {
        regions: &project.regions,
        palette: &palette,
        identity: Identity::default(),
        world: world_extent(root)?,
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &project.families,
    };
    let (spawns, _) = realize_spawns(&input, &resolver)?;
    let (table, report) = realize_npcs(&input, &resolver, &spawns)?;
    let blocked: Vec<_> = report
        .routes_held
        .iter()
        .filter(|hold| PASSAGES.contains(&format!("{}/{}", hold.service, hold.route).as_str()))
        .collect();
    assert!(
        blocked.is_empty(),
        "native passage arrivals held: {blocked:?}"
    );
    for (key, x, y) in [
        ("oteryn:npc.jack_fate_goroma", 32161, 32559),
        ("oteryn:npc.captain_jack", 32190, 31958),
    ] {
        assert!(
            table.npcs.iter().any(|npc| {
                npc.key == key
                    && npc
                        .placements
                        .iter()
                        .any(|p| (p.x, p.y, p.floor) == (x, y, -6))
            }),
            "{key} must still be available on the dock"
        );
    }
    Ok(())
}
