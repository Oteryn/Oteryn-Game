//! W15d client replay capture: the player walks to the Way of the Monk guidance shrine and uses
//! it. The raw quest-journal `List` deltas received before and after the use are written byte
//! for byte alongside the Guidance mission line. The quest remains in the list
//! while the shrine advances the mission and its durable track.
use super::*;
use crate::map::view::MapFacts;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::quest_log as journal;
use oteryn_gameplay_quests::quest::{loader, log, monk_shrines};
use oteryn_protocol_oteryn::world_object::UseDisposition;

const SHRINE: TilePos = TilePos {
    x: 32454,
    y: 32292,
    floor: -7,
};
const APPEARANCE: u16 = 50242;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "W15D_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

struct Listed {
    base: u64,
    new: u64,
    sequence: u64,
    payload: Vec<u8>,
    entries: Vec<journal::QuestListEntry>,
}

fn list_text(entries: &[journal::QuestListEntry]) -> String {
    entries
        .iter()
        .map(|entry| format!("{}:{}", entry.quest_index, u8::from(entry.completed)))
        .collect::<Vec<_>>()
        .join(";")
}

async fn journal_delta(
    client: &mut DevClientSession,
    query: journal::QuestLogQuery,
) -> TestResult<oteryn_dev_client::AppliedDelta<Vec<u8>>> {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(1050);
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    client.take_events();
    let payload = journal::encode_quest_log_query(&query)
        .map_err(|error| format!("list encode: {error:?}"))?;
    let reply = journal::send(client.session_mut(), &payload).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("quest list reply={reply:?}").into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(journal::STATE_DOMAIN_QUEST_LOG, 1)
        .await?;
    Ok(oteryn_dev_client::AppliedDelta {
        value: delta.payload,
        server_sequence: delta.server_sequence,
        base_revision: delta.base_revision,
        new_revision: delta.new_revision,
    })
}

async fn list(client: &mut DevClientSession) -> TestResult<Listed> {
    let delta = journal_delta(client, journal::QuestLogQuery::List).await?;
    let entries = match journal::decode_quest_log(&delta.value)
        .map_err(|error| format!("quest list decode={error:?}"))?
        .view
    {
        Some(journal::QuestLogView::List(entries)) => entries,
        other => return Err(format!("quest list view={other:?}").into()),
    };
    Ok(Listed {
        base: delta.base_revision,
        new: delta.new_revision,
        sequence: delta.server_sequence,
        payload: delta.value,
        entries,
    })
}

fn shrine_placement(world: &BundleWorld) -> TestResult<(Vec<u8>, u64)> {
    let tile = world
        .base()
        .tile(SHRINE.x, SHRINE.y, SHRINE.floor)
        .ok_or("authored guidance shrine tile absent")?;
    let ordinal = tile
        .ids()
        .iter()
        .zip(tile.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .find_map(|(ordinal, (id, _))| {
            let ordinal = u8::try_from(ordinal).ok()?;
            world
                .facts()
                .base_entry(SHRINE, ordinal, *id)
                .filter(|facts| facts.appearance_id == APPEARANCE)
                .map(|_| ordinal)
        })
        .ok_or("authored guidance shrine appearance absent")?;
    let key = crate::map::view::placement_key(SHRINE, ordinal).ok_or("shrine placement key")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok((placement, world.facts().object_revision(key)))
}

fn shrine_result_matches(
    eligible: bool,
    committed: bool,
    journal_changed: bool,
    before: i64,
    after: i64,
) -> bool {
    committed == eligible
        && committed == journal_changed
        && after == if committed { 1 } else { before }
}

#[test]
fn eligible_guidance_shrine_cannot_always_refuse() {
    assert!(!shrine_result_matches(true, false, false, 0, 0));
    assert!(!shrine_result_matches(true, true, true, 0, 0));
    assert!(shrine_result_matches(true, true, true, 0, 1));
    assert!(shrine_result_matches(false, false, false, 1, 1));
}

fn guidance_probe_ready(level: i64, questline: i64, guidance: i64) -> bool {
    level >= 6 && guidance == 0 && questline == 3
}

#[test]
fn guidance_probe_requires_pilgrimage_started() {
    assert!(!guidance_probe_ready(8, 0, 0));
    assert!(!guidance_probe_ready(5, 3, 0));
    assert!(!guidance_probe_ready(8, 3, 1));
    assert!(guidance_probe_ready(8, 3, 0));
}

async fn shrine_progress(
    connection: &mut sqlx::PgConnection,
    clients: &SeamClients<'_>,
) -> TestResult<(i64, i64, i64)> {
    let progress = sqlx::query_as("SELECT p.level::bigint, COALESCE(l.value, 0)::bigint, COALESCE(g.value, 0)::bigint FROM game_character_progression_state p LEFT JOIN game_character_quest_tracks l ON l.character_id = p.character_id AND l.track_key = 'oteryn:quest-progress/quest/u15_00/the_way_of_the_monk/questline' LEFT JOIN game_character_quest_tracks g ON g.character_id = p.character_id AND g.track_key = 'oteryn:quest-progress/quest/u15_00/the_way_of_the_monk/main_shrines/guidance' WHERE p.character_id = encode($1, 'hex')::uuid")
        .bind(clients.characters[0].as_slice()).fetch_one(connection).await?;
    Ok(progress)
}

async fn seed_pilgrimage(
    connection: &mut sqlx::PgConnection,
    character: &[u8],
    content_revision: &str,
    definition_hash: &[u8; 32],
) -> TestResult {
    use sqlx::Connection as _;
    let mut tx = connection.begin().await?;
    sqlx::query(
        "SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE",
    )
    .bind(character)
    .fetch_one(&mut *tx)
    .await?;
    let tracks = [monk_shrines::START_TRACK, monk_shrines::PILGRIMAGE_TRACK];
    let after = [1_i64, 3];
    let revision: String = sqlx::query_scalar("INSERT INTO game_character_quest_receipts(character_id,cause_kind,cause_id,cause_ordinal,transition_key,quest_key,request_binding,pinned_content_revision,definition_hash,completes,track_keys,values_before,values_after,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,committed_at) SELECT character_id,'command',game_character_uuid_v7(),1,$2,$3,$4,$5,$6,false,$7::text[],ARRAY[0,0]::bigint[],$8::bigint[],character_revision,character_revision+1,level,level,total_experience,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,0 FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid RETURNING committed_character_revision::text")
        .bind(character).bind(monk_shrines::START).bind(monk_shrines::QUEST)
        .bind(b"qualification-w15d".as_slice()).bind(content_revision).bind(definition_hash.as_slice())
        .bind(tracks.as_slice()).bind(after.as_slice()).fetch_one(&mut *tx).await?;
    for (track, value) in tracks.into_iter().zip(after) {
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5::text::numeric)")
            .bind(character).bind(track).bind(monk_shrines::QUEST).bind(value).bind(&revision).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5::text::numeric,$5::text::numeric)")
        .bind(character).bind(monk_shrines::QUEST).bind(content_revision).bind(definition_hash.as_slice()).bind(&revision).execute(&mut *tx).await?;
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
fn isolated_guidance_fixture_commits_an_unclaimed_pilgrimage() -> TestResult {
    use crate::bestiary_postgres_harness::{Harness, configured_admin, id, runtime};
    use sqlx::Connection as _;
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async {
        let h = Harness::create(admin, "qguidanceseed", true).await?;
        let probe = async {
            let character = id(41);
            let mut connection = sqlx::PgConnection::connect(&h.database.url).await?;
            let content = "content-1";
            let lowered = loader::load_embedded_quest_state(content)
                .map_err(|error| format!("quest catalogue: {error:?}"))?;
            let hash = lowered.catalogue().definition_hash(monk_shrines::QUEST)
                .ok_or("pilgrimage definition missing")?;
            seed_pilgrimage(&mut connection, &character, content, &hash).await?;
            let tracks: Vec<(String, i64)> = sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid ORDER BY track_key")
                .bind(character.as_slice()).fetch_all(&mut connection).await?;
            assert_eq!(tracks, [(monk_shrines::START_TRACK.into(), 1), (monk_shrines::PILGRIMAGE_TRACK.into(), 3)]);
            Ok::<_, Box<dyn std::error::Error>>(())
        }.await;
        h.cleanup().await?;
        probe
    })
}

fn needs_guidance_level(level: i64) -> bool {
    level < 6
}
#[test]
fn fresh_guidance_fixture_needs_progression() {
    assert!(needs_guidance_level(1));
    assert!(!needs_guidance_level(6));
}

async fn journey(clients: &SeamClients<'_>, world: &BundleWorld) -> TestResult<String> {
    use sqlx::Connection as _;
    let mut client = connect_character(clients, 0x88, 0).await?;
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let character = clients.characters[0].as_slice();
    let content_revision: String = sqlx::query_scalar("SELECT content_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(character).fetch_one(&mut connection).await?;
    let lowered = loader::load_embedded_quest_state(&content_revision)
        .map_err(|error| format!("quest catalogue: {error:?}"))?;
    let loaded = log::loader::load_embedded_quest_log(&lowered)?;
    let quest_index = u32::try_from(
        loaded
            .content
            .active()
            .quests()
            .iter()
            .position(|quest| quest.key == monk_shrines::QUEST)
            .ok_or("pilgrimage journal missing")?,
    )?;
    let (level, questline, guidance) = shrine_progress(&mut connection, clients).await?;
    if !guidance_probe_ready(level, questline, guidance) {
        evidence(&format!("W15d game gap level={level} questline={questline} guidance={guidance}"));
        return Err(Box::new(verdict::MissingServerRoute(concat!("source:", file!(), ":", line!()))));
    }
    let snapshot = client
        .session_mut()
        .gated_snapshot(journal::STATE_DOMAIN_QUEST_LOG)
        .ok_or("step=snapshot quest log snapshot absent")?
        .clone();
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at.floor == SHRINE.floor
            && at.x.abs_diff(SHRINE.x).max(at.y.abs_diff(SHRINE.y)) <= 1
            && at != SHRINE
    })
    .ok_or("step=walk no player route to the guidance shrine")?;
    walk(&mut client, world, &path).await?;
    let (placement, revision) = shrine_placement(world)?;
    let before = list(&mut client).await?;
    let used = client
        .session_mut()
        .use_object(&placement, revision)
        .await?;
    let outcome = match (used.status, used.disposition) {
        (CommandStatus::Accepted, UseDisposition::Committed) => "Committed",
        (CommandStatus::Rejected, UseDisposition::Rejected) => "Rejected",
        other => return Err(format!("step=use observed={other:?}").into()),
    };
    let after = list(&mut client).await?;
    if before.entries != after.entries
        || !after
            .entries
            .iter()
            .any(|entry| entry.quest_index == quest_index && !entry.completed)
    {
        return Err("guidance shrine changed which quests are listed or prematurely completed the pilgrimage".into());
    }
    let line = journal_delta(&mut client, journal::QuestLogQuery::Quest { quest_index }).await?;
    let projection = journal::decode_quest_log(&line.value)
        .map_err(|error| format!("pilgrimage journal line: {error:?}"))?;
    let changed = matches!(projection.view, Some(journal::QuestLogView::Line(line))
        if line.quest_index == quest_index
            && line.missions.iter().any(|mission| mission.name == "Shrine of Guidance" && mission.done));
    let (_, _, guidance_after) = shrine_progress(&mut connection, clients).await?;
    if !shrine_result_matches(
        guidance_probe_ready(level, questline, guidance),
        outcome == "Committed",
        changed,
        guidance,
        guidance_after,
    ) {
        return Err(format!("step=list use={outcome} list_changed={changed} level={level} questline={questline} guidance={guidance}->{guidance_after}").into());
    }
    let expected = format!(
        "snapshot_revision={}\nuse={outcome}\nlist_before={}\nbefore_base={}\nbefore_new={}\nbefore_sequence={}\n\
         list_after={}\nquest_index={quest_index}\nline_base={}\nline_new={}\nline_sequence={}\nafter_base={}\nafter_new={}\nafter_sequence={}\n",
        snapshot.revision,
        list_text(&before.entries),
        before.base,
        before.new,
        before.sequence,
        list_text(&after.entries),
        line.base_revision,
        line.new_revision,
        line.server_sequence,
        after.base,
        after.new,
        after.sequence,
    );
    capture("w15d-snapshot.bin", &snapshot.payload);
    capture("w15d-before.bin", &before.payload);
    capture("w15d-after.bin", &after.payload);
    capture("w15d-line.bin", &line.value);
    capture("w15d-expected.txt", expected.as_bytes());
    let _ = client.session_mut().logout().await;
    Ok(format!(
        "use={outcome} before={} after={}",
        list_text(&before.entries),
        list_text(&after.entries)
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    match verdict::indexed_timeout(
        Duration::from_secs(180),
        verdict::observe(journey(clients, world)),
    )
    .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W15d.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W15d.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=W15d.client status=FAIL evidence=timeout"),
    }
}
