//! Capture packet for SCENARIO_MATRIX M03's automatic floor change and aggressive-action
//! exhaustion. The matching client replay exercises `Session::step`/`exchange_step` and
//! `Session::cast_spell`/`exchange_cast` in `crates/session/src/session_commands.rs`; their
//! domain application is `Session::apply_delta` in `crates/session/src/domain_dispatch.rs`.
use super::*;
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
use oteryn_protocol_oteryn::world_spatial::{encode_step_result, encode_world_spatial};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = journey(clients).await;
    let evidence_detail = match &result {
        Ok(()) => "floor-spatial-and-exhaust-refusal".to_owned(),
        Err(error) => error.to_string(),
    };
    evidence(&format!(
        "e2e_all scenario=M03.client status={} evidence={}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        evidence_detail,
    ));
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_character(clients, 0x93, 1).await?;
    let row_result: TestResult<_> = async {
        let from = tile(client.world_spatial().actor_position)?;
        let path = route(world, from, |at| at.floor != from.floor)
            .ok_or("M03 client capture has no reachable automatic floor transition")?;
        let (last, prefix) = path.split_last().ok_or("M03 floor route was empty")?;
        walk(&mut client, world, prefix).await?;
        let before = *client.world_spatial();
        let moved = client.step_retrying(*last).await?;
        let delta = moved
            .world_spatial_delta
            .as_ref()
            .ok_or("M03 floor step sent no spatial delta")?;
        if tile(delta.value.actor_position)?.floor == from.floor {
            return Err("M03 capture step did not change floor".into());
        }

        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let input = crate::content::native_gameplay::NativeGameplayInput::from_manifest(
            &root.join("content/spells.manifest.json"),
        )?;
        let catalog =
            crate::spell::executable_catalog::compile(&input.catalog.bytes, &input.catalog.sha256)?;
        let selections = crate::spell::executable_catalog::compile_source_selection(
            &input.source_selection.bytes,
            &input.source_selection.sha256,
            &catalog,
        )?;
        let book = catalog.into_spell_book_with_selections(&selections)?;
        let spell = book
            .spoken("exevo flam hur")
            .ok_or("M03 flame wave is absent")?
            .spell;
        let index = (1..=book.source_len())
            .filter_map(|value| {
                u32::try_from(value)
                    .ok()
                    .and_then(std::num::NonZeroU32::new)
            })
            .find(|value| {
                book.indexed(*value)
                    .is_some_and(|entry| entry.key == spell.key)
            })
            .ok_or("M03 flame wave index is absent")?;
        let vitals_before = client.actor_vitals().copied();
        let cast = client.cast_spell(index, SpellTarget::None, false).await?;
        if cast.disposition != SpellCastDisposition::CoolingDown
            || cast.actor_vitals_delta.is_some()
            || client.actor_vitals() != vitals_before.as_ref()
        {
            return Err(format!("M03 floor exhaust refusal was not stable: {cast:?}").into());
        }

        if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let directory = Path::new(&directory);
            let mut writes = vec![
                ("m03-spatial-before.bin", encode_world_spatial(&before)),
                ("m03-spatial-floor.bin", encode_world_spatial(&delta.value)),
                ("m03-step-result.bin", encode_step_result(moved.disposition)),
                (
                    "m03-cast-refusal.bin",
                    oteryn_protocol_oteryn::actor_spell::encode_spell_cast_result(cast.disposition),
                ),
            ];
            if let Some(vitals) = vitals_before {
                match oteryn_protocol_oteryn::actor_spell::encode_actor_vitals(&vitals) {
                    Ok(payload) => writes.push(("m03-vitals-before.bin", payload)),
                    Err(error) => return Err(format!("M03 vitals encode: {error:?}").into()),
                }
            }
            for (name, payload) in writes {
                std::fs::write(directory.join(name), payload)?;
            }
            let expected = format!(
                "{} {} {}\n{} {} {}\n{}\n",
                before.actor_position.x,
                before.actor_position.y,
                before.actor_position.floor,
                delta.value.actor_position.x,
                delta.value.actor_position.y,
                delta.value.actor_position.floor,
                vitals_before.map_or(0, |vitals| vitals.mana),
            );
            std::fs::write(directory.join("m03-expected.txt"), expected)?;
        }
        Ok(())
    }
    .await;
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}
