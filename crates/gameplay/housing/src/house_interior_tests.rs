#![allow(clippy::expect_used)]

use super::*;
use oteryn_gameplay_domain::domain::WorldId;

const HOUSE: &str = "oteryn:content.house.thais_1";

#[derive(Clone)]
struct HouseInteriorAuditChange {
    item_instance_id: ItemInstanceId,
    actor: CharacterId,
    effect: HouseInteriorEffect,
}

impl HouseInteriorAuditChange {
    fn validate(&self) -> Result<(), HouseInteriorError> {
        self.effect.validate(self.item_instance_id, self.actor)
    }
}

fn uuid(seed: u8) -> [u8; 16] {
    let mut value = [seed; 16];
    value[6] = 0x70 | (seed & 0x0f);
    value[8] = 0x80 | (seed & 0x3f);
    value
}

fn actor(seed: u8) -> CharacterId {
    CharacterId::from_bytes(uuid(seed)).expect("valid immutable house fixture")
}

fn change() -> HouseInteriorAuditChange {
    let item = ItemInstanceId::from_bytes(uuid(3)).expect("valid immutable house fixture");
    HouseInteriorAuditChange {
        item_instance_id: item,
        actor: actor(2),
        effect: HouseInteriorEffect::Place {
            destination: HouseInteriorLocation {
                house: HouseId::new(
                    WorldId::from_bytes(uuid(1)).expect("valid immutable house fixture"),
                    HOUSE,
                )
                .expect("valid immutable house fixture"),
                spatial_position: vec![1, 2, 3],
                stack_ordinal: 1,
                scope_ownership_generation: 1,
                placement_transaction_id: uuid(4),
            },
            provenance: HousingReclaimProvenance {
                house: HouseId::new(
                    WorldId::from_bytes(uuid(1)).expect("valid immutable house fixture"),
                    HOUSE,
                )
                .expect("valid immutable house fixture"),
                item_instance_id: item,
                reclaim_subject: actor(2),
                placement_transaction_id: uuid(4),
                revision: 1,
            },
        },
    }
}

fn tile_move() -> HouseInteriorAuditChange {
    let mut value = change();
    let HouseInteriorEffect::Place {
        destination: source,
        provenance,
    } = value.effect
    else {
        unreachable!()
    };
    let mut destination = source.clone();
    destination.spatial_position = vec![4, 5, 6];
    destination.placement_transaction_id = uuid(5);
    let mut after = provenance.clone();
    after.placement_transaction_id = uuid(5);
    after.revision = 2;
    value.effect = HouseInteriorEffect::TileMove {
        source,
        destination,
        provenance_before: provenance,
        provenance_after: after,
    };
    value
}

#[test]
fn house_interior_accepted_effects_and_role_table() {
    let place = change();
    let moved = tile_move();
    let HouseInteriorEffect::Place {
        destination,
        provenance,
    } = place.effect.clone()
    else {
        unreachable!()
    };
    let mut take = place.clone();
    take.effect = HouseInteriorEffect::Take {
        source: destination,
        provenance,
    };
    for effect in [&place, &moved, &take] {
        assert_eq!(effect.validate(), Ok(()));
        for role in [
            HouseItemRole::Owner,
            HouseItemRole::Subowner,
            HouseItemRole::Guest,
        ] {
            assert_eq!(role.check_policy(effect.actor, &effect.effect), Ok(()));
        }
    }
    for role in [HouseItemRole::Owner, HouseItemRole::Subowner] {
        assert_eq!(role.check_policy(actor(8), &take.effect), Ok(()));
    }
    assert_eq!(
        HouseItemRole::Guest.check_policy(actor(8), &take.effect),
        Err(HouseInteriorError::NoAccess)
    );
    assert_eq!(
        HouseItemRole::Guest.check_policy(actor(8), &moved.effect),
        Ok(())
    );
    assert_eq!(HOUSE_INTERIOR_ITEMS_MAX, 2_000);
}

#[test]
fn house_interior_unknown_access_role_is_closed() {
    for role in ["", "owner", "ADMIN", "OWNER "] {
        assert_eq!(HouseItemRole::from_access_role(role), None);
    }
    assert_eq!(
        HouseItemRole::from_access_role("OWNER"),
        Some(HouseItemRole::Owner)
    );
    assert_eq!(
        HouseItemRole::from_access_role("SUBOWNER"),
        Some(HouseItemRole::Subowner)
    );
    assert_eq!(
        HouseItemRole::from_access_role("GUEST"),
        Some(HouseItemRole::Guest)
    );
}

#[test]
fn house_interior_location_bounds() {
    let HouseInteriorEffect::Place { destination, .. } = change().effect else {
        unreachable!()
    };
    for position in [vec![], vec![1; 129]] {
        let mut bad = destination.clone();
        bad.spatial_position = position;
        assert_eq!(bad.validate(), Err(HouseInteriorError::InvalidLocation));
    }
    let mut edge = destination.clone();
    edge.spatial_position = vec![1; 128];
    edge.stack_ordinal = u64::MAX;
    assert_eq!(edge.validate(), Ok(()));
    for field in 0..3 {
        let mut bad = destination.clone();
        match field {
            0 => bad.stack_ordinal = 0,
            1 => bad.scope_ownership_generation = 0,
            _ => bad.placement_transaction_id = [0; 16],
        }
        assert_eq!(bad.validate(), Err(HouseInteriorError::InvalidLocation));
    }
}

#[test]
fn house_interior_placement_binds_item_placer_transaction_and_revision() {
    for mutation in 0..5 {
        let mut bad = change();
        let HouseInteriorEffect::Place { provenance, .. } = &mut bad.effect else {
            unreachable!()
        };
        match mutation {
            0 => {
                provenance.item_instance_id =
                    ItemInstanceId::from_bytes(uuid(9)).expect("valid immutable house fixture")
            }
            1 => provenance.reclaim_subject = actor(9),
            2 => provenance.placement_transaction_id = uuid(9),
            3 => provenance.revision = 0,
            _ => {
                provenance.house = HouseId::new(
                    WorldId::from_bytes(uuid(1)).expect("valid immutable house fixture"),
                    "oteryn:content.house.other",
                )
                .expect("valid immutable house fixture")
            }
        }
        assert_eq!(bad.validate(), Err(HouseInteriorError::InvalidProvenance));
    }
}

#[test]
fn house_interior_tile_move_cannot_substitute_reclaim_or_generation() {
    for mutation in 0..5 {
        let mut bad = tile_move();
        let HouseInteriorEffect::TileMove {
            source,
            destination,
            provenance_after,
            ..
        } = &mut bad.effect
        else {
            unreachable!()
        };
        match mutation {
            0 => provenance_after.reclaim_subject = actor(9),
            1 => provenance_after.revision = 3,
            2 => destination.scope_ownership_generation = 2,
            3 => {
                destination.placement_transaction_id = source.placement_transaction_id;
                provenance_after.placement_transaction_id = source.placement_transaction_id;
            }
            _ => {
                provenance_after.item_instance_id =
                    ItemInstanceId::from_bytes(uuid(9)).expect("valid immutable house fixture")
            }
        }
        assert_eq!(bad.validate(), Err(HouseInteriorError::InvalidProvenance));
    }
}

#[test]
fn house_interior_tile_move_rejects_cross_house_and_cross_world() {
    for world in [1, 9] {
        let mut bad = tile_move();
        let HouseInteriorEffect::TileMove {
            destination,
            provenance_after,
            ..
        } = &mut bad.effect
        else {
            unreachable!()
        };
        let key = if world == 1 {
            "oteryn:content.house.other"
        } else {
            HOUSE
        };
        destination.house = HouseId::new(
            WorldId::from_bytes(uuid(world)).expect("valid immutable house fixture"),
            key,
        )
        .expect("valid immutable house fixture");
        provenance_after.house = destination.house.clone();
        assert_eq!(bad.validate(), Err(HouseInteriorError::CrossHouseMove));
    }
}

#[test]
fn house_interior_provenance_revision_cannot_wrap() {
    let mut bad = tile_move();
    let HouseInteriorEffect::TileMove {
        provenance_before,
        provenance_after,
        ..
    } = &mut bad.effect
    else {
        unreachable!()
    };
    provenance_before.revision = u64::MAX;
    provenance_after.revision = 1;
    assert_eq!(bad.validate(), Err(HouseInteriorError::InvalidProvenance));
}
