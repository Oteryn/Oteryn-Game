use super::*;

// Pinned 15.30 appearances (SHA-256 2dfa943b...dae7e2): each of these six
// decorations has unmove=true and neither unpass nor unpath set. The source
// profile producer independently checks the complete flags and XML overrides.
const DECORATIONS: [u32; 6] = [1534, 1535, 1856, 1857, 1865, 1867];

fn on_route_thread(run: fn() -> TestResult) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || run().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "NBROUTE worker panicked")??;
    Ok(())
}

#[test]
fn qualification_nbroute_native_decoration_profiles() -> TestResult {
    on_route_thread(|| {
        let world = WorldId::decode(&v7(1, 2))?;
        let input = native_input()?;
        let room = crate::content::qualify_selected_native_gameplay_room(world, input)?;
        let issuance = crate::content::NativeEntryActivationIssuance {
            world_id: world,
            activation_sequence: 1,
            server_artifact_digest: room.compiled().server_digest(),
            client_artifact_digest: room.compiled().client_digest(),
            frame_binding_digest: room.frame_binding().digest(),
        };
        let mut controller = crate::content::ContentActivationController::new();
        crate::content::activate_native_entry_room_with_gameplay(
            &mut controller,
            &crate::content::NodeBootQuiescence::before_channel_runtime(),
            world,
            &issuance,
            input,
        )?;
        let native = controller
            .active()
            .and_then(|active| active.native_gameplay());
        for id in DECORATIONS {
            let key = format!("oteryn:item.tibia.i{id}");
            let definition = crate::map::boot::native_item_definition(native, &key)
                .ok_or_else(|| format!("missing native Item {key}"))?;
            assert_eq!(definition.solid, Some(false), "donor collision for {key}");
            assert!(
                !definition.blocks_projectile,
                "donor projectile flag for {key}"
            );
            assert!(!definition.pickupable, "fixed donor decoration {key}");
        }
        Ok(())
    })
}

fn endpoint(world: &BundleWorld, from: TilePos, path: &[WalkAction]) -> TestResult<TilePos> {
    path.iter().try_fold(from, |at, direction| {
        destination(world, at, *direction).ok_or_else(|| "route contains a refused step".into())
    })
}

#[test]
fn qualification_nbroute_compiled_town() -> TestResult {
    on_route_thread(|| {
        use oteryn_world_bundle_compiler::{bundle::BuildClass, compile, source};
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        // The same compile-regions inputs and equivalence proof as node boot.
        let project = source::project_regions(&root, &[(126, 125), (126, 126), (127, 127)])?;
        let (registry, palette) = source::registry(&root, &project)?;
        let input = compile::Input {
            regions: &project.regions,
            palette: &palette,
            identity: serde_json::from_slice(&std::fs::read(
                root.join("content/world/pins/oteryn.identity.json"),
            )?)?,
            world: source::world_extent(&root)?,
            build_class: BuildClass::NonProduction,
            draft_areas: oteryn_world_bundle_compiler::project::draft_areas(&project.index)?,
            families: &project.families,
        };
        let compiled = compile::compile(&input, &registry)?;
        compile::equivalence(&input, &registry, &compiled.bytes)?;
        let mut pin: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("content/world/pins/oteryn.json"))?)?;
        pin["digest"] = compiled
            .digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
            .into();
        let world = boot_world(
            compiled.bytes,
            &pin,
            WorldId::decode(&v7(1, 2))?,
            ChannelId::decode(&v7(1, 3))?,
        )?;
        let shrine = TilePos {
            x: 32376,
            y: 32239,
            floor: -7,
        };
        // The donor shrine has unpass=true. P10 uses an adjacent PZ tile, never
        // walks onto the shrine itself or changes its collision to reach it.
        assert!(!world.enterable(shrine));
        let rats = [
            TilePos {
                x: 32368,
                y: 32240,
                floor: -8,
            },
            TilePos {
                x: 32368,
                y: 32242,
                floor: -8,
            },
        ];
        // A failed/finished preceding hunt can leave the next row downstairs.
        // Exercise PZ exit, shrine approach and cleanup from those saved positions too.
        for from in [
            TOWN,
            TilePos {
                x: 32369,
                y: 32241,
                floor: -7,
            },
            rats[0],
            rats[1],
        ] {
            for target in rats {
                let path = route_toward(&world, from, target, |at| at == target)
                    .ok_or_else(|| format!("no rat route: {from:?} -> {target:?}"))?;
                if from.floor != target.floor {
                    assert!(
                        path.iter()
                            .any(|action| matches!(action, WalkAction::Use { .. })),
                        "sewer descent must use the authored grate"
                    );
                }
                assert_eq!(endpoint(&world, from, &path)?, target);
                let back = route_toward(&world, target, from, |at| at == from)
                    .ok_or_else(|| format!("no post-hunt return: {target:?} -> {from:?}"))?;
                assert_eq!(endpoint(&world, target, &back)?, from);
            }
            let path = route_toward(&world, from, shrine, |at| {
                at.floor == shrine.floor
                    && at.x.abs_diff(shrine.x).max(at.y.abs_diff(shrine.y)) <= 1
                    && world
                        .base()
                        .tile(at.x, at.y, at.floor)
                        .is_some_and(|tile| tile.protection_zone())
            })
            .ok_or_else(|| format!("no shrine approach: {from:?} -> {shrine:?}"))?;
            let inside = endpoint(&world, from, &path)?;
            let outside = route_toward(&world, inside, "outside PZ", |at| {
                at.floor == inside.floor
                    && at.x.abs_diff(inside.x).max(at.y.abs_diff(inside.y)) >= 4
                    && world
                        .base()
                        .tile(at.x, at.y, at.floor)
                        .is_some_and(|tile| !tile.protection_zone())
            })
            .ok_or("no post-shrine route outside PZ")?;
            let outside = endpoint(&world, inside, &outside)?;
            let back = route_toward(&world, outside, from, |at| at == from)
                .ok_or("no outside-PZ return")?;
            assert_eq!(endpoint(&world, outside, &back)?, from);
        }
        Ok(())
    })
}
