use super::*;
use daily_reward::returner::{CLAIM, QUERY, Reply};
use oteryn_client_runtime::raw_feature::{FeatureCommand, GatedCommandResult};
pub fn request_returner(panel: &mut DailyRewardPanel, claim: bool) -> Option<FeatureCommand> {
    if !panel.selected || panel.returner_pending.is_some() {
        return None;
    }
    let (kind, payload) = if claim {
        {
            let Some(id) = panel.returner.claim(0) else {
                panel.returner.feedback = Some(tr!("No unclaimed returner reward is available."));
                return None;
            };
            (CLAIM, id.to_vec())
        }
    } else {
        panel.returner.pending = true;
        (QUERY, vec![])
    };
    panel.returner_pending = Some(kind);
    panel.returner.feedback = None;
    Some(FeatureCommand {
        capability: 182,
        kind,
        payload,
    })
}
/// Returns a notification identity only for a newly confirmed available offer.
pub fn apply_returner(
    panel: &mut DailyRewardPanel,
    kind: u32,
    result: &GatedCommandResult,
) -> Option<[u8; 16]> {
    if panel.returner_pending != Some(kind) {
        return None;
    }
    panel.returner_pending = None;
    let accepted = matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    );
    if kind == QUERY {
        let reply = accepted
            .then(|| Reply::decode(&result.payload).ok())
            .flatten();
        if let Some(reply) = reply {
            panel.returner.install(reply.id, reply.state == 2);
            if reply.state == 1
                && !panel.returner.is_claimed()
                && panel.returner_notified != reply.id
            {
                panel.returner_notified = reply.id;
                return reply.id;
            }
        } else {
            panel.returner.install(None, false);
            panel.returner.feedback = Some(tr!(
                "The returner reward is unavailable. Refresh to try again."
            ));
        }
    } else {
        let id = panel.returner.offer.as_ref().map(|o| o.id);
        let success = accepted
            && matches!(decode_reply(DailyRewardCommand::Claim,&result.payload),Ok(DailyRewardReply::Result(reply)) if matches!(reply.disposition,1|2));
        if let Some(id) = id {
            panel.returner.finish(id, success);
        } else {
            panel.returner.pending = false;
        }
        if !success {
            panel.returner.feedback = Some(tr!("The server refused the returner reward claim."));
        }
    }
    None
}
