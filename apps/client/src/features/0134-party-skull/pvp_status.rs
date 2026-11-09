use oteryn_client_runtime::{party_skull::PvpStatus, raw_feature::GatedCommandResult};
use oteryn_client_ui::party_skull::{FragWindow, Panel, SkullTimer, UnjustifiedSnapshot};
pub(in crate::play) fn apply(panel: &mut Panel, result: &GatedCommandResult) {
    panel.unjustified = None;
    panel.skull_timer = None;
    panel.black_limits_milli = None;
    panel.status_received_at = None;
    if !crate::play::feature_0115_mail::protocol::accepted(result) {
        panel.feedback = "The server refused the PvP observation".into();
        return;
    }
    let Ok(status) = PvpStatus::decode(&result.payload) else {
        panel.feedback = "Invalid PvP status reply".into();
        return;
    };
    panel.unjustified = Some(UnjustifiedSnapshot {
        windows: std::array::from_fn(|i| FragWindow {
            points_milli: status.points_milli[i],
            limit_milli: status.red_limits_milli[i],
        }),
        open_situations: Some(
            status
                .situations
                .iter()
                .map(|s| {
                    format!(
                        "{} attack: {}s at observation",
                        if s.incoming { "Incoming" } else { "Outgoing" },
                        s.remaining_ms.div_ceil(1000)
                    )
                })
                .collect(),
        ),
    });
    panel.skull_timer = Some(SkullTimer {
        skull: status.skull,
        remaining_ms: Some(status.remaining_ms),
    });
    panel.status_received_at = Some(std::time::Instant::now());
    panel.black_limits_milli = Some(status.black_limits_milli);
    panel.feedback = "PvP status updated".into();
}
