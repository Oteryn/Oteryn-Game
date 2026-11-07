//! Crystal201189f8 Hairycles212–216/451–456: a completed quest unlocks both Shaman bases.
use super::*;
const STAGE: &str = "oteryn:quest-progress/quest/u7_6/the_ape_city/questline";
const CLAIMED: &str = "oteryn:quest-progress/quest/u7_6/the_ape_city/shaman_outfit";
const GRANT: &str = "oteryn:appearance-grant/npc/hairycles/shaman/base";
pub(super) async fn historical_stage(h: &Harness, stage: i64) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,'oteryn:quest.the_ape_city_quest',$3,greatest(2,character_revision) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid ON CONFLICT(character_id,track_key) DO UPDATE SET value=excluded.value").bind(id(41).as_slice()).bind(STAGE).bind(stage).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
pub(super) async fn exercise(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
) -> TestResult {
    let started = std::time::Instant::now();
    super::super::grizzly_npc::wire::talk(
        owner,
        admitted,
        npc,
        &[(1, "hi"), (2, "outfit"), (3, "yes")],
    )
    .await?;
    assert_eq!(
        super::grants(h, GRANT).await?,
        0,
        "stage17 cannot acquire the source base"
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2").bind(id(41).as_slice()).bind(CLAIMED).fetch_one(&h.pool).await?,0,"first source claim has no stored Shaman flag");
    historical_stage(h, 18).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let effects = super::super::grizzly_npc::wire::talk_effects(
        owner,
        admitted,
        npc,
        &[(4, "hi"), (5, "shamanic"), (6, "yes")],
    )
    .await?;
    assert_eq!(
        super::grants(h, GRANT).await?,
        1,
        "actual reachable Hairycles must grant Shaman154/158 through command7"
    );
    assert_blue(&effects)?;
    let before:i64=sqlx::query_scalar("SELECT (track_changes->0->>'before')::bigint FROM game_npc_cosmetic_receipts WHERE source_key='oteryn:npc-cosmetic/hairycles/shaman' ORDER BY committed_character_revision DESC LIMIT 1").fetch_one(&h.pool).await?;
    assert_eq!(
        before, -1,
        "the mature writer records absence as -1, never NULL"
    );

    let flags:(i64,i64)=sqlx::query_as("SELECT (SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),(SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$3)").bind(id(41).as_slice()).bind(STAGE).bind(CLAIMED).fetch_one(&h.pool).await?;
    assert_eq!(
        flags,
        (18, 1),
        "only the once-claim flag changes; the quest owner keeps stage18"
    );
    let count_before: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_npc_cosmetic_receipts WHERE npc_key='oteryn:npc.hairycles'",
    )
    .fetch_one(&h.pool)
    .await?;
    super::super::grizzly_npc::wire::talk(
        owner,
        admitted,
        npc,
        &[(6, "yes"), (7, "hi"), (8, "outfit"), (9, "yes")],
    )
    .await?;
    assert_eq!(
        super::grants(h, GRANT).await?,
        1,
        "replay/reconnect/repeated claim cannot duplicate the base"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_npc_cosmetic_receipts WHERE npc_key='oteryn:npc.hairycles'"
        )
        .fetch_one(&h.pool)
        .await?,
        count_before
    );
    eprintln!(
        "SRV7_HAIRYCLES_SHAMAN_REAL_NPC_EXECUTED elapsed={:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

pub(super) fn assert_blue(batches: &[Vec<u8>]) -> TestResult {
    let events = batches
        .iter()
        .map(|bytes| {
            oteryn_protocol_oteryn::presentation::decode_item_effect_batch(bytes).map_err(debug)
        })
        .collect::<TestResult<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    assert_eq!(
        events.len(),
        1,
        "one fresh donor NPC grant must emit one native source effect"
    );
    assert!(
        matches!(
            events[0].kind,
            oteryn_protocol_oteryn::presentation::PresentationEventKind::MagicEffect {
                effect_id: 13,
                ..
            }
        ),
        "pinned utils_definitions.hpp68 normalizes MAGIC_BLUE13"
    );
    Ok(())
}
