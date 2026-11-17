use super::*;
use crate::durability::house_scope_handoff::HouseId;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use oteryn_gameplay_housing::house_management::HouseManagementList;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn query_house_acl(
        &self,
        fence: CurrentCharacterItemFence,
        town: &str,
        read: wire::HouseAclRead,
        content_digest: [u8; 32],
        guildhalls: bool,
    ) -> Option<Vec<u8>> {
        let refuse = || Some(wire::encode_house_result(D::Rejected, 0));
        let Some(record) = catalogue()?
            .iter()
            .find(|h| h.identity.key == read.house && h.town.key == town)
        else {
            return refuse();
        };
        if record.kind == CatalogueKind::Guildhall && !guildhalls {
            return refuse();
        }
        let Some(house) = HouseId::new(self.world_id, &read.house) else {
            return refuse();
        };
        let list = match read.selector {
            wire::HouseAclSelector::Guest => HouseManagementList::Guest,
            wire::HouseAclSelector::Subowner => HouseManagementList::Subowner,
            wire::HouseAclSelector::Door { x, y, floor } => {
                let cell = crate::content::LogicalCell {
                    x: i32::from(x),
                    y: i32::from(y),
                    z: i32::from(floor),
                };
                if let Some(map) = self.bundle_map() {
                    if map
                        .facts
                        .house_door(
                            crate::map::overlay::TilePos { x, y, floor },
                            record.provenance.engine_house.house_id,
                        )
                        .is_none()
                    {
                        return refuse();
                    }
                } else {
                    let Some(placement) = self
                        .movement_cells()
                        .house_tiles()
                        .house_at(self.movement_cells().scope(), cell)
                        .ok()
                        .flatten()
                    else {
                        return refuse();
                    };
                    if placement.house().key != read.house || placement.door_at(cell).is_none() {
                        return refuse();
                    }
                }
                HouseManagementList::Door { x, y, floor }
            }
        };
        match self
            .root
            .read_house_acl(
                self.character,
                self.holder,
                fence,
                house,
                list,
                content_digest,
            )
            .await
        {
            Ok(view) => wire::encode_house_acl_snapshot(&wire::HouseAclSnapshot {
                revision: view.revision,
                members: view.members,
            })
            .ok(),
            Err(crate::durability::house_management::HouseManagementError::Unavailable(_)) => None,
            Err(_) => refuse(),
        }
    }
}
