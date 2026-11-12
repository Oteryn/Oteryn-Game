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
    let observed = super::client_row_s13c::observed(clients).await;
    if let Ok(value) = observed {
        super::client_row_s13c::capture(
            "s13a-expected.txt",
            format!(
                "solo\tstatus={:?}\tdisposition={:?}\nrefusal\tstatus={:?}\tdisposition={:?}\n",
                value.solo_status,
                value.solo_disposition,
                value.refusal_status,
                value.refusal_disposition
            )
            .as_bytes(),
        );
    }
    super::client_row_s13c::report("S13a", observed, |value| {
        format!("solo={:?} refusal={:?}", value.solo_disposition, value.refusal_disposition)
    });
}
