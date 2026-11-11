//! S13b client replay capture: the party spell's mana cost. The shared observation
//! (`client_row_s13c`) records the caster's mana before and after the in-party cast.
//! `Session::actor_vitals` holds the decoded vitals, so `s13b-expected.txt` records the observed
//! mana values and the replay encodes them with the production ACTOR_VITALS encoder.
use super::*;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    let observed = super::client_row_s13c::observed(clients).await;
    if let Ok(value) = observed {
        let written = (|| -> TestResult {
            super::client_row_s13c::capture(
                "s13b-expected.txt",
                format!("{}\n{}\n", value.vitals_before, value.vitals_after).as_bytes(),
            )?;
            Ok(())
        })();
        if let Err(error) = written {
            evidence(&format!(
                "e2e_all scenario=S13b.client status=FAIL evidence=capture-write:{error}"
            ));
            return;
        }
    }
    super::client_row_s13c::report("S13b", observed, |value| {
        format!(
            "mana={}->{} spent={}",
            value.mana_before,
            value.mana_after,
            value.mana_before - value.mana_after
        )
    });
}
