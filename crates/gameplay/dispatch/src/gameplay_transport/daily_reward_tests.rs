//! Reservation checks through the existing negotiation and real connection dispatch.
//! These cases do not qualify a daily claim, value delivery or reset history.

use super::*;
use oteryn_protocol_oteryn::daily_reward::{
    CAPABILITY_DAILY_REWARD_V1, COMMAND_TYPE_DAILY_REWARD_CLAIM, COMMAND_TYPE_DAILY_REWARD_QUERY,
    STATE_DOMAIN_DAILY_REWARD,
};

#[test]
fn daily_reward_reservation_is_not_offered_or_selected() {
    for offered in [
        PRODUCTION_OFFERED_CAPABILITIES,
        ITEM_VIEW_OFFERED_CAPABILITIES,
        BUNDLE_WORLD_OFFERED_CAPABILITIES,
    ] {
        assert!(
            offered
                .iter()
                .all(|entry| entry.id != CAPABILITY_DAILY_REWARD_V1)
        );
        let selected = SelectedCapabilities::select(offered, &[CAPABILITY_DAILY_REWARD_V1])
            .expect("bounded selection");
        assert!(!selected.contains(CAPABILITY_DAILY_REWARD_V1));
        assert!(!selected.command_selected(COMMAND_TYPE_DAILY_REWARD_CLAIM));
        assert!(!selected.command_selected(COMMAND_TYPE_DAILY_REWARD_QUERY));
        assert!(!selected.domain_selected(STATE_DOMAIN_DAILY_REWARD));
    }
}

#[test]
fn daily_reward_reserved_commands_are_refused_through_real_dispatch() -> Result<(), Box<dyn Error>>
{
    run(async {
        let authority = NegotiatingAuthority::new(None);
        let (admitted, accepted) =
            admit(&authority, &bootstrap(&[CAPABILITY_DAILY_REWARD_V1])?).await?;
        assert!(accepted_selection(&accepted)?.is_empty());
        let admitted = admitted.map_err(|end| format!("admission failed: {end:?}"))?;
        let commands = [
            command(1, COMMAND_TYPE_DAILY_REWARD_CLAIM, &[]),
            command(2, COMMAND_TYPE_DAILY_REWARD_CLAIM, b"malformed"),
            command(3, COMMAND_TYPE_DAILY_REWARD_QUERY, &[]),
        ];
        let (_, written) = serve(&authority, admitted, &commands).await?;
        let mut results = Vec::new();
        for frame in &written {
            let envelope = decode_wire_envelope(frame)?;
            if envelope.message_type() == MessageType::CommandResult {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                assert_eq!(result.status, crate::foundation::CommandStatus::Rejected);
                assert!(result.payload.is_empty());
                results.push(result.command_id);
            }
            if envelope.message_type() == MessageType::SnapshotChunk {
                let (_, domains) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())?;
                assert!(
                    domains
                        .iter()
                        .all(|domain| domain.domain_id != STATE_DOMAIN_DAILY_REWARD)
                );
            }
        }
        assert_eq!(results, [1, 2, 3]);
        assert_eq!(authority.steps.get(), 0);
        assert!(authority.item_uses.borrow().is_empty());
        Ok(())
    })
}
