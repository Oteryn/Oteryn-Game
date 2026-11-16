//! Real USE/STEP ingress for the shared Dreamer entrance; no seeded quest progress.
use super::TestResult;
use oteryn_gameplay_quests::quest::{dreamer_fire_bug, world_callbacks::Position};

#[test]
fn m07_dreamer_fire_bug_retains_input_and_resets_shared_entrance() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("m07-dreamer-fire-bug", || {
        super::quest_world_callbacks::scenario(
            "dreamers-challenge-fire-bug-ignite",
            dreamer_fire_bug::SOURCE,
            false,
        )
    })
}

/// The admitted ignition scenario also proves the bundle Channel frame arm
/// (`bundle_channel_frame_admission`) against the real retained proof.
#[test]
fn dreamer_ignition_accepts_bundle_channel_frame() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("m07-dreamer-bundle-frame", || {
        super::quest_world_callbacks::scenario(
            "dreamers-challenge-fire-bug-ignite",
            dreamer_fire_bug::SOURCE,
            false,
        )
    })
}

#[allow(
    clippy::too_many_arguments,
    reason = "the common admitted wire scenario supplies the fenced owners"
)]
pub(super) async fn exercise(
    owner: &mut super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    h: &super::fixture::Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    callback: &oteryn_gameplay_quests::quest::world_callbacks::Callback,
    trigger: Position,
) -> TestResult {
    use super::fixture::debug;
    use crate::map::view::MapFacts;
    use oteryn_gameplay_quests::quest::world_callbacks::Offering;
    use oteryn_protocol_oteryn::{
        world_object::{UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent},
        world_spatial::{StepDirection, StepDisposition, decode_step_result, encode_step_intent},
    };
    let key = oteryn_world_bundle::bundle::placement_key(trigger.floor, trigger.x, trigger.y, 1)
        .ok_or("ashes key")?;
    let pos = crate::map::overlay::TilePos {
        x: trigger.x,
        y: trigger.y,
        floor: trigger.floor,
    };
    let mut placement = owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .digest()
        .to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = |revision| {
        encode_use_intent(&WorldObjectTarget {
            placement: placement.clone(),
            expected_revision: revision,
        })
        .map_err(debug)
    };
    let before = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload(0)?).await?;
    assert_eq!(
        decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Rejected
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_shared_input_proofs")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(count, 0);
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: trigger,
            item: "oteryn:item.tibia.i5467".into(),
            count: 1,
        },
        60,
        true,
    )
    .await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("pacing")?;
    let before_use = owner.owner_now().get();
    let reply = super::quest_world_callbacks::wire(owner, admitted, 2, 2, payload(0)?).await?;
    let after_use = owner.owner_now().get();
    assert_eq!(
        decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Committed
    );
    let map = owner.bundle_map().ok_or("map")?;
    assert_eq!(
        map.facts
            .base_entry(pos, 1, 1)
            .ok_or("lit glyph")?
            .appearance_id,
        1949
    );
    let cues = map.facts.movement_effects_since(0);
    assert_eq!(
        cues.iter()
            .filter(|(_, _, effect)| *effect == dreamer_fire_bug::IGNITION_EFFECT)
            .count(),
        8
    );
    let cue_cursor = cues.last().map_or(0, |(sequence, _, _)| *sequence);
    let due = map
        .facts
        .next_movement_decay_time()
        .ok_or("five-minute reset")?;
    assert!((before_use + 300_000_000..=after_use + 300_000_000).contains(&due));
    assert_eq!(map.facts.expire_movement_items(due - 1), 0);
    let input:(i64,i64,bool)=sqlx::query_as("SELECT i.quantity,i.lifecycle::bigint,p.proof_sha256=sha256(p.proof) FROM game_quest_shared_input_proofs p JOIN game_item_instances i USING(item_instance_id)").fetch_one(&h.pool).await?;
    assert_eq!(input, (1, 1, true));
    let shape:(i32,bool)=sqlx::query_as("SELECT cardinality(track_keys),completes FROM game_character_quest_receipts WHERE transition_key=$1").bind(&callback.key).fetch_one(&h.pool).await?;
    assert_eq!(shape, (0, false));
    let reply = super::quest_world_callbacks::wire(owner, admitted, 2, 2, payload(0)?).await?;
    assert_eq!(
        decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Committed
    );
    assert_eq!(
        owner
            .bundle_map()
            .ok_or("map")?
            .facts
            .next_movement_decay_time(),
        Some(due)
    );
    assert!(
        owner
            .bundle_map()
            .ok_or("map")?
            .facts
            .movement_effects_since(cue_cursor)
            .is_empty(),
        "replay cannot repeat ignition cues"
    );
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("pacing")?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, 3, 2, payload(1)?).await?;
    assert_eq!(
        decode_use_result(&reply).map_err(debug)?,
        UseDisposition::NothingToUse
    );
    let map = owner.bundle_map().ok_or("map")?;
    assert_eq!(map.facts.expire_movement_items(due), 2);
    assert_eq!(
        map.facts
            .base_entry(pos, 1, 1)
            .ok_or("restored ashes")?
            .appearance_id,
        3134
    );
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("pacing")?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, 4, 2, payload(2)?).await?;
    assert_eq!(
        decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Committed
    );
    let second_due = owner
        .bundle_map()
        .ok_or("map")?
        .facts
        .next_movement_decay_time()
        .ok_or("second reset")?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("pacing")?;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        5,
        1,
        encode_step_intent(StepDirection::East),
    )
    .await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    let actor = admitted.runtime_actor.ok_or("actor")?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        super::quest_world_callbacks::local(dreamer_fire_bug::DESTINATION)
    );
    let map = owner.bundle_map().ok_or("map")?;
    assert_eq!(
        map.facts
            .base_entry(pos, 1, 1)
            .ok_or("shared portal remains")?
            .appearance_id,
        1949
    );
    assert_eq!(
        map.facts.object_revision(key),
        3,
        "traversal must not invalidate the shared timer"
    );
    assert_eq!(map.facts.next_movement_decay_time(), Some(second_due));
    assert_eq!(
        map.facts
            .quest_portal_destination(pos, owner.owner_now().get()),
        Some(crate::map::overlay::TilePos {
            x: dreamer_fire_bug::DESTINATION.x,
            y: dreamer_fire_bug::DESTINATION.y,
            floor: dreamer_fire_bug::DESTINATION.floor,
        })
    );
    assert_eq!(map.facts.expire_movement_items(second_due), 2);
    assert_eq!(
        map.facts
            .base_entry(pos, 1, 1)
            .ok_or("expired ashes")?
            .appearance_id,
        3134
    );
    assert_eq!(map.facts.quest_portal_destination(pos, 0), None);
    let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_quest_shared_input_proofs),(SELECT count(*) FROM game_quest_callback_consumptions),(SELECT count(*) FROM game_character_quest_tracks)").fetch_one(&h.pool).await?;
    assert_eq!(counts, (2, 0, 0));
    let destination = dreamer_fire_bug::DESTINATION;
    let landing = crate::map::overlay::TilePos {
        x: destination.x,
        y: destination.y,
        floor: destination.floor,
    };
    assert_eq!(
        map.facts
            .movement_effects_since(cue_cursor)
            .iter()
            .filter(|(_, position, effect)| *position == landing && *effect == 11)
            .count(),
        1
    );
    // Hash-valid proofs for a different source or content pin cannot admit ignition.
    for (path, value) in [
        (vec!["source"], serde_json::json!([32850, 32233, 9, 1])),
        (
            vec!["context", "content_revision"],
            serde_json::json!(format!("sha256:{}", "00".repeat(32))),
        ),
        (
            vec!["context", "placement_context"],
            serde_json::json!(vec![0_u8; 32]),
        ),
    ] {
        let mut tx = h.pool.begin().await?;
        let error=sqlx::query("INSERT INTO game_quest_shared_input_proofs(character_id,cause_id,cause_ordinal,transition_key,world_id,channel_id,item_instance_id,before_item,location,kind,proof,proof_sha256) SELECT character_id,cause_id,1000,transition_key,world_id,channel_id,item_instance_id,before_item,location,kind,changed,sha256(changed) FROM (SELECT p.*,convert_to(jsonb_set(convert_from(proof,'UTF8')::jsonb,$1::text[],$2)::text,'UTF8') AS changed FROM game_quest_shared_input_proofs p LIMIT 1) forged")
            .bind(path).bind(value).execute(&mut *tx).await.err().ok_or("forged proof admitted")?;
        assert_eq!(
            error
                .as_database_error()
                .and_then(|error| error.code())
                .as_deref(),
            Some("23514")
        );
        tx.rollback().await?;
    }
    bundle_channel_frame_admission(owner, h).await?;
    let after = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(after.tracks(), before.tracks());
    eprintln!(
        "M07 POSTGRES_CASE_COMPLETED shared_fire_bug real_USE real_STEP retained_item no_progress five_minute_reset replay_no_extension"
    );
    Ok(())
}

/// Production bundle Channels pin the bundle frame of the published map, while
/// the content activation retains the entry frame (channel_boot bundle_channel_pin).
/// Ignition admits that bundle frame only when the active reset map proves it.
async fn bundle_channel_frame_admission(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    h: &super::fixture::Harness,
) -> TestResult {
    const PROOF: &str =
        "(SELECT * FROM game_quest_shared_input_proofs ORDER BY cause_id LIMIT 1) p";
    let copy = format!(
        "INSERT INTO game_quest_shared_input_proofs(character_id,cause_id,cause_ordinal,transition_key,world_id,channel_id,item_instance_id,before_item,location,kind,proof,proof_sha256) \
         SELECT character_id,cause_id,1001,transition_key,world_id,channel_id,item_instance_id,before_item,location,kind,changed,sha256(changed) \
         FROM (SELECT p.*,convert_to(jsonb_set(convert_from(p.proof,'UTF8')::jsonb,'{{context,placement_context}}',\
          (SELECT jsonb_agg(get_byte($1,b.n) ORDER BY b.n) FROM generate_series(0,octet_length($1)-1) AS b(n)))::text,'UTF8') AS changed \
          FROM {PROOF}) framed"
    );
    // The retained context carries the raw published bundle revision: the input
    // of the bundle frame (qn1-elemental-world-pin proves SQL equals the Rust pin).
    let revision: String = sqlx::query_scalar(&format!(
        "SELECT convert_from(p.proof,'UTF8')::jsonb->'context'->>'map_revision' FROM {PROOF}"
    ))
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        revision,
        crate::map::overlay::map_revision(owner.bundle_map().ok_or("map")?.overlay.base())
    );
    let (entry, bundle): (Vec<u8>, Vec<u8>) = sqlx::query_as(&format!(
        "SELECT a.frame_binding_digest,game_elemental_bundle_frame(p.world_id,convert_from(p.proof,'UTF8')::jsonb->'context'->>'map_revision') \
         FROM {PROOF} JOIN game_content_activations a ON a.world_id=p.world_id AND a.channel_id=p.channel_id \
         ORDER BY a.activation_sequence DESC LIMIT 1"
    ))
    .fetch_one(&h.pool)
    .await?;
    assert_ne!(
        entry, bundle,
        "the bundle frame differs from the activation entry frame"
    );
    // Control: the retained entry-frame proof is admitted unchanged.
    let mut tx = h.pool.begin().await?;
    sqlx::query(&copy)
        .bind(entry.as_slice())
        .execute(&mut *tx)
        .await?;
    tx.rollback().await?;
    // Without an active reset map the bundle frame proves nothing.
    let mut tx = h.pool.begin().await?;
    let error = sqlx::query(&copy)
        .bind(bundle.as_slice())
        .execute(&mut *tx)
        .await
        .err()
        .ok_or("unproven bundle frame admitted")?;
    assert_eq!(
        error
            .as_database_error()
            .and_then(|error| error.code())
            .as_deref(),
        Some("23514")
    );
    tx.rollback().await?;
    // The active reset map with this content and map revision proves the bundle frame.
    let mut tx = h.pool.begin().await?;
    sqlx::query(&format!(
        "INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) \
         SELECT p.world_id,0,a.server_artifact_digest,convert_from(p.proof,'UTF8')::jsonb->'context'->>'map_revision',\
          a.server_artifact_digest,a.client_artifact_digest,a.frame_binding_digest,'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp(),'2026-11' \
         FROM {PROOF} JOIN game_content_activations a ON a.world_id=p.world_id AND a.channel_id=p.channel_id \
         ORDER BY a.activation_sequence DESC LIMIT 1"
    ))
    .execute(&mut *tx)
    .await?;
    sqlx::query(&format!(
        "INSERT INTO game_world_reset_active SELECT p.world_id,0 FROM {PROOF}"
    ))
    .execute(&mut *tx)
    .await?;
    sqlx::query(&copy)
        .bind(bundle.as_slice())
        .execute(&mut *tx)
        .await?;
    tx.rollback().await?;
    eprintln!("M07 POSTGRES_CASE_COMPLETED dreamer_ignition_accepts_bundle_channel_frame");
    Ok(())
}
