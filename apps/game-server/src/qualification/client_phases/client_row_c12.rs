//! Captures C12's real, unchanged spell-result and actor-vitals payload schemas. Equipment
//! augments are server-owned cast inputs and intentionally add no client protocol fields.

use super::*;
use oteryn_protocol_oteryn::actor_spell::{self, SpellCastDisposition, SpellTarget};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    if let Err(error) = verdict::observe(capture(clients)).await {
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C12.client status=FAIL evidence={error}"
        ));
    }
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    use oteryn_protocol_oteryn::actor_spell_catalog as catalog;
    let mut client = connect_world(clients, 0xc2).await?;
    let snapshot = client
        .session_mut()
        .gated_snapshot(catalog::STATE_DOMAIN_ACTOR_SPELL_CATALOG)
        .ok_or("C12 spell catalogue snapshot absent")?;
    let spell = catalog::decode_catalog(&snapshot.payload)
        .map_err(|error| format!("C12 spell catalogue malformed: {error:?}"))?
        .into_iter()
        .find(|entry| entry.available && entry.rune_item.is_none())
        .ok_or("C12 has no available instant spell")?;
    let cast_index = std::num::NonZeroU32::new(spell.cast_index)
        .ok_or("C12 available spell has zero cast index")?;
    let outcome = client
        .cast_spell(cast_index, SpellTarget::None, false)
        .await?;
    if outcome.status != CommandStatus::Accepted
        || outcome.disposition != SpellCastDisposition::Cast
    {
        return Err(format!("C12 cast was not accepted: {outcome:?}").into());
    }
    let vitals = outcome
        .actor_vitals_delta
        .as_ref()
        .map(|delta| delta.value)
        .ok_or("C12 accepted cast omitted actor-vitals")?;
    let result_payload = actor_spell::encode_spell_cast_result(outcome.disposition);
    let vitals_payload = actor_spell::encode_actor_vitals(&vitals)
        .map_err(|error| format!("C12 actor-vitals encoding failed: {error:?}"))?;
    write_capture("c12-cast-result.bin", &result_payload);
    write_capture("c12-actor-vitals.bin", &vitals_payload);
    write_capture(
        "c12-expected.txt",
        format!(
            "{} {} {} {} {} {} {}\n",
            vitals.health,
            vitals.max_health,
            vitals.mana,
            vitals.max_mana,
            vitals.soul,
            vitals.harmony,
            u8::from(vitals.serene)
        )
        .as_bytes(),
    );
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=C12.client status=PASS evidence=spell-index-{}-vitals-{vitals:?}",
        spell.cast_index
    ));
    Ok(())
}

fn write_capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(std::path::Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C12.client status=CAPTURE-WRITE-ERROR evidence={name}:{error}"
        ));
    }
}
