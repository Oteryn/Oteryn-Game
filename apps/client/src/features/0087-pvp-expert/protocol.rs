use crate::play::feature_0115_mail::protocol::accepted;
use crate::play::{Outbox, Stepper, mailbox::lock};
use oteryn_client_runtime::{
    pvp_expert as wire,
    raw_feature::{FeatureCommand, GatedCommandResult},
};
use oteryn_client_ui::pvp_expert::{Intent, Panel};

pub(in crate::play) fn command(panel: &Panel, _: Intent) -> Result<FeatureCommand, String> {
    use wire::attack::{ChaseMode, FightMode, FightModes};
    let fight_mode = match panel.fields[0].trim() {
        "1" => FightMode::Offensive,
        "2" => FightMode::Balanced,
        "3" => FightMode::Defensive,
        _ => return Err(tr!("Choose a fight mode from 1 to 3").into()),
    };
    let chase = match panel.fields[1].trim() {
        "1" => ChaseMode::Stand,
        "2" => ChaseMode::Chase,
        _ => return Err(tr!("Choose stand (1) or chase (2)").into()),
    };
    let secure = match panel.fields[2].trim() {
        "0" => false,
        "1" => true,
        _ => return Err(tr!("Choose secure off (0) or on (1)").into()),
    };
    let expert = match panel.fields[3].trim() {
        "0" => wire::ExpertMode::Dove,
        "1" => wire::ExpertMode::WhiteHand,
        "2" => wire::ExpertMode::YellowHand,
        "3" => wire::ExpertMode::RedFist,
        _ => return Err(tr!("Choose an expert mode from 0 to 3").into()),
    };
    Ok(FeatureCommand {
        capability: wire::CAPABILITY_PVP_EXPERT_V1,
        kind: wire::COMMAND_TYPE_EXPERT_FIGHT_MODES_INTENT,
        payload: wire::encode_expert_fight_modes(&wire::ExpertFightModes {
            modes: FightModes {
                fight_mode,
                chase,
                secure,
            },
            expert,
        }),
    })
}
pub(in crate::play) fn apply(
    panel: &mut Panel,
    command: &FeatureCommand,
    result: &GatedCommandResult,
) {
    if !panel.pending || panel.pending_kind != command.kind {
        return;
    }
    panel.pending = false;
    panel.feedback = if !accepted(result) {
        tr!("The server refused the expert mode request")
    } else {
        match wire::attack::decode_attack_intent_result(&result.payload) {
            Ok(wire::attack::AttackIntentDisposition::Ok) => {
                tr!("Request accepted; confirmed modes follow the server state")
            }
            Ok(_) => tr!("The server refused the expert mode request"),
            Err(_) => tr!("Invalid expert mode reply"),
        }
    }
    .into();
}
fn state(panel: &mut Panel, modes: wire::ExpertFightModes) {
    if panel.confirmed == Some(modes) {
        return;
    }
    panel.confirmed = Some(modes);
    if !panel.pending {
        panel.fields = [
            (modes.modes.fight_mode as u32).to_string(),
            (modes.modes.chase as u32).to_string(),
            u8::from(modes.modes.secure).to_string(),
            (modes.expert as u32).to_string(),
        ];
    }
    panel.summary = lfmt!(
        "Confirmed: {:?}, {:?}, secure {}, {:?}",
        modes.modes.fight_mode,
        modes.modes.chase,
        modes.modes.secure,
        modes.expert
    );
}
pub(in crate::play) fn initialize<T: Stepper>(panel: &mut Panel, session: &T) {
    if !panel.selected {
        return;
    }
    if let Some(modes) = session
        .feature_snapshot(wire::STATE_DOMAIN_EXPERT_FIGHT_MODES)
        .and_then(|bytes| wire::decode_expert_fight_modes(&bytes).ok())
    {
        state(panel, modes);
    } else {
        panel.feedback = tr!("Invalid expert mode state").into();
    }
}
impl Outbox {
    pub(in crate::play) fn pvp_expert_events(&self, events: &[oteryn_session::SessionEvent]) {
        let mut shared = lock(&self.0);
        let panel = &mut shared.properties.pvp_expert;
        if !panel.selected {
            return;
        }
        for event in events {
            match event {
                oteryn_session::SessionEvent::ExpertFightModes(delta) => state(panel, delta.value),
                oteryn_session::SessionEvent::GatedSnapshotReplaced(delta)
                    if delta.value.domain_id == wire::STATE_DOMAIN_EXPERT_FIGHT_MODES =>
                {
                    if let Ok(modes) = wire::decode_expert_fight_modes(&delta.value.payload) {
                        state(panel, modes);
                    }
                }
                _ => {}
            }
        }
    }
}
