use super::TestResult;

use super::fixture::debug;

use super::srx3_activity_source_fixture::SourceScene;

use crate::foundation::{ChannelContentPin, MovementLocalPosition};

pub(super) fn source_world(origin: MovementLocalPosition) -> TestResult<SourceScene> {
    source_world_with_bundle(origin).map(|(scene, _, _)| scene)
}

/// Keep the actual decoded spawn family and original entry activation available
/// to World source regressions without manufacturing a second map or catalogue.
pub(super) fn source_world_with_bundle(
    origin: MovementLocalPosition,
) -> TestResult<(
    SourceScene,
    crate::map::boot::BundleWorld,
    ChannelContentPin,
)> {
    let world = super::fixture::fence(1)?.runtime_scope.world_id();
    let channel = super::fixture::fence(1)?
        .runtime_scope
        .channel_id()
        .ok_or("World Channel")?;
    // Nextest runs each case in a separate process. Resolve the shared artifact
    // before activating native content so the cold pin build runs only once and
    // its compiler allocations are released before the World is booted.
    let (bytes, pin) = if let Some(path) = std::env::var_os("SRX3_DEFAULT_WORLD_BUNDLE") {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let pin = serde_json::from_slice(&std::fs::read(root.join("content/world/pins/oteryn.json"))?)?;
        (std::fs::read(path)?, pin)
    } else {
        super::qc_pinned_world::artifact()?
    };
    let mut boot_pins = super::qc_pinned_world::boot_pins(&pin)?;
    boot_pins.start = crate::map::overlay::TilePos {
        x: u16::try_from(origin.x)?,
        y: u16::try_from(origin.y)?,
        floor: i8::try_from(origin.floor.checked_neg().ok_or("source native floor")?)?,
    };
    // check verifies the published digest and manifest against the actual pin.
    // The fixture does not need a separate whole-bundle visit to rediscover them.
    let checked = crate::map::boot::check(bytes, boot_pins).map_err(debug)?;
    let (controller, baseline) =
        super::death_ml_loss_composed_fixtures::production_generation_with_pin(world, "content-1")?;
    let active = controller.active().ok_or("actual active content")?;
    let native = active
        .native_gameplay()
        .ok_or("actual native definitions")?;
    let index = native.item_index().ok_or("actual item index")?;
    let missing = std::cell::RefCell::new(std::collections::BTreeSet::new());
    let projected = std::cell::RefCell::new(std::collections::BTreeMap::new());
    let boot=checked.boot(world,channel,|key|{
        use crate::content::{ReferenceItemField as F,ReferenceItemType};
        if let Some(cached)=projected.borrow().get(key).copied(){return Some(cached);}
        let (Some(revision), Some(reference)) = (index.revision(key), index.revision(key).and_then(|revision|index.definition_ref("Item",key,revision))) else {
            let mut seen=missing.borrow_mut();
            if seen.len()<16 && seen.insert(key.to_owned()) { eprintln!("SRX3_WHOLE_WORLD_MISSING_ITEM_REFERENCE key={key}"); }
            return None;
        };
        let profile=native.item_policy(key,revision).map(|p|p.record());
        let definition=crate::map::facts::ItemDefinition {
            reference,
            solid:profile.and_then(|p|p.attributes.blocks_movement),
            blocks_projectile:profile.and_then(|p|p.attributes.blocks_projectile).unwrap_or(true),
            pickupable:profile.is_some_and(|p|matches!(&p.semantics.physical,F::Known(v) if v.pickupable==F::Known(true))),
            door:profile.is_some_and(|p|matches!(&p.semantics.classification,F::Known(v) if v.item_type==F::Known(ReferenceItemType::Door))),
        };
        // Only keys independently present in this immutable native index enter
        // the cache; it cannot grow beyond the admitted Item key universe.
        projected.borrow_mut().insert(key.to_owned(),definition);
        Some(definition)
    }).map_err(debug)?;
    let cells = boot
        .movement_cells(baseline.qualified_room().movement_cells())
        .map_err(debug)?;
    let mut room = baseline
        .qualified_room()
        .bundle_view(
            cells,
            boot.frame_binding_digest(world),
            boot.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: origin.x,
                y: origin.y,
                floor: origin.floor,
            },
        )
        .ok_or("actual World room projection")?;
    let mut pin = ChannelContentPin::from_activation(
        world,
        active.activation_sequence(),
        active.identity().server_artifact_digest(),
        active.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (origin.x, origin.y, origin.floor),
    );
    let context = active
        .qualify_published_world_context(&room, &pin)
        .ok_or("whole authenticated source World proof")?;
    // The argument names donor source coordinates. No physical floor sign is
    // guessed: only the authenticated published-frame converter selects the
    // actual runtime entry used when the Character is created below.
    let (x, y, floor) = context
        .source_frame()
        .physical_position([origin.x, origin.y, i32::from(origin.floor)])
        .ok_or("source World physical frame conversion")?;
    room = baseline
        .qualified_room()
        .bundle_view(
            room.movement_cells().clone(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            crate::content::NativeEntryStart { x, y, floor },
        )
        .ok_or("qualified World physical entry")?;
    pin = ChannelContentPin::from_activation(
        world,
        active.activation_sequence(),
        active.identity().server_artifact_digest(),
        active.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (x, y, floor),
    );
    assert!(
        context.matches(&room, &pin),
        "entry conversion preserves the exact current World proof"
    );
    Ok((
        SourceScene {
            controller,
            room,
            pin,
        },
        boot,
        baseline.into_channel_pin(),
    ))
}
