use super::fixture::debug;

use super::TestResult;

use crate::content::native_gameplay::NativeGameplayState;

use crate::content::{ReferenceItemField, ReferenceItemType};

use oteryn_world_bundle_compiler::{bundle, compile, source};

/// Boot exact current Item definition references and policy flags, as
/// node/serve.rs::boot_bundle_items does; no test-wide walkability override.
pub(super) fn source_bundle(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
    native: &NativeGameplayState,
    candidate: &crate::content::bosses::activity_template::BossLeverCandidate,
) -> TestResult<crate::map::boot::BundleWorld> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let anchor = candidate.boss_position.ok_or("source boss position")?;
    let region = (u16::try_from(anchor[0] / 256)?, u16::try_from(anchor[1] / 256)?);
    let project = source::project_regions(&root, &[region])?;
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
    let low = candidate.zone_from.ok_or("source room lower bound")?;
    let high = candidate.zone_to.ok_or("source room upper bound")?;
    let held: std::collections::BTreeSet<_> = compiled
        .diagnostics
        .iter()
        .filter(|diagnostic| {
            let at = [
                i32::from(diagnostic.x),
                i32::from(diagnostic.y),
                -i32::from(diagnostic.floor),
            ];
            let in_room = (low[0]..=high[0]).contains(&at[0])
                && (low[1]..=high[1]).contains(&at[1])
                && (low[2]..=high[2]).contains(&at[2]);
            in_room
                || candidate
                    .lever_positions
                    .iter()
                    .chain(&candidate.entry_positions)
                    .chain(candidate.boss_position.iter())
                    .chain(candidate.exit_position.iter())
                    .any(|anchor| *anchor == at)
        })
        .map(|diagnostic| diagnostic.key.as_str())
        .collect();
    assert!(
        held.is_empty(),
        "a real source boss room cannot silently omit provisional geometry: {held:?}"
    );
    let parity = compile::equivalence(&input, &registry, &compiled.bytes)?;
    assert!(parity.tiles > 0 && parity.entries > 0);
    let start = candidate.lever_positions.first().ok_or("source staging start")?;
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
            x: u16::try_from(start[0])?,
            y: u16::try_from(start[1])?,
            floor: i8::try_from(-start[2])?,
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
