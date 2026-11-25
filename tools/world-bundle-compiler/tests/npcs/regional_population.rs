use std::path::Path;

use oteryn_world_bundle_compiler::source;

use super::*;

const EXPECTED: &[(&str, (u16, u16, i8))] = &[
    // Yalahar
    ("a_grumpy_cyclops", (32822, 31238, -7)),
    ("armenius", (32859, 31325, -7)),
    ("barry", (32884, 31160, -7)),
    ("bruce", (32740, 31113, -7)),
    ("captain_cookie", (32804, 31277, -6)),
    ("chuckles", (32790, 31236, -5)),
    ("dorbin", (32783, 31236, -7)),
    ("ethan", (32879, 31076, -7)),
    ("golem_guardian", (32799, 31190, -7)),
    ("golem_guardian", (32803, 31190, -7)),
    ("gree_dee", (32789, 31236, -6)),
    ("guide_edna", (32804, 31269, -6)),
    ("hal", (32687, 31190, -7)),
    ("julius", (32830, 31276, -7)),
    ("karith", (32818, 31278, -5)),
    ("lisander", (32812, 31248, -7)),
    ("lorietta", (32787, 31277, -7)),
    ("maris", (32648, 31292, -6)),
    ("maritima", (32902, 31167, -13)),
    ("menacing_mummy", (32807, 31246, -7)),
    ("morpel", (32821, 31240, -7)),
    ("mr_west", (32771, 31357, -6)),
    ("oiriz", (32787, 31238, -7)),
    ("oliver", (32894, 31228, -7)),
    ("orc_berserker", (32777, 31237, -7)),
    ("ortheus", (32877, 31087, -6)),
    ("peter", (32856, 31302, -7)),
    ("pugwah", (32810, 31251, -7)),
    ("redward", (32823, 31251, -6)),
    ("reed", (32801, 31103, -7)),
    ("serafin", (32852, 31209, -7)),
    ("soilance", (32821, 31222, -7)),
    ("tamara", (32892, 31061, -7)),
    ("tamerin", (32659, 31211, -8)),
    ("tamoril", (32911, 31018, -5)),
    ("tarak_inner", (32917, 31198, -7)),
    ("tarak_sunken", (32941, 31181, -7)),
    ("timothy", (32787, 31226, -7)),
    ("tony", (32694, 31256, -7)),
    ("vincent", (32790, 31237, -3)),
    ("zarak", (32720, 31288, -7)),
    ("znozel", (32822, 31249, -7)),
    // Farmine
    ("esrik", (33036, 31530, -10)),
    ("esrik", (33037, 31533, -12)),
    ("esrik", (33035, 31531, -14)),
    ("melian", (32983, 31539, -1)),
    ("murim", (33012, 31494, -10)),
    ("ongulf", (33023, 31542, -10)),
    ("ongulf", (33024, 31539, -12)),
    ("ongulf", (33024, 31543, -14)),
    ("pompan", (33037, 31543, -10)),
    ("pompan", (33035, 31543, -12)),
    ("pompan", (33033, 31543, -14)),
    ("prezil", (33027, 31525, -15)),
    ("rabaz", (33009, 31532, -10)),
    ("swolt", (32993, 31542, -10)),
    ("thorgrin", (33025, 31553, -10)),
    ("thorgrin", (33024, 31554, -12)),
    ("thorgrin", (33024, 31554, -14)),
    // Gray Beach
    ("alternative_rock", (33466, 31325, -7)),
    ("eighty", (33452, 31308, -8)),
    ("old_rock_boy", (33457, 31321, -7)),
    ("rock_in_a_hard_place", (33458, 31302, -6)),
    ("rock_steady", (33453, 31329, -7)),
    ("rock_with_a_soft_spot", (33444, 31321, -9)),
    // Rathleton
    ("a_beggar", (33597, 31953, -7)),
    ("alaistar", (33619, 31884, -4)),
    ("azalea", (33593, 31899, -6)),
    ("barazbaz", (33652, 31883, -5)),
    ("barnabas_dee", (33631, 31881, -4)),
    ("chavis", (33548, 31948, -7)),
    ("christine", (33613, 31895, -5)),
    ("doubleday", (33640, 31959, -6)),
    ("dr_merlay", (33579, 31868, -12)),
    ("dregor", (33539, 31950, -14)),
    ("drystan_wildweed", (33703, 31942, -12)),
    ("eathar", (33536, 31948, -13)),
    ("edmund", (33625, 31894, -4)),
    ("elbert", (33583, 31988, -12)),
    ("elliott", (33571, 31908, -8)),
    ("emily", (33611, 31886, -5)),
    ("fayla", (33640, 31898, -4)),
    ("fillias", (33570, 31926, -7)),
    ("flint", (33635, 31883, -6)),
    ("flora", (33654, 31937, -9)),
    ("fyodor", (33639, 31897, -6)),
    ("graham", (33611, 31886, -6)),
    ("hexel", (33558, 31945, -14)),
    ("jacob", (33571, 31985, -9)),
    ("jondrin", (33652, 31885, -4)),
    ("jossel", (33566, 31948, -13)),
    ("klesar", (33542, 31947, -14)),
    ("kromrek", (33554, 31966, -13)),
    ("maro", (33622, 31883, -6)),
    ("marvin", (33662, 31878, -5)),
    ("mr_brandon", (33548, 31864, -7)),
    ("nicholas", (33640, 31898, -5)),
    ("normod", (33550, 31939, -14)),
    ("peremin", (33550, 31963, -14)),
    ("roswitha", (33584, 31942, -7)),
    ("sholley", (33652, 31883, -3)),
    ("silas", (33634, 31883, -5)),
    ("strange_pipe", (33533, 31950, -14)),
    ("strange_pipe_1", (33742, 31933, -15)),
    ("strange_pipe_2", (33541, 31963, -15)),
    ("strange_pipe_3", (33609, 32053, -15)),
    ("terrence", (33645, 31951, -7)),
    ("the_beggar_king", (33573, 31983, -8)),
    ("virgil", (33627, 31894, -5)),
    ("wesley", (33611, 31898, -12)),
    ("zeronex", (33608, 31974, -4)),
    // Roshamuul
    ("asnarus", (33542, 32383, -7)),
    ("auron", (33526, 32385, -7)),
    ("captain_chelop", (33494, 32568, -7)),
    ("dream_butterfly", (33716, 32376, -6)),
    ("eruaran", (33618, 32372, -5)),
    ("gederas", (33497, 32549, -7)),
    ("maun", (33522, 32562, -7)),
    ("morax", (33628, 32344, -6)),
    ("mortis", (33536, 32503, -5)),
    ("sandomo", (33507, 32554, -7)),
    ("siestaar", (33559, 32388, -7)),
    ("zedrulon_the_fallen", (33515, 32363, -6)),
    // Issavi
    ("alyxo", (33941, 31465, -7)),
    ("ambassador_of_rathleton", (33872, 31487, -3)),
    ("atur", (33948, 31506, -7)),
    ("captain_harava", (33902, 31463, -6)),
    ("eliyas", (33915, 31519, -7)),
    ("eshaya", (33913, 31497, -6)),
    ("faloriel", (33910, 31514, -7)),
    ("guard_bazaya", (33937, 31527, -7)),
    ("guard_inurta", (33868, 31498, -4)),
    ("guard_saros", (33919, 31492, -7)),
    ("guard_senet", (33866, 31494, -7)),
    ("guide_meruka", (33891, 31474, -7)),
    ("inkaef", (33904, 31518, -7)),
    ("iptar_sin", (33924, 31476, -5)),
    ("kallimae", (33935, 31484, -5)),
    ("khanna", (33916, 31506, -7)),
    ("moe", (33909, 31495, -5)),
    ("narsai", (33913, 31530, -7)),
    ("ninev", (33871, 31528, -7)),
    ("ninos", (33904, 31498, -7)),
    ("ramina", (33903, 31495, -7)),
    ("tamed_lion", (33874, 31487, -4)),
    ("tanyt", (33957, 31513, 0)),
    ("taya", (33811, 31464, -7)),
    ("the_empress", (33868, 31497, -3)),
    ("the_librarian", (33873, 31494, -5)),
    ("yonan", (33922, 31513, -7)),
    // Marapur
    ("amra", (33800, 32756, -5)),
    ("asphota", (33777, 32844, -6)),
    ("dhira", (33726, 32861, -7)),
    ("giri", (33819, 32768, -4)),
    ("haani", (33811, 32771, -4)),
    ("isika", (33773, 32840, -6)),
    ("javala", (33889, 32654, -8)),
    ("nilavarna", (33729, 32824, -6)),
    ("nipuna", (33778, 32837, -6)),
    ("saraki", (33722, 32764, -4)),
    ("sessek", (33741, 32769, -5)),
    ("sharai", (33874, 32661, -5)),
    ("sissek", (33794, 32755, -5)),
    ("sundara", (33822, 32773, -4)),
    ("tarisu", (33799, 32860, -6)),
    ("udu", (33849, 32788, -2)),
    ("ziyad", (33805, 32768, -2)),
];

#[test]
fn regional_npcs_compile_at_their_source_cells() -> TestResult {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let regions: std::collections::BTreeSet<_> = EXPECTED
        .iter()
        .map(|(_, (x, y, _))| (x / 256, y / 256))
        .collect();
    let project = source::project_regions(root, &regions.into_iter().collect::<Vec<_>>())?;
    let (resolver, palette) = source::registry(root, &project)?;
    let input = Input {
        regions: &project.regions,
        palette: &palette,
        identity: Identity::default(),
        world: source::world_extent(root)?,
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &project.families,
    };
    let (spawns, _) = realize_spawns(&input, &resolver)?;
    let (table, report) = realize_npcs(&input, &resolver, &spawns)?;
    let missing: Vec<_> = EXPECTED
        .iter()
        .filter(|(stem, cell)| {
            let key = format!("oteryn:npc.{stem}");
            !table.npcs.iter().any(|npc| {
                npc.key == key && npc.placements.iter().any(|p| (p.x, p.y, p.floor) == *cell)
            })
        })
        .collect();
    assert!(
        missing.is_empty(),
        "missing regional placements: {missing:?}; holds: {:?}",
        report
            .held
            .iter()
            .filter(|hold| {
                missing
                    .iter()
                    .any(|(stem, _)| hold.npc.strip_prefix("oteryn:npc.") == Some(stem))
            })
            .collect::<Vec<_>>()
    );
    Ok(())
}
