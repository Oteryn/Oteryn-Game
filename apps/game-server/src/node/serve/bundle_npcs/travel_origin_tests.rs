use super::tests::channel_runtime_with;
use super::*;

#[test]
fn reference_travel_origins_boot_with_their_source_routes() -> Result<(), String> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| check_reference_travel_origins().map_err(|error| error.to_string()))
        .map_err(|error| error.to_string())?
        .join()
        .unwrap_or_else(|failure| std::panic::resume_unwind(failure))
}

fn check_reference_travel_origins() -> Result<(), Box<dyn std::error::Error>> {
    use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
    use oteryn_world_bundle_compiler::{
        bundle::{BuildClass, Identity},
        compile::{Input, compile},
        source,
    };

    // Crystal 201189f8, Global world-npc.xml. Brodrosch and Harava stand one
    // cell from their donor anchors to keep the authored landings clear.
    let origins = [
        (
            "anderson",
            (32127u16, 31660u16, 7u8),
            &["folda", "tibia", "vega"][..],
        ),
        ("brodrosch", (32659, 31958, 15), &["gnomprona"]),
        (
            "captain_bluebear",
            (32310, 32209, 6),
            &[
                "ab_dendriel",
                "edron",
                "krailos",
                "liberty_bay",
                "oramond",
                "port_hope",
                "roshamuul",
                "svargrond",
                "venore",
            ],
        ),
        (
            "captain_breezelda",
            (32341, 31109, 6),
            &["carlin", "thais", "venore"],
        ),
        ("captain_chelop", (33494, 32568, 7), &["thais", "town"]),
        (
            "captain_cookie",
            (32804, 31277, 6),
            &["liberty_bay", "passage"],
        ),
        ("captain_corsarah", (34010, 32475, 6), &["venore"]),
        (
            "captain_fearless",
            (32957, 32021, 6),
            &[
                "ab_dendriel",
                "ankrahmun",
                "carlin",
                "edron",
                "gray_island",
                "issavi",
                "krailos",
                "liberty_bay",
                "port_hope",
                "svargrond",
                "thais",
            ],
        ),
        (
            "captain_greyhound",
            (32387, 31822, 6),
            &["ab_dendriel", "edron", "svargrond", "thais", "venore"],
        ),
        ("captain_gulliver", (33479, 31986, 7), &["issavi", "thais"]),
        (
            "captain_harava",
            (33902, 31463, 6),
            &["darashia", "krailos", "oramond", "venore"],
        ),
        ("captain_max_calassa", (31915, 32710, 6), &["liberty_bay"]),
        (
            "captain_pelagia",
            (33491, 31710, 6),
            &["darashia", "edron", "issavi", "oramond", "thais", "venore"],
        ),
        (
            "captain_seagull",
            (32735, 31670, 6),
            &["carlin", "edron", "gray_island", "thais", "venore"],
        ),
    ];
    let regions: BTreeSet<_> = origins
        .iter()
        .map(|(_, (x, y, _), _)| (x / 256, y / 256))
        .collect();
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let project = source::project_regions(root, &regions.into_iter().collect::<Vec<_>>())?;
    let (resolver, palette) = source::registry(root, &project)?;
    let bundle = compile(
        &Input {
            regions: &project.regions,
            palette: &palette,
            identity: Identity::default(),
            world: source::world_extent(root)?,
            build_class: BuildClass::NonProduction,
            draft_areas: Vec::new(),
            families: &project.families,
        },
        &resolver,
    )?;
    let content = root.join("content");
    let catalogue = NpcDataCatalogue::load(
        &content,
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )?;
    let world = content.join("world");
    let boot = prepare(
        Some(read(&bundle.bytes).map_err(|error| error.to_string())?),
        Some((&catalogue, &world)),
        true,
    )
    .map_err(|error| error.to_string())?;
    let mut runtime = channel_runtime_with(bundle.npcs.written);
    place(&mut runtime, boot.as_ref(), None).map_err(|error| error.to_string())?;
    let actors = runtime.visible_entities();
    let absent: Vec<_> = origins
        .iter()
        .filter(|(stem, (x, y, z), _)| {
            let key = format!("oteryn:npc.{stem}");
            !actors.npcs.iter().any(|actor| {
                runtime.npc_key(actor.actor) == Some(key.as_str())
                    && (actor.position.x, actor.position.y, actor.position.floor)
                        == (i32::from(*x), i32::from(*y), i16::from(*z))
            })
        })
        .map(|(stem, _, _)| *stem)
        .collect();
    assert!(
        absent.is_empty(),
        "travel NPCs missing at their authored cells: {absent:?}; compiler holds: {:?}",
        bundle
            .npcs
            .held
            .iter()
            .filter(|hold| absent.contains(&hold.npc.trim_start_matches("oteryn:npc.")))
            .collect::<Vec<_>>()
    );
    for arrival in [(32659, 31957, 15), (33902, 31462, 6)] {
        assert!(
            !bundle
                .npcs
                .routes_held
                .iter()
                .any(|hold| hold.destination == arrival),
            "travel landing {arrival:?} must remain usable"
        );
    }
    let book = talk_book(boot.as_ref()).ok_or("bound NPC conversation book")?;
    for (stem, _, routes) in origins {
        let key = format!("oteryn:npc.{stem}");
        let service = format!("oteryn:service.travel.{stem}");
        let entry = book
            .get(&key)
            .ok_or_else(|| format!("missing {key} in talk book"))?;
        for route in routes {
            assert!(
                entry.travel_routes.iter().any(|entry| {
                    entry.service == service && entry.route.key == *route && !entry.held
                }),
                "missing available travel route {service}/{route}"
            );
        }
    }
    Ok(())
}
