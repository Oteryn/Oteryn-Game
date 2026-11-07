//! S06 natural Feverish source Bundle: source geometry and loot remain unchanged.
use super::TestResult;
use crate::foundation::MovementLocalPosition;
pub(super) fn positions() -> TestResult<Option<(MovementLocalPosition, MovementLocalPosition)>> {
    if super::acquisition_natural::creature().is_some() {
        return super::acquisition_natural::positions();
    }
    if !super::cosmetic_source_loot::natural_active() {
        return Ok(None);
    }
    let mut path =
        std::env::var_os("OTERYN_SRV7_S06_BUNDLE").ok_or("actual source compiler Bundle")?;
    path.push(".cells.json");
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
    assert_eq!(value["raid"], "raid:venore.feverishcitizens");
    let decode = |key: &str| -> TestResult<MovementLocalPosition> {
        let a = value[key].as_array().ok_or("actual source coordinate")?;
        let x = i32::try_from(a[0].as_u64().ok_or("x")?)?;
        let y = i32::try_from(a[1].as_u64().ok_or("y")?)?;
        let floor = i16::try_from(a[2].as_u64().ok_or("floor")?)?;
        assert!((32861..=33012).contains(&x) && (32052..=32126).contains(&y) && floor == 6);
        Ok(MovementLocalPosition { x, y, floor })
    };
    let player = decode("player")?;
    let creature = decode("creature")?;
    assert_eq!((player.x + 1, player.y), (creature.x, creature.y));
    Ok(Some((player, creature)))
}
pub(super) fn source_room(
    original: &crate::content::QualifiedNativeEntryRoom,
    native: &crate::content::native_gameplay::NativeGameplayState,
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
) -> TestResult<crate::content::QualifiedNativeEntryRoom> {
    use super::fixture::debug;
    let index = native.item_index().ok_or("Item index")?;
    let bytes = std::fs::read(
        std::env::var_os(if super::stampor_natural_death::active() {
            "OTERYN_SRV7_STAMPOR_BUNDLE"
        } else if super::spike_natural_death::active() {
            "OTERYN_SRV7_SUPPORT_BUNDLE"
        } else if super::acquisition_natural::creature().is_some() {
            "OTERYN_SRV7_ACQUISITION_BUNDLE"
        } else {
            "OTERYN_SRV7_S06_BUNDLE"
        })
        .ok_or("actual compiler Bundle path")?,
    )?;
    let bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    let pins = crate::map::BundlePins {
        digest: bundle.digest,
        project_format_version: bundle.manifest.identity.project_format_version.clone(),
        world_schema_version: bundle.manifest.identity.world_schema_version.clone(),
        content_revision: bundle.manifest.identity.content_revision.clone(),
        production: false,
    };
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let (player, _) = positions()?.ok_or("natural source positions")?;
    let start = crate::map::overlay::TilePos {
        x: u16::try_from(player.x)?,
        y: u16::try_from(player.y)?,
        floor: -i8::try_from(player.floor)?,
    };
    let map=crate::map::boot::boot(&bytes,&crate::map::boot::BootPins{bundle:pins,map_revision:crate::map::overlay::map_revision(&base),start},world,channel,|key|{
            let revision=index.revision(key)?;
            let p=native.item_policy(key,revision).map(|p|p.record());
            Some(crate::map::facts::ItemDefinition{
                reference:index.definition_ref("Item",key,revision)?,
                solid:p.and_then(|p|p.attributes.blocks_movement),
                blocks_projectile:p.and_then(|p|p.attributes.blocks_projectile).unwrap_or(true),
                pickupable:p.is_some_and(|p|matches!(&p.semantics.physical,crate::content::ReferenceItemField::Known(p) if p.pickupable==crate::content::ReferenceItemField::Known(true))),door:false,
            })
        }).map_err(debug)?;
    let room = original
        .bundle_view(
            map.movement_cells(original.movement_cells())
                .map_err(debug)?,
            map.frame_binding_digest(world),
            map.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: player.x,
                y: player.y,
                floor: player.floor,
            },
        )
        .ok_or("real Venore natural source bundle view")?;

    Ok(room)
}
