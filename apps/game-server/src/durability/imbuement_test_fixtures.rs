//! Unchanged raw-row fixtures shared by the pure and native PostgreSQL cases.
//! These values are DATA and never fabricate current authority or elapsed provenance.

use crate::domain::ItemInstanceId;
use crate::domain::imbuement::{SlotBinding, SlotRecord, Tier};
use crate::domain::imbuement_checkpoint::SlotCheckpoint;

pub(super) fn uuid(tag: u8) -> [u8; 16] {
    let mut value = [0; 16];
    value[6] = 0x70;
    value[8] = 0x80;
    value[15] = tag;
    value
}

pub(super) fn item(tag: u8) -> ItemInstanceId {
    ItemInstanceId::from_bytes(uuid(tag)).expect("native UUIDv7 constructor")
}

pub(super) fn binding(tag: u8, slot: u8) -> SlotBinding {
    // Synthetic raw row bindings, not admitted definitions or current authority.
    let mut applying_occurrence = uuid(100);
    applying_occurrence[0] = tag;
    applying_occurrence[1] = slot;
    let mut transaction_id = uuid(101);
    transaction_id[0] = tag;
    transaction_id[1] = slot;
    SlotBinding {
        item: item(tag),
        slot,
        key: format!("fixture:imbue-{slot}"),
        content_revision: "definition-r1".into(),
        category: format!("fixture-category-{slot}"),
        tier: Tier::Basic,
        applying_occurrence,
        transaction_id,
    }
}

pub(super) fn row(binding: SlotBinding, remaining: u64) -> SlotRecord {
    SlotRecord::new(binding, remaining).expect("structural row fixture")
}

pub(super) fn change(tag: u8, slot: u8, before: u64, after: u64) -> SlotCheckpoint {
    SlotCheckpoint {
        before: row(binding(tag, slot), before),
        after: row(binding(tag, slot), after),
    }
}
