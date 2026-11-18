//! Independent GlobalEvent registered by Crystal at 201189f8,
//! globalevents_pits_of_inferno_bosses.lua:1–24.
use super::{
    CatalogueAmount, CatalogueArea, CatalogueEnd, CatalogueEntry, CatalogueKind, CataloguePosition,
    CatalogueRaid, CatalogueSchedule, CatalogueSpawnMode, CatalogueVariant, CatalogueWave,
    RaidCatalogue, RaidError,
};

pub(super) fn raid(catalogue: &RaidCatalogue) -> Result<CatalogueRaid, RaidError> {
    // The script draws one entry from this source-ordered list. These bindings
    // address existing world tiles and creature definitions.
    let spawns = [
        ("massacre", 32876, 32270),
        ("dracola", 32837, 32309),
        ("the_imperor", 32907, 32215),
        ("mr_punish", 32763, 32243),
        ("countess_sorrow", 32802, 32333),
        ("the_plasmother", 32845, 32332),
        ("the_handmaiden", 32785, 32290),
    ];
    let variants = spawns
        .into_iter()
        .map(|(name, x, y)| {
            let creature = format!("oteryn:creature.{name}");
            let position = CataloguePosition { x, y, floor: 15 };
            let region_path = format!(
                "content/world/placements/region-z15-x{}-y{}.b3",
                x / 256,
                y / 256
            );
            let bound_area = catalogue
                .active()
                .flat_map(|record| &record.variants)
                .flat_map(|variant| &variant.waves)
                .flat_map(|wave| &wave.entries)
                .filter(|entry| entry.creature == creature)
                .map(|entry| &entry.area)
                .find_map(|area| {
                    area.regions
                        .iter()
                        .find(|region| region.path == region_path)
                        .map(|region| (area.coordinate_frame.clone(), region.clone()))
                })
                .ok_or(RaidError::InvalidDefinition)?;
            let area = CatalogueArea {
                from: position,
                to: position,
                coordinate_frame: bound_area.0,
                regions: vec![bound_area.1],
            };
            Ok(CatalogueVariant {
                key: name.to_owned(),
                announcements: Vec::new(),
                waves: vec![CatalogueWave {
                    delay_ms: 0,
                    entries: vec![CatalogueEntry {
                        creature,
                        amount: CatalogueAmount { min: 1, max: 1 },
                        area_anchor: format!("raid-anchor:{x}_{y}_15-{x}_{y}_15"),
                        area,
                        area_choices: Vec::new(),
                        boss: true,
                        spawn_mode: CatalogueSpawnMode::Single,
                    }],
                }],
            })
        })
        .collect::<Result<_, RaidError>>()?;
    let raid = CatalogueRaid {
        key: "raid:poi.rare_bosses".to_owned(),
        kind: CatalogueKind::MultiVariant,
        active: true,
        inactive_reasons: Vec::new(),
        schedule: CatalogueSchedule::Periodic {
            interval_ms: 46_800_000,
        },
        variant_group: None,
        variants,
        end: CatalogueEnd::UntilReset,
        requires_world_change: None,
    };
    raid.validate()?;
    Ok(raid)
}
