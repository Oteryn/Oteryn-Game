//! C17c client replay capture: casting Creature Illusion (`utevo res ina "rat"`) as a player
//! changes the caster's own appearance binding. Client owner: `AppearanceState` in
//! `crates/session` (`Session::visible_appearances`). The raw appearance join snapshot is the
//! server payload; the Mappings payload is read directly from the server.
use super::*;
use oteryn_dev_client::{ChatDisposition, ChatIntent, ChatSpeechMode, DevClientSession};
use oteryn_protocol_oteryn::appearance::{self, Look};

const WORDS: &str = "utevo res ina \"rat\"";

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

fn outfit(look: &Look) -> TestResult<String> {
    match look {
        Look::Outfit {
            look_type,
            colours,
            addons,
        } => Ok(format!(
            "{look_type} {} {} {} {} {addons}",
            colours[0], colours[1], colours[2], colours[3]
        )),
        _ => Err("C17c look is not an outfit".into()),
    }
}

fn hex(bytes: &[u8; 16]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    let mut client: DevClientSession = connect_world(clients, 0x31).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    let own = *client
        .world_entities()
        .ok_or("C17c entities absent")?
        .own_identity();
    let snapshot = client
        .session_mut()
        .gated_snapshot(appearance::STATE_DOMAIN_APPEARANCE)
        .ok_or("C17c appearance snapshot absent")?
        .clone();
    let decoded = appearance::Snapshot::decode(&snapshot.payload)
        .map_err(|error| format!("appearance snapshot: {error:?}"))?;
    let baseline = decoded
        .bindings
        .iter()
        .find(|binding| binding.entity.identity == own)
        .ok_or("C17c own appearance binding absent")?
        .clone();
    client.take_events();
    let cast = client
        .chat(&ChatIntent::Say {
            mode: ChatSpeechMode::Say,
            text: WORDS.into(),
        })
        .await?;
    if cast.status != CommandStatus::Accepted || cast.disposition != ChatDisposition::Ok {
        return Err(format!(
            "illusion cast refused status={:?} disposition={:?}; the ordinary starter lacks the level and vocation (full_e2ef5 C17c)",
            cast.status, cast.disposition
        )
        .into());
    }
    // Read the actual Mappings payload, including the server's root revision.
    let delta = client.session_mut().read_gated_delta(
        appearance::STATE_DOMAIN_APPEARANCE,
        appearance::DELTA_TYPE_APPEARANCE_MAPPINGS,
    ).await?;
    let mappings = appearance::Mappings::decode(&delta.payload)
        .map_err(|error| format!("mapping decode: {error:?}"))?;
    let binding = mappings.upserts.iter().find(|entry| entry.entity.identity == own)
        .ok_or("own mapping absent")?;
    if binding.look == baseline.look || binding.root_revision <= baseline.root_revision {
        return Err("illusion did not advance the own appearance mapping".into());
    }
    let mapping_payload = delta.payload;
    let sequence = cast.result_server_sequence;
    let expected = format!(
        "revision={}\nidentity={}\ngeneration={}\nref={}\nbaseline={}\nlook={}\n",
        snapshot.revision,
        hex(&own),
        baseline.entity.generation,
        baseline.appearance_ref,
        outfit(&baseline.look)?,
        outfit(&binding.look)?
    );
    capture("c17c-snapshot.bin", &snapshot.payload)?;
    capture("c17c-mapping.bin", &mapping_payload)?;
    capture("c17c-expected.txt", expected.as_bytes())?;

    Ok(format!("look={} sequence={sequence}", outfit(&binding.look)?))

    }).await.unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=C17c.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=C17c.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=C17c.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_preserves_full_identity_width() {
        assert_eq!(hex(&[0; 16]), "00".repeat(16));
        assert_eq!(hex(&[255; 16]), "ff".repeat(16));
    }
}

#[cfg(test)]
mod data_tests {
    use super::*;
    #[test]
    fn outfits_reject_non_outfits_and_keep_colour_width() -> TestResult {
        assert!(outfit(&Look::EmptyOutfit).is_err());
        assert!(outfit(&Look::Item { definition_ref: 1 }).is_err());
        assert_eq!(
            outfit(&Look::Outfit {
                look_type: u32::MAX,
                colours: [0, 255, 256, u16::MAX],
                addons: 3
            })?,
            "4294967295 0 255 256 65535 3"
        );
        Ok(())
    }
}
