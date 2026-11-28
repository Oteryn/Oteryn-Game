//! ITEM_INSCRIPTION_READ_V1: the stored text of one carried item for the writing window.
//! Custody is the MAIL text writer's, proven under the same session item fence; the read may
//! reconcile the carried death tree but never writes the item text.
use super::{ComposedFreshAdmission, connection::AdmittedSession, item_view::ItemKey};
use crate::content::ReferenceItemField::Known;
use crate::content::native_gameplay::NativeGameplayState;
use crate::domain::item_definition::TypedDefinitionRef;
use crate::durability::item_use::CarriedItemText;
use crate::durability::mail_text_policy::NativeGameplayMailTextPolicyExt;
use oteryn_protocol_oteryn::item_inscription::{
    CAPABILITY_ITEM_INSCRIPTION_ATTRIBUTION_V1, CAPABILITY_ITEM_INSCRIPTION_READ_V1,
    MAX_TEXT_LENGTH, Reply,
};

/// The item's content text profile. A generated Guild Bank receipt uses its
/// fixed body length as the read window bound. MAIL policy controls rewriting.
pub(super) fn reply(
    native: &NativeGameplayState,
    definition: &TypedDefinitionRef,
    text: String,
) -> Option<Reply> {
    // MAIL keeps the original letter text when it stamps the carried item. The
    // stamped definition's zero write limit still permits reading that text.
    if definition.production_key == oteryn_gameplay_items::mail::STAMPED_LETTER_KEY {
        native.item_index()?.definition_ref(
            &definition.family,
            &definition.production_key,
            &definition.revision_ref,
        )?;
        let reply = Reply {
            text,
            max_length: u32::try_from(crate::domain::mail::LETTER_TEXT_SCALARS_MAX).ok()?,
            writable: false,
            writer_name: None,
            written_at_unix_ms: None,
        };
        return reply.valid().then_some(reply);
    }
    let policy = native.item_policy(&definition.production_key, &definition.revision_ref)?;
    let Known(profile) = &policy.record().semantics.readable_writeable else {
        return None;
    };
    if profile.readable != Known(true) {
        return None;
    }
    let writable = native.mail_text_policy(definition).is_ok();
    let max_length = match profile.max_text_length {
        Known(max_length @ 1..=MAX_TEXT_LENGTH) => max_length,
        _ if !writable && matches!(
            definition.production_key.as_str(),
            "oteryn:item.tibia.i21932" | "oteryn:item.tibia.i21933"
        ) => u32::try_from(text.chars().count().max(1)).ok()?,
        _ => return None,
    };
    let reply = Reply {
        text,
        max_length,
        writable,
        writer_name: None,
        written_at_unix_ms: None,
    };
    reply.valid().then_some(reply)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn item_inscription_wire(
        &self,
        admitted: AdmittedSession,
        item: Option<ItemKey>,
        attributed: bool,
    ) -> Option<Vec<u8>> {
        let capability = if attributed {
            CAPABILITY_ITEM_INSCRIPTION_ATTRIBUTION_V1
        } else {
            CAPABILITY_ITEM_INSCRIPTION_READ_V1
        };
        if !admitted
            .continuity
            .selected_capabilities
            .contains(capability)
            || admitted.controller.is_none()
        {
            return None;
        }
        let (Some(fence), Some(ItemKey::Instance(item))) = (admitted.item_fence, item) else {
            return None;
        };
        let native = self.active_generation?.native_gameplay()?;
        let (definition, inscription) = self
            .root
            .read_carried_item_text(
                self.character,
                self.holder,
                fence,
                self.resident_origin(admitted).await,
                item,
            )
            .await
            .ok()??;
        let CarriedItemText {
            text,
            writer_name,
            written_at_unix_ms,
        } = inscription;
        let mut reply = reply(native, &definition, text)?;
        if attributed {
            reply.writer_name = writer_name.filter(|_| written_at_unix_ms.is_some());
            reply.written_at_unix_ms = written_at_unix_ms;
        }
        reply.encode()
    }
}

macro_rules! authority_methods {
    () => {
        fn item_inscription_read(
            &self,
            _admitted: $crate::gameplay_transport::AdmittedSession,
            _item: Option<$crate::gameplay_transport::item_view::ItemKey>,
            _attributed: bool,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn item_inscription_read(
            &self,
            admitted: $crate::gameplay_transport::AdmittedSession,
            item: Option<$crate::gameplay_transport::item_view::ItemKey>,
            attributed: bool,
        ) -> Option<Vec<u8>> {
            self.item_inscription_wire(admitted, item, attributed).await
        }
    };
}
pub(super) use composed_methods;

#[cfg(test)]
mod tests {
    use super::*;

    fn definition(id: u32) -> TypedDefinitionRef {
        TypedDefinitionRef {
            family: "Item".into(),
            production_key: format!("oteryn:item.tibia.i{id}"),
            revision_ref: "definition-r1".into(),
        }
    }

    #[test]
    fn inscription_reply_uses_the_content_maximum_and_the_mail_writer_policy()
    -> Result<(), Box<dyn std::error::Error>> {
        let world = crate::foundation::WorldId::decode(&crate::bestiary_postgres_harness::id(42))?;
        let (controller, _) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let native = controller
            .active()
            .and_then(|active| active.native_gameplay())
            .ok_or("production gameplay activation missing")?;
        // A letter: writable, the accepted MAIL limit, the stored text echoed.
        let letter = reply(native, &definition(3505), "Old inscription".into()).ok_or("letter")?;
        assert_eq!(
            letter,
            Reply {
                text: "Old inscription".into(),
                max_length: 1_999,
                writable: true,
                writer_name: None,
                written_at_unix_ms: None,
            }
        );
        let stamped =
            reply(native, &definition(3506), "Delivered".into()).ok_or("stamped letter")?;
        assert_eq!(
            stamped,
            Reply {
                text: "Delivered".into(),
                max_length: 1_999,
                writable: false,
                writer_name: None,
                written_at_unix_ms: None
            }
        );
        // The write-once document target 2834 shows its text and refuses a rewrite.
        let sealed = reply(native, &definition(2834), "Sealed".into()).ok_or("read-only")?;
        assert!(!sealed.writable);
        assert_eq!(sealed.text, "Sealed");
        for (id, max_length, writable) in [(401, 1023, true), (8748, 511, false)] {
            let length = usize::try_from(max_length)?;
            let inscription = reply(native, &definition(id), "x".repeat(length))
                .ok_or("Crystal inscription")?;
            assert_eq!(inscription.max_length, max_length);
            assert_eq!(inscription.writable, writable);
            assert_eq!(inscription.text.len(), length);
            assert!(inscription.text.bytes().all(|byte| byte == b'x'));
        }
        // A stored text beyond the item's maximum, a non-readable item and an unknown
        // revision are refused rather than sent.
        assert!(reply(native, &definition(3507), "x".repeat(80)).is_none());
        assert!(reply(native, &definition(3031), String::new()).is_none());
        let mut unknown = definition(3505);
        unknown.revision_ref = "definition-r2".into();
        assert!(reply(native, &unknown, String::new()).is_none());
        Ok(())
    }
}
