//! MAIL-1: pure address planning into the existing native recipient owner.
//! This does not qualify posting: Inbox trees and qualified stamp/mailbox
//! content are still missing. No replacement Inbox or test delivery cause.

use crate::bestiary_postgres_harness::{self as fixture, Harness, TestResult, id};
use crate::domain::WorldId;
use crate::domain::item_definition::{ItemDefinitionFacts, ItemStackClass, TypedDefinitionRef};
use crate::domain::mail::{MailText, MailTextClass};
use crate::durability::mail::{lock_mail_recipient_root, mail_recipient_hint};
use oteryn_gameplay_items::item_transfer::InventoryItem;
use oteryn_gameplay_items::mail::{
    LETTER_KEY, MAIL_INBOX_ITEMS_MAX, MailPostingInput, MailPostingPlan, MailTreeItem,
    PARCEL_DIRECT_CHILDREN_MAX, PARCEL_HEIGHT_MAX, PARCEL_ITEMS_MAX, STAMPED_LETTER_KEY,
};

#[test]
fn mail_rules_registry_limits_match_pure_rules() -> TestResult {
    use crate::domain::mail::{LABEL_TEXT_SCALARS_MAX, LETTER_TEXT_SCALARS_MAX};

    let registry: serde_json::Value =
        serde_json::from_str(oteryn_protocol_oteryn::RESOURCE_LIMITS_REGISTRY_JSON)?;
    let entries = registry["entries"].as_array().ok_or("missing limits")?;
    for (id, maximum) in [
        ("MAIL0-RL-01", LETTER_TEXT_SCALARS_MAX as u64),
        ("MAIL0-RL-02", LABEL_TEXT_SCALARS_MAX as u64),
        ("MAIL0-RL-03-DIRECT", PARCEL_DIRECT_CHILDREN_MAX as u64),
        ("MAIL0-RL-03-HEIGHT", PARCEL_HEIGHT_MAX as u64),
        ("MAIL0-RL-03-ITEMS", PARCEL_ITEMS_MAX as u64),
        ("MAIL0-RL-05", MAIL_INBOX_ITEMS_MAX),
    ] {
        let entry = entries.iter().find(|entry| entry["id"] == id).ok_or(id)?;
        assert_eq!(entry["hard_maximum"].as_u64(), Some(maximum), "{id}");
    }
    Ok(())
}

#[test]
fn mail_rules_postgres_resolve_recipient_without_town_or_channel_partition() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    fixture::runtime()?.block_on(async {
        let harness = Harness::create(admin, "mail_rules_recipient", true).await?;
        let result = async {
            let definition = TypedDefinitionRef {
                family: "Item".into(),
                production_key: LETTER_KEY.into(),
                revision_ref: "definition-r1".into(),
            };
            // Explicit rules input, not an admitted/current item or Content
            // capability. Only actual recipient reads/locks are exercised.
            let item = InventoryItem {
                item_instance_id: id(90),
                definition: definition.clone(),
                quantity: 1,
            };
            let facts = ItemDefinitionFacts {
                definition,
                weight: Some(50),
                stack: ItemStackClass::NonStackable,
                container_capacity: None,
                container_slot_equip_pattern: false,
            };
            let world = WorldId::from_bytes(id(fixture::WORLD))?;
            for body in [
                " Fixture Hero \nThais",
                "Fixture Hero\nVenore",
                "Fixture Hero\ninvalid town 🦀",
                "Fixture Hero",
            ] {
                let text = MailText::parse(MailTextClass::Letter, body)
                    .map_err(|_| "invalid mail text fixture")?;
                let nodes = [MailTreeItem {
                    item: &item,
                    facts: &facts,
                    parent: None,
                    ordinal: 1,
                    text: Some(&text),
                }];
                let plan = MailPostingPlan::try_from(MailPostingInput {
                    root: item.item_instance_id,
                    items: &nodes,
                    root_has_store_owner: false,
                    sender_junior: false,
                    recipient_junior: false,
                    inbox_items: 0,
                })
                .map_err(|error| format!("mail rule refusal: {error:?}"))?;
                assert_eq!(plan.stamped_production_key, STAMPED_LETTER_KEY);
                let mut tx = harness.pool.begin().await?;
                let recipient = mail_recipient_hint(&mut tx, world, &plan.recipient)
                    .await?
                    .ok_or("missing native recipient")?;
                assert_eq!(recipient.as_bytes(), &id(fixture::CHARACTER));
                assert!(
                    lock_mail_recipient_root(&mut tx, world, recipient, &plan.recipient).await?
                );
                // A World mismatch is still privacy-safe, even for this name.
                assert!(
                    mail_recipient_hint(&mut tx, WorldId::from_bytes(id(91))?, &plan.recipient)
                        .await?
                        .is_none()
                );
                tx.rollback().await?;
            }
            Ok(())
        }
        .await;
        harness.cleanup().await?;
        if result.is_ok() {
            eprintln!("MAIL-1 PostgreSQL recipient rules: database body and cleanup completed");
        }
        result
    })
}
