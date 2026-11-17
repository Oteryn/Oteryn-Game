use super::DailyBoostPanel;
use oteryn_client_runtime::cui3::daily_boost::{COMMAND_TYPE_DAILY_BOOST_QUERY, DailyBoostView};
use oteryn_client_runtime::raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intent {
    pub command: u32,
    pub payload: Vec<u8>,
}
pub fn request(panel: &mut DailyBoostPanel, intent: Intent) -> Option<FeatureCommand> {
    if !panel.selected {
        panel.feedback = Some(tr!("Feature unavailable in this session"));
        return None;
    }
    if panel.pending {
        panel.feedback = Some(tr!("Waiting for the server"));
        return None;
    }
    if intent.command != COMMAND_TYPE_DAILY_BOOST_QUERY || !intent.payload.is_empty() {
        panel.feedback = Some(tr!("Unsupported feature action"));
        return None;
    }
    panel.pending = true;
    panel.feedback = None;
    panel.last_command = Some(intent.command);
    Some(FeatureCommand {
        capability: 151,
        kind: intent.command,
        payload: intent.payload,
    })
}
fn decode(mut b: &[u8]) -> Option<DailyBoostView> {
    let epoch = u64::from_be_bytes(b.get(..8)?.try_into().ok()?);
    b = &b[8..];
    fn text(b: &mut &[u8]) -> Option<String> {
        let n = usize::from(*b.first()?);
        let v = std::str::from_utf8(b.get(1..=n)?).ok()?.to_owned();
        *b = &b[n + 1..];
        (!v.is_empty()).then_some(v)
    }
    let creature_key = text(&mut b)?;
    let boss_key = text(&mut b)?;
    b.is_empty().then_some(DailyBoostView {
        reset_epoch: epoch,
        creature_key,
        boss_key,
    })
}
pub fn apply_result(p: &mut DailyBoostPanel, r: &GatedCommandResult) {
    p.pending = false;
    if matches!(
        r.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        p.result.clone_from(&r.payload);
        p.boost = decode(&r.payload);
        p.feedback = if p.boost.is_some() {
            None
        } else {
            Some(tr!("The server returned an invalid boost status"))
        }
    } else {
        p.feedback = Some(tr!("The server refused the action"))
    }
}
#[cfg(test)]
// Direct fixture construction keeps the wire assertions readable.
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    #[test]
    fn controls_encode_reply_and_refusal() {
        let mut p = DailyBoostPanel {
            selected: true,
            ..Default::default()
        };
        let c = request(
            &mut p,
            Intent {
                command: 177,
                payload: vec![],
            },
        )
        .unwrap();
        assert!(c.payload.is_empty());
        let mut wire = 9u64.to_be_bytes().to_vec();
        wire.extend([3, b'r', b'a', b't', 4, b'b', b'o', b's', b's']);
        apply_result(
            &mut p,
            &GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: CommandStatus::Accepted,
                payload: wire,
            },
        );
        assert_eq!(p.boost.as_ref().unwrap().boss_key, "boss");
        apply_result(
            &mut p,
            &GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: CommandStatus::Rejected,
                payload: vec![],
            },
        );
        assert!(p.feedback.unwrap().contains("refused"));
    }
}
