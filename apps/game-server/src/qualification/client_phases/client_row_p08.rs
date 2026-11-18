//! P08 client replay capture: real weapon-proficiency catalogue (660), browse (641), perk
//! selection (378) and a stale-revision RankUp refusal. The consumer is `apply_result` in
//! `crates/client-ui/src/features/0195-weapon-proficiency/protocol.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::weapon_proficiency as wire;
use std::num::NonZeroU32;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "P08_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

async fn command(
    client: &mut DevClientSession,
    kind: u32,
    payload: &[u8],
    name: &str,
) -> TestResult<Vec<u8>> {
    let reply = client
        .session_mut()
        .gated_command(wire::CAPABILITY_WEAPON_PROFICIENCY_V1, kind, payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("{name} status={:?}", reply.status).into());
    }
    capture(&format!("p08-{name}.bin"), &reply.payload);
    Ok(reply.payload)
}

fn perk_selected(accepted: bool, selections: &[Option<u8>], prior: Option<u8>) -> bool {
    accepted && prior != Some(0) && selections.first() == Some(&Some(0))
}

#[test]
fn proficiency_selection_requires_the_requested_perk() {
    assert!(!perk_selected(false, &[Some(0)], None));
    assert!(!perk_selected(true, &[Some(1)], None));
    assert!(perk_selected(true, &[Some(0)], None));
    assert!(!perk_selected(true, &[Some(0)], Some(0)));
}

async fn seed_track(url: &str, character: &[u8], weapon: &str, threshold: u32) -> TestResult {
    use crate::durability::character_proficiency::ProficiencyDefinitions;
    use crate::gameplay_transport::weapon_proficiency::ProductionProficiencyCatalogue;
    use sqlx::{Connection as _, PgConnection};
    let catalogue = ProductionProficiencyCatalogue::embedded().ok_or("proficiency catalogue")?;
    let definition = catalogue
        .resolve_weapon(weapon)
        .ok_or("proficiency fixture weapon")?;
    let selections: Vec<Option<i16>> =
        vec![None; usize::from(definition.shape.track_shape().perk_levels())];
    let mut connection = PgConnection::connect(url).await?;
    let mut tx = connection.begin().await?;
    sqlx::query(
        "SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE",
    )
    .bind(character)
    .fetch_one(&mut *tx)
    .await?;
    let occurrence: Vec<u8> = sqlx::query_scalar("INSERT INTO game_character_proficiency_receipts(proficiency_occurrence_id,character_id,original_character_revision,committed_character_revision,cause,command_binding,policy_digest,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,committed_at) SELECT game_character_uuid_v7(),character_id,character_revision,character_revision+1,'training',$2,$3,level,level,total_experience,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,1 FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid RETURNING uuid_send(proficiency_occurrence_id)")
        .bind(character).bind(b"qualification-p08".as_slice()).bind(catalogue.digest().as_slice()).fetch_one(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_proficiency_receipt_lines(proficiency_occurrence_id,character_id,committed_character_revision,cause,item_key,definition_key_before,definition_revision_before,definition_key_after,definition_revision_after,progress_before,progress_after,selections_before,selections_after) SELECT proficiency_occurrence_id,character_id,committed_character_revision,cause,$2,$3,$4,$3,$4,0,$5,$6,$6 FROM game_character_proficiency_receipts WHERE proficiency_occurrence_id=encode($1,'hex')::uuid")
        .bind(&occurrence).bind(&definition.canonical_item_key).bind(&definition.definition_key).bind(&definition.definition_revision)
        .bind(i64::from(threshold.max(1))).bind(&selections).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_proficiency(character_id,item_key,definition_key,definition_revision,progress,selections,committed_character_revision,last_proficiency_occurrence_id) SELECT character_id,item_key,definition_key_after,definition_revision_after,progress_after,selections_after,committed_character_revision,proficiency_occurrence_id FROM game_character_proficiency_receipt_lines WHERE proficiency_occurrence_id=encode($1,'hex')::uuid")
        .bind(&occurrence).execute(&mut *tx).await?;
    for statement in [
        "UPDATE game_character_roots SET character_revision=character_revision+1 WHERE character_id=encode($1,'hex')::uuid",
        "UPDATE game_character_progression_state SET character_revision=character_revision+1 WHERE character_id=encode($1,'hex')::uuid",
    ] {
        sqlx::query(statement)
            .bind(character)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

#[test]
fn isolated_proficiency_fixture_commits_an_unselected_track() -> TestResult {
    use crate::bestiary_postgres_harness::{Harness, configured_admin, id, runtime};
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async {
        let h = Harness::create(admin, "qperkseed", true).await?;
        let probe = async {
            let definitions = oteryn_gameplay_content::content::import_committed_weapon_proficiency_data()
                .map_err(|error| format!("proficiency definitions: {error:?}"))?;
            let weapon = definitions.bindings().keys().next().ok_or("fixture weapon missing")?;
            let character = id(41);
            seed_track(&h.database.url, &character, weapon, 100).await?;
            let (progress, selections): (i64, Vec<Option<i16>>) = sqlx::query_as("SELECT progress,selections FROM game_character_proficiency WHERE character_id=encode($1,'hex')::uuid")
                .bind(character.as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(progress, 100);
            assert!(!selections.is_empty() && selections.iter().all(Option::is_none));
            Ok::<_, Box<dyn std::error::Error>>(())
        }.await;
        h.cleanup().await?;
        probe
    })
}

async fn select_perk(
    client: &mut DevClientSession,
    definition: NonZeroU32,
    perk: Option<u8>,
    name: &str,
) -> TestResult<wire::SelectPerkReply> {
    let bytes = command(
        client,
        wire::COMMAND_TYPE_PROFICIENCY_SELECT_PERK,
        &wire::SelectPerkRequest {
            definition,
            level: 0,
            perk,
        }
        .encode(),
        name,
    )
    .await?;
    wire::SelectPerkReply::decode(&bytes).ok_or_else(|| "select reply".into())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0x2a).await?;
    let catalogue = command(
        &mut client,
        wire::COMMAND_TYPE_PROFICIENCY_CATALOGUE,
        &0_u32.to_be_bytes(),
        "catalogue",
    )
    .await?;
    let catalogue = wire::ProficiencyCataloguePage::decode(&catalogue).ok_or("catalogue page")?;
    let browse = command(
        &mut client,
        wire::COMMAND_TYPE_PROFICIENCY_BROWSE,
        &0_u32.to_be_bytes(),
        "browse",
    )
    .await?;
    let browse = wire::ProficiencyPage::decode(&browse).ok_or("browse page")?;
    let (entry, definition) = catalogue
        .entries
        .iter()
        .filter(|entry| {
            entry
                .levels
                .first()
                .is_some_and(|level| !level.perks.is_empty())
        })
        .filter_map(|entry| Some((entry, NonZeroU32::new(entry.definition)?)))
        .find(|(entry, _)| entry.experience == 0 || entry.experience >= entry.thresholds[0])
        .ok_or("no weapon suitable for an unlocked level-zero probe")?;
    let catalogue_first = catalogue
        .entries
        .first()
        .ok_or("catalogue first row absent")?;
    let mut prior = entry.levels.first().and_then(|level| level.selected);
    if entry.experience == 0 {
        return Err(Box::new(verdict::MissingServerRoute(concat!("source:", file!(), ":", line!()))));
    }
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| tile.protection_zone())
    })
    .ok_or("step=perk no reachable protection zone")?;
    walk(&mut client, world, &path).await?;
    if prior == Some(0) {
        let cleared = select_perk(&mut client, definition, None, "setup-clear").await?;
        if cleared.result != wire::SelectPerkResult::Accepted
            || cleared.selections.first() != Some(&None)
        {
            return Err(
                format!("level-zero perk was not cleared for the transition: {cleared:?}").into(),
            );
        }
        prior = None;
    }
    let select = select_perk(&mut client, definition, Some(0), "select").await?;
    if !perk_selected(
        select.result == wire::SelectPerkResult::Accepted,
        &select.selections,
        prior,
    ) {
        return Err(format!("requested level-zero perk was not selected: {select:?}").into());
    }
    let confirmed = command(
        &mut client,
        wire::COMMAND_TYPE_PROFICIENCY_BROWSE,
        &0_u32.to_be_bytes(),
        "confirmed",
    )
    .await?;
    let confirmed =
        wire::ProficiencyPage::decode(&confirmed).ok_or("confirmed proficiency page")?;
    if !confirmed
        .rows
        .iter()
        .any(|row| row.definition == definition && row.selections == select.selections)
    {
        return Err("level-zero perk absent from authoritative proficiency readback".into());
    }
    let operation = wire::ModificationOperation::RankUp;
    let stale = command(
        &mut client,
        operation.command_type(),
        &wire::ModificationRequest {
            definition,
            slot: 1,
            operation,
            expected_revision: u64::MAX,
        }
        .encode(),
        "stale",
    )
    .await?;
    let stale = wire::ModificationReply::decode(&stale).ok_or("stale reply")?;
    if stale.result == wire::ModificationResult::Accepted || stale.slot.is_some() {
        return Err(format!("stale RankUp was not refused: {stale:?}").into());
    }
    let expected = format!(
        "catalogue_total={}\ncatalogue_first={}|{}|{}\nbrowse_total={}\nbrowse_rows={}\nselect_definition={}\nselect_result={:?}\nselect_levels={:?}\nstale_result={:?}\n",
        catalogue.total,
        catalogue_first.definition,
        catalogue_first.name,
        catalogue_first.experience,
        browse.total,
        browse.rows.len(),
        definition,
        select.result,
        select.selections,
        stale.result
    );
    capture("p08-expected.txt", expected.as_bytes());
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P08.client status=CAPTURED evidence={}",
            expected.replace(' ', "_")
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P08.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P08.client status=FAIL evidence=timeout"),
    }
}
