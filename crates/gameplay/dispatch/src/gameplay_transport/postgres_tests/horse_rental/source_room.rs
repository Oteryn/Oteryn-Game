//! Actual source B3, with conservative unknown background semantics.
use super::*;
use crate::content::{ReferenceItemField as F, ReferenceItemType};
pub(super) fn world(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    let bytes = std::fs::read(
        std::env::var_os("OTERYN_SRV7_HORSE_BUNDLE").ok_or("actual source horse station bundle")?,
    )?;
    let b = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    let pins = crate::map::BundlePins {
        digest: b.digest,
        project_format_version: b.manifest.identity.project_format_version.clone(),
        world_schema_version: b.manifest.identity.world_schema_version.clone(),
        content_revision: b.manifest.identity.content_revision.clone(),
        production: false,
    };
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let index = native.item_index().ok_or("native Item index")?;
    crate::map::boot::boot(&bytes,&crate::map::boot::BootPins{bundle:pins,map_revision:crate::map::overlay::map_revision(&base),start:crate::map::overlay::TilePos{x:32851,y:32124,floor:-7}},world,channel,|key|{
  let rev=index.revision(key)?;let policy=native.item_policy(key,rev).map(|policy|policy.record());
  // Every key retains the real generation reference. Unqualified background
  // Item semantics remain conservative; the acquired easel is strictly qualified.
  if matches!(key,"oteryn:item.tibia.i31159"|"oteryn:item.tibia.i2174")&&policy.is_none(){return None;}

  Some(crate::map::facts::ItemDefinition{reference:index.definition_ref("Item",key,rev)?,solid:policy.and_then(|p|p.attributes.blocks_movement),
   blocks_projectile:policy.and_then(|p|p.attributes.blocks_projectile).unwrap_or(true),pickupable:policy.is_some_and(|p|matches!(&p.semantics.physical,F::Known(p) if p.pickupable==F::Known(true))),
   door:policy.is_some_and(|p|matches!(&p.semantics.classification,F::Known(c) if c.item_type==F::Known(ReferenceItemType::Door)))})
 }).map_err(debug)
}
