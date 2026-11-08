use super::*;
mod tests {
    use super::*;
    #[test]
    fn forge_resource_confirmation_is_explicit_single_use_and_scope_fenced()
    -> Result<(), &'static str> {
        let mut p = ItemForgePanel {
            selected: true,
            ..Default::default()
        };
        p.resource_dialog.offer(Operation::DustToSlivers, None);
        let serial = p.resource_dialog.serial;
        let mut id = [1; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        let request = p.confirm_resource(serial, id).ok_or("confirm")?;
        assert_eq!(request.operation, Operation::DustToSlivers);
        assert!(p.confirm_resource(serial, id).is_none());
        p.resource_dialog.offer(Operation::DustLimit, None);
        let old = p.resource_dialog.serial;
        p.scope_changed();
        assert!(p.confirm_resource(old, id).is_none());
        assert!(p.resource_dialog.content.is_none());
        Ok(())
    }
}
mod publication_tests {
    use super::*;
    #[test]
    fn forge_resource_receipt_survives_inventory_refresh_but_not_scope_end()
    -> Result<(), &'static str> {
        let mut p = ItemForgePanel {
            selected: true,
            ..Default::default()
        };
        let receipt = reply::Reply::CurrentDust {
            dust: 40,
            limit: 100,
        };
        p.resource_dialog.result(receipt.clone());
        let serial = p.resource_dialog.serial;
        p.observe(vec![oteryn_client_runtime::depot::ItemEntry {
            handle: std::num::NonZeroU64::MIN,
            item_definition_ref: std::num::NonZeroU32::MIN,
            count: std::num::NonZeroU32::MIN,
            sub_type: 0,
        }]);
        assert_eq!(p.resource_dialog.serial, serial);
        assert_eq!(p.resource_dialog.content, Some(Content::Result(receipt)));
        p.scope_changed();
        assert!(p.resource_dialog.content.is_none());
        Ok(())
    }
    #[test]
    fn forge_resource_expired_review_and_invalid_occurrence_never_submit()
    -> Result<(), &'static str> {
        let mut p = ItemForgePanel {
            selected: true,
            ..Default::default()
        };
        p.resource_dialog.offer(Operation::DustLimit, None);
        let serial = p.resource_dialog.serial;
        assert!(p.confirm_resource(serial, [0; 16]).is_none());
        p.resource_dialog.expires = Some(Instant::now());
        let mut id = [1; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        assert!(p.confirm_resource(serial, id).is_none());
        assert!(matches!(
            p.resource_dialog.content,
            Some(Content::Message(_))
        ));
        assert!(!p.pending);
        Ok(())
    }
    #[test]
    fn forge_resource_unknown_reply_retains_original_attempt_without_new_charge()
    -> Result<(), &'static str> {
        let mut p = ItemForgePanel {
            selected: true,
            ..Default::default()
        };
        p.resource_dialog.offer(Operation::DustToSlivers, None);
        let mut id = [1; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        let attempt = p
            .confirm_resource(p.resource_dialog.serial, id)
            .ok_or("review")?;
        crate::item_forge::request(
            &mut p,
            crate::item_forge::command(attempt.encode().ok_or("wire")?.to_vec()),
        )
        .ok_or("submit")?;
        crate::item_forge::apply_result(
            &mut p,
            &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
                command_id: 1,
                server_sequence: 1,
                status: oteryn_client_runtime::server_mechanic::CommandStatus::Accepted,
                payload: vec![0],
            },
        );
        assert_eq!(p.retry, Some(attempt));
        assert!(!p.pending && !p.editing());
        assert!(matches!(
            p.resource_dialog.content,
            Some(Content::Message(_))
        ));
        assert!(p.confirm_resource(p.resource_dialog.serial, id).is_none());
        Ok(())
    }
}
