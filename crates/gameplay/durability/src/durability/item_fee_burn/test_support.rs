//! Existing GOLD-FEE fixture builders, moved once for shared native PostgreSQL tests.
use super::*;
use crate::durability::item_transfer::ItemStackClass;

pub type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
pub const WORLD: u8 = 42;
pub const GOLD: &str = "oteryn:item.tibia.i3031";
pub const PLATINUM: &str = "oteryn:item.tibia.i3035";
pub const CRYSTAL: &str = "oteryn:item.tibia.i3043";
pub const BACKPACK_KEY: &str = "oteryn:item.tibia.i2854";
pub const OCCURRED_AT: i64 = 1_790_000_000_000;

pub fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}

pub fn debug(error: impl std::fmt::Debug) -> Box<dyn std::error::Error> {
    format!("{error:?}").into()
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn uuid(seed: u8) -> String {
    format!("'{}'::uuid", hex(&id(seed)))
}

pub fn item(seed: u8, key: &str, revision: &str, quantity: u32) -> String {
    format!(
        "INSERT INTO game_item_instances(item_instance_id, world_id, definition_family, \
           definition_production_key, definition_revision_ref, quantity, lifecycle, \
           minted_transaction_id) \
         VALUES ({}, {}, 'Item', '{key}', '{revision}', {quantity}, 1, {});",
        uuid(seed),
        uuid(WORLD),
        uuid(seed + 1),
    )
}

pub fn definition(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "rev-1".into(),
    }
}

pub fn request(occurrence: u8, fee: u64) -> TestResult<FeeBurnRequest> {
    Ok(FeeBurnRequest {
        cause: FeeBurnCause::CharmUnassign {
            charm: CharmKey::new(format!("oteryn:charm.c{occurrence}")).map_err(debug)?,
            occurrence: CharmCommandOccurrence::from_bytes(id(occurrence)).map_err(debug)?,
        },
        fee_gold_units: fee,
        transaction_id: id(occurrence + 100),
        event_id: id(occurrence + 130),
        occurred_at_unix_ms: OCCURRED_AT,
        server_build_id: "fee-test-build".into(),
        change: FeeChangeFacts {
            item_instance_ids: [id(occurrence + 150), id(occurrence + 170)],
            platinum: definition(PLATINUM),
            gold: definition(GOLD),
            crystal: definition(CRYSTAL),
            backpack: ItemDefinitionFacts {
                weight: None,
                definition: definition(BACKPACK_KEY),
                stack: ItemStackClass::NonStackable,
                container_capacity: Some(20),
                container_slot_equip_pattern: true,
            },
        },
    })
}
