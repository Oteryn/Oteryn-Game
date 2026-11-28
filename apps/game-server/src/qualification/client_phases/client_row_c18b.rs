//! C18b client verification of a live Death Echo position cast.

use super::*;
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTargetPosition};

const SPELL: &str = "exevo mort ora";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result =
        verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(capture(clients))).await;
    match result {
        Ok(Ok(())) => evidence(
            "SEAM_EVIDENCE e2e_all scenario=C18b.client status=PASS evidence=original-position-cast-frames-replayed-and-vitals-retained",
        ),
        Ok(Err(error)) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C18b.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence(
            "SEAM_EVIDENCE e2e_all scenario=C18b.client status=FAIL evidence=journey-timeout",
        ),
    }
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let book = spell_book()?;
    let client = connect_character(clients, 0x18, 1).await?;
    let mut client =
        provisioning::provision_profile(client, clients, 1, 120, "sorcerer-120").await?;
    let spell = book
        .spoken(SPELL)
        .ok_or("C18b.client Death Echo absent")?
        .spell;
    let level = full_e2ef5::prepare_spell_access(&mut client, spell).await?;
    if level != 120 {
        return Err(format!("C18b.client expected level 120, got {level}").into());
    }
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        if !world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
        {
            return false;
        }
        let cursor = (0..3).try_fold(at, |from, _| {
            destination(world, from, StepDirection::East).filter(|to| to.floor == at.floor)
        });
        cursor.is_some_and(|to| {
            world
                .base()
                .tile(to.x, to.y, to.floor)
                .is_some_and(|tile| !tile.protection_zone())
        })
    })
    .ok_or("C18b.client needs a reachable non-PZ firing tile with an open east cursor line")?;
    walk(&mut client, world, &path).await?;
    let here = client.world_spatial().actor_position;
    let target = SpellTargetPosition {
        x: here
            .x
            .checked_add(3)
            .ok_or("C18b cursor coordinate overflow")?,
        y: here.y,
        floor: here.floor,
    };

    let crate::spell::ManaCost::Fixed(mana) = spell.mana else {
        return Err("C18b.client Death Echo must have a fixed mana cost".into());
    };
    if mana != 150
        || spell.soul != 0
        || !matches!(&spell.carrier, crate::spell::Carrier::Instant { .. })
    {
        return Err("C18b.client expected Global Death Echo (150 mana, no soul or rune)".into());
    }
    let index = spell_index(&book, SPELL)?;
    let before = *client.actor_vitals().ok_or("C18b.client vitals absent")?;
    let expected_mana = before
        .mana
        .checked_sub(mana)
        .ok_or("C18b.client insufficient fixture mana")?;
    let capture_path = std::env::temp_dir().join(format!(
        "q-c18b-client-{}.frames",
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
    ));
    let prepare_logout = client.session_mut().logout().await?;
    if prepare_logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(
            format!("C18b recording preparation logout refused: {prepare_logout:?}").into(),
        );
    }
    drop(client);
    let (outcome, session) = connect_character_using(clients, 0x19, 1, async |request| {
        oteryn_dev_client::replay_live_position_cast(request, index, target, before, &capture_path)
            .await
            .map_err(|error| error.to_string().into())
    })
    .await?;
    if outcome.status != CommandStatus::Accepted
        || outcome.disposition != SpellCastDisposition::Cast
    {
        return Err(format!("C18b.client first Death Echo refused: {outcome:?}").into());
    }
    let delta = outcome
        .actor_vitals_delta
        .ok_or("C18b.client cast omitted actor-vitals delta")?;
    let expected = oteryn_protocol_oteryn::actor_spell::ActorVitals {
        mana: expected_mana,
        ..before
    };
    if delta.server_sequence <= outcome.result_server_sequence || delta.value != expected {
        return Err(format!(
            "C18b.client cast did not debit mana in the applied vitals: {before:?} -> {:?}",
            delta.value
        )
        .into());
    }
    let fresh = connect_character(clients, 0x1a, 1).await?;
    if fresh.join_snapshot().game_session_id.as_bytes() == &session
        || fresh.world_spatial().actor_position != here
        || fresh.actor_vitals() != Some(&delta.value)
    {
        return Err("C18b client fresh admission did not retain the exact cast result".into());
    }
    evidence(&format!(
        "C18B_CHECK scenario=C18b.client scope=live-cast-and-replay session={} result_sequence={} mana_before={} mana_after={}",
        uuid_text(&session),
        outcome.result_server_sequence,
        before.mana,
        delta.value.mana,
    ));
    Ok(())
}

fn spell_book() -> TestResult<crate::spell::SpellBook> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
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
    Ok(catalog.into_spell_book_with_selections(&selections)?)
}

fn spell_index(book: &crate::spell::SpellBook, words: &str) -> TestResult<std::num::NonZeroU32> {
    let key = &book
        .spoken(words)
        .ok_or("C18b.client spell words absent")?
        .spell
        .key;
    (1..=book.source_len())
        .filter_map(|index| {
            u32::try_from(index)
                .ok()
                .and_then(std::num::NonZeroU32::new)
        })
        .find(|index| book.indexed(*index).is_some_and(|spell| &spell.key == key))
        .ok_or_else(|| "C18b.client spell index absent".into())
}
