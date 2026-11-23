use oteryn_client_runtime::{party_skull::PvpStatus, raw_feature::GatedCommandResult};
use oteryn_client_ui::party_skull::{FragWindow, Panel, SkullTimer, UnjustifiedSnapshot};
pub(in crate::play) fn apply(panel: &mut Panel, result: &GatedCommandResult) {
    panel.incoming_situations = None;
    panel.incoming_ids = None;
    panel.unjustified = None;
    panel.skull_timer = None;
    panel.black_limits_milli = None;
    panel.status_received_at = None;
    if !crate::play::feature_0115_mail::protocol::accepted(result) {
        panel.feedback = "The server refused the PvP observation".into();
        return;
    }
    let Some(status) = PvpStatus::decode(&result.payload).ok().filter(|status| {
        status
            .situations
            .iter()
            .all(|s| s.situation_id.is_some() == panel.situation_identity)
    }) else {
        panel.feedback = "Invalid PvP status reply".into();
        return;
    };
    if panel.situation_identity {
        panel.incoming_ids = status
            .situations
            .iter()
            .filter(|s| s.incoming)
            .map(|s| s.situation_id)
            .collect();
    }
    let mut incoming: Vec<_> = status
        .situations
        .iter()
        .filter(|s| s.incoming)
        .map(|s| s.remaining_ms)
        .collect();
    incoming.sort_unstable_by(|a, b| b.cmp(a));
    panel.incoming_situations = Some(incoming);
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

#[cfg(test)]
#[test]
fn pvp_status_rejects_identity_selection_mismatch() -> Result<(), String> {
    use oteryn_client_runtime::party_skull::PvpSituation;
    for selected in [false, true] {
        for incoming in [false, true] {
            let mut panel = Panel {
                situation_identity: selected,
                ..Default::default()
            };
            let status = PvpStatus {
                points_milli: [0; 3],
                red_limits_milli: [3000; 3],
                black_limits_milli: [6000; 3],
                skull: 0,
                remaining_ms: 0,
                situations: vec![PvpSituation {
                    incoming,
                    remaining_ms: 9000,
                    situation_id: (!selected).then_some(9),
                }],
            };
            apply(
                &mut panel,
                &GatedCommandResult {
                    command_id: 1,
                    server_sequence: 1,
                    status: oteryn_session::CommandStatus::Accepted,
                    payload: status.encode().map_err(|e| format!("{e:?}"))?,
                },
            );
            assert_eq!(panel.feedback, "Invalid PvP status reply");
            assert!(panel.status_received_at.is_none());
        }
    }
    Ok(())
}
