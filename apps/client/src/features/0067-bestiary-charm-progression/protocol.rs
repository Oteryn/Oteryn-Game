use crate::play::feature_0115_mail::protocol::accepted;
use oteryn_client_runtime::{
    charm_progression as wire,
    raw_feature::{FeatureCommand, GatedCommandResult},
};
use oteryn_client_ui::charm_progression::{Intent, Panel};
use std::num::NonZeroU32;
fn index(text: &str) -> Result<NonZeroU32, String> {
    text.trim()
        .parse::<u32>()
        .ok()
        .and_then(NonZeroU32::new)
        .ok_or_else(|| "Choose a positive index".into())
}
pub(in crate::play) fn command(panel: &Panel, intent: Intent) -> Result<FeatureCommand, String> {
    use wire::CharmProgressionAction as A;
    let action = match intent {
        Intent::Query => A::Query,
        Intent::ResetAll => A::ResetAll,
        Intent::Unlock => {
            let charm = index(&panel.fields[0])?;
            let expected_stage = panel
                .reply
                .as_ref()
                .and_then(|r| r.charms.charms.iter().find(|c| c.charm == charm))
                .ok_or("Query the charm before unlocking")?
                .unlocked_stage;
            A::Unlock {
                charm,
                expected_stage,
            }
        }
        Intent::Assign => A::Assign {
            charm: index(&panel.fields[0])?,
            race: index(&panel.fields[1])?,
        },
        Intent::Remove => A::Remove {
            charm: index(&panel.fields[0])?,
        },
    };
    Ok(FeatureCommand {
        capability: wire::CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1,
        kind: wire::COMMAND_TYPE_CHARM_PROGRESSION,
        payload: wire::encode_charm_progression_request(&action)
            .map_err(|_| "Invalid charm request")?,
    })
}
pub(in crate::play) fn apply(
    panel: &mut Panel,
    command: &FeatureCommand,
    result: &GatedCommandResult,
) {
    if super::input::catalogue::apply(panel, command, result) { return; }
    if !panel.pending || panel.pending_kind != command.kind {
        return;
    }
    panel.pending = false;
    if !accepted(result) {
        panel.feedback = "The server refused the charm request".into();
        return;
    }
    let Ok(reply) = wire::decode_charm_progression_reply(&result.payload) else {
        panel.feedback = "Invalid charm reply".into();
        return;
    };
    use wire::{CharmProgressionOutcome as O, CharmRemoveDisposition as R};
    let success = matches!(
        reply.outcome,
        O::Query
            | O::Unlock(wire::charm::CharmUnlockDisposition::Unlocked)
            | O::Assign(wire::charm::CharmAssignDisposition::Assigned)
            | O::Remove(R::Removed)
            | O::ResetAll(R::Removed)
    );
    panel.feedback = format!(
        "{}: {:?}; fee: {} gold",
        if success {
            "Completed"
        } else {
            "Server refused"
        },
        reply.outcome,
        reply.fee_gold
    );
    panel.summary = format!(
        "Charm points: {}; minor echoes: {}; assignment slots: {:?}\r\n",
        reply.charms.charm_points_available,
        reply.charms.minor_charm_echoes_available,
        reply.charms.assignment_slot_limit
    );
    for race in &reply.bestiary {
        panel.summary.push_str(&format!(
            "Race {}: {} kills, stage {}, thresholds {:?}, next {:?}\r\n",
            race.race,
            race.kill_count,
            race.unlocked_stage(),
            race.kill_thresholds,
            race.next_threshold()
        ));
    }
    for charm in &reply.charms.charms {
        panel.summary.push_str(&format!(
            "Charm {}: {:?}, stage {}, next cost {}, race {:?}, active {}\r\n",
            charm.charm,
            charm.kind,
            charm.unlocked_stage,
            charm.next_stage_cost,
            charm.assigned_race,
            charm.effect_active
        ));
    }
    panel.reply = Some(reply);
}
