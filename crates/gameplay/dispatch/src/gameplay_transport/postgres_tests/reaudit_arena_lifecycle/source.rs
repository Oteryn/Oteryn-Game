//! Genuine Crystal B3 hall geometry, with the production native Item flags.
//! Crystal 201189f data-global/lib/quests/svargrond_arena.lua:10 identifies
//! this source hall; working gathering cell comes from its unchanged B3 geometry.
use super::super::TestResult;
use super::super::fixture::debug;
use crate::content::native_gameplay::NativeGameplayState;
use crate::content::{ReferenceItemField, ReferenceItemType};
use oteryn_world_bundle_compiler::{bundle, compile, source};

pub(super) const ENTRY: crate::foundation::MovementLocalPosition =
    crate::foundation::MovementLocalPosition {
        x: 32222,
        y: 31093,
        floor: 6,
    };

pub(super) fn world(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
    native: &NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let project = source::project_regions(&root, &[(125, 121)])?;
    let (registry, palette) = source::registry(&root, &project)?;
    let identity: bundle::Identity = serde_json::from_slice(&source::read(
        &root.join("content/world/pins/oteryn.identity.json"),
    )?)?;
    let input = compile::Input {
        regions: &project.regions,
        palette: &palette,
        identity: identity.clone(),
        world: source::world_extent(&root)?,
        build_class: bundle::BuildClass::NonProduction,
        draft_areas: vec![],
        families: &project.families,
    };
    let compiled = compile::compile(&input, &registry)?;
    let low = [ENTRY.x - 6, ENTRY.y - 6, i32::from(ENTRY.floor)];
    let high = [ENTRY.x + 6, ENTRY.y + 6, i32::from(ENTRY.floor)];
    let held: std::collections::BTreeSet<_> = compiled
        .diagnostics
        .iter()
        .filter(|diagnostic| {
            let at = [
                i32::from(diagnostic.x),
                i32::from(diagnostic.y),
                -i32::from(diagnostic.floor),
            ];
            (low[0]..=high[0]).contains(&at[0])
                && (low[1]..=high[1]).contains(&at[1])
                && (low[2]..=high[2]).contains(&at[2])
        })
        .map(|diagnostic| diagnostic.key.as_str())
        .collect();
    assert!(
        held.is_empty(),
        "a native Drome gathering area cannot silently omit provisional geometry: {held:?}"
    );
    let parity = compile::equivalence(&input, &registry, &compiled.bytes)?;
    assert!(parity.tiles > 0 && parity.entries > 0);
    let pins = crate::map::boot::BootPins {
        bundle: crate::map::BundlePins {
            digest: compiled.digest,
            project_format_version: identity.project_format_version,
            world_schema_version: identity.world_schema_version,
            content_revision: identity.content_revision,
            production: false,
        },
        map_revision: crate::map::overlay::internal_map_revision(
            &compiled
                .digest
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
        ),
        start: crate::map::overlay::TilePos {
            x: 32222,
            y: 31093,
            floor: -6,
        },
    };
    let index = native.item_index().ok_or("active native Item index")?;
    crate::map::boot::boot(&compiled.bytes, &pins, world, channel, |key| {
        let revision = index.revision(key)?;
        let reference = index.definition_ref("Item", key, revision)?;
        let profile = native
            .item_policy(key, revision)
            .map(|policy| policy.record());
        let pickupable = profile.is_some_and(|profile| {
            matches!(
                &profile.semantics.physical, ReferenceItemField::Known(physical)
                    if physical.pickupable == ReferenceItemField::Known(true)
            )
        });
        let door = profile.is_some_and(|profile| matches!(
            &profile.semantics.classification, ReferenceItemField::Known(classification)
                if classification.item_type == ReferenceItemField::Known(ReferenceItemType::Door)
        ));
        Some(crate::map::facts::ItemDefinition {
            reference,
            solid: profile.and_then(|profile| profile.attributes.blocks_movement),
            blocks_projectile: profile
                .and_then(|profile| profile.attributes.blocks_projectile)
                .unwrap_or(true),
            pickupable,
            door,
        })
    })
    .map_err(debug)
}
