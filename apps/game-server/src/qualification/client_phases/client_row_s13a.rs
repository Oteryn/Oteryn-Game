//! S13a client replay capture: the party gate. The shared observation (`client_row_s13c`) casts the
//! caster's party spell while solo (refused, no vitals change), then in the party, then again during
//! the cooldown (refused). `Session::cast_spell` decodes the cast result into `SpellCastOutcome`
//! and does not retain the raw bytes, so `s13a-expected.txt` records the server-observed dispositions
//! and the replay encodes them with the production spell-cast encoder.
use super::*;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    let observed = super::client_row_s13c::observed(clients, super::client_row_s13c::CaptureChecks::Gate).await;
    let observed = observed.and_then(|value| {
        if value.solo.is_some() && value.refusal.is_some() {
            Ok(value)
        } else {
            Err("party gate observations missing".into())
        }
    });
    if let Ok(value) = &observed
        && let (Some((solo_status, solo)), Some((refusal_status, refusal))) = (value.solo, value.refusal)
    {
        super::client_row_s13c::capture(
            "s13a-expected.txt",
            format!("solo\tstatus={solo_status:?}\tdisposition={solo:?}\nrefusal\tstatus={refusal_status:?}\tdisposition={refusal:?}\n").as_bytes(),
        );
    }
    super::client_row_s13c::report("S13a", &observed, |value| {
        format!("solo={:?} refusal={:?}", value.solo, value.refusal)
    });
}
