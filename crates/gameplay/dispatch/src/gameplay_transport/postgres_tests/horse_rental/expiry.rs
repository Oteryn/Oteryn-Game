//! Real25 caller, truthful historical prior, source clock expiry, and native wire capture.
use super::*;
use crate::durability::horse_rental::rules;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::AdmittedSession;
use oteryn_protocol_oteryn::appearance as wire;
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub(super) async fn exercise(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
) -> TestResult {
    let started = std::time::Instant::now();
    let actor = admitted.runtime_actor.ok_or("rental actor")?;
    let first:(i32,i64,i64)=sqlx::query_as("SELECT mount_id,issued_seconds,expires_seconds FROM game_horse_rental_receipts WHERE character_id=encode($1,'hex')::uuid ORDER BY issued_seconds DESC LIMIT 1")
  .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(first.2 - first.1, 86400, "source24realhours");
    let first_fee:i64=sqlx::query_scalar("SELECT fee_gold_units FROM game_item_fee_burns WHERE cause_kind=473 ORDER BY occurred_at DESC LIMIT 1").fetch_one(&h.pool).await?;
    assert_eq!(first_fee, 500);
    // Re-present the exact committed native confirmation, then a genuinely new
    // confirmation during the active contract. Neither renews nor charges.
    super::super::grizzly_npc::wire::talk(owner, admitted, npc, &[(3, "yes")]).await?;
    let next = admitted.continuity.next_command_id;
    super::super::grizzly_npc::wire::talk(
        owner,
        admitted,
        npc,
        &[(next, "hi"), (next + 1, "rent"), (next + 2, "yes")],
    )
    .await?;
    let untouched:(i64,i64)=sqlx::query_as("SELECT count(*),max(expires_seconds) FROM game_horse_rental_receipts WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        untouched,
        (1, first.2),
        "replay and active renewal keep the original contract"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_item_fee_burns WHERE cause_kind=473"
        )
        .fetch_one(&h.pool)
        .await?,
        1,
        "source active contract never charges twice"
    );
    let capture = super::super::pumpkin_skinning_use::take_claim_frames();
    drop(capture);
    let (context, content) = owner
        .appearance_context()
        .ok_or("source appearance context")?;
    let observed = owner
        .appearance_observation(actor, admitted.game_session_id, Vec::new())
        .await
        .ok_or("rental catalogue")?;
    let outfit = observed
        .catalogue
        .outfits
        .first()
        .ok_or("starter outfit")?
        .key
        .clone();
    let selected_key = rules::Horse::from_source_id(u16::try_from(first.0)?)
        .ok_or("source horse")?
        .key()
        .to_owned();
    assert!(
        !owner
            .session_premium_gate(admitted.game_session_id)
            .await
            .premium_current(),
        "fixture really has no current Premium authority"
    );
    let change = wire::Change {
        context,
        expected_root_revision: observed.catalogue.root_revision,
        prior: observed.catalogue.selected.clone(),
        selection: wire::Selection {
            outfit_key: outfit.clone(),
            colours: [1, 2, 3, 4],
            addons: 0,
            mount_key: Some(selected_key),
        },
        content_revision: content.content_revision().into(),
    };
    let (status, reply) = super::super::pumpkin_skinning_use::exchange(
        owner,
        admitted,
        wire::COMMAND_TYPE_APPEARANCE_CHANGE,
        &change.encode().map_err(debug)?,
        None,
    )
    .await?;
    if rules::Horse::from_source_id(u16::try_from(first.0)?).is_some_and(|h| h.premium()) {
        assert_eq!(
            status,
            oteryn_protocol_oteryn::CommandStatus::Rejected,
            "buying is free-account compatible; premium-only colors cannot be worn"
        );
        assert_eq!(
            wire::ChangeResult::decode(&reply).map_err(debug)?,
            wire::ChangeResult::Rejected
        );
    }
    historical_permanent_forgeries(h).await?;
    let recovered = owner
        .appearance_observation(actor, admitted.game_session_id, Vec::new())
        .await
        .ok_or("recovery read of legacyforgeries")?;
    assert_eq!(
        recovered
            .catalogue
            .mounts
            .iter()
            .filter(|key| rules::Horse::for_key(key).is_some())
            .count(),
        1,
        "recovery ignores all3historicalpermanentforgeries; only the currentpersonalcontract survives"
    );
    // Source equality is live and cannot renew; one second later the same current
    // independently fenced Character loses only its temporary earned fact.
    clock(h, first.2).await?;
    let exact = owner
        .appearance_observation(actor, admitted.game_session_id, Vec::new())
        .await
        .ok_or("exact expiry catalogue")?;
    assert!(
        exact
            .catalogue
            .mounts
            .iter()
            .any(|k| rules::Horse::for_key(k)
                .is_some_and(|horse| i32::from(horse.source_id()) == first.0)),
        "Timer>=now remains owned at equality"
    );
    clock(h, first.2 + 1).await?;
    let expired = owner
        .appearance_observation(actor, admitted.game_session_id, Vec::new())
        .await
        .ok_or("expired catalogue")?;
    assert!(
        !expired
            .catalogue
            .mounts
            .iter()
            .any(|k| rules::Horse::for_key(k).is_some()),
        "expiry removes all3temporary choices"
    );
    // Find a real future confirmation yielding the free regular horse. No fake
    // grant or command reference: every skipped choice sends actual rent/no.
    let mut acquired = false;
    for _ in 0..64 {
        let next = admitted.continuity.next_command_id;
        // The confirmation belongs to this same real hi/rent connection;
        // closing the stream after rent correctly clears NPC focus.
        let yes = next.checked_add(2).ok_or("rental command range")?;
        // The actual source seed is immutable World+Character+GameSession+command,
        // assembled by PostgreSQL exactly as the private production arm.
        let wanted:bool=sqlx::query_scalar("SELECT game_horse_station_draw($1,sha256(uuid_send(encode($2,'hex')::uuid)||uuid_send(encode($3,'hex')::uuid)||uuid_send(encode($4,'hex')::uuid)||convert_to($5,'UTF8')),3)=1 AND game_horse_station_draw($6,sha256(uuid_send(encode($2,'hex')::uuid)||uuid_send(encode($3,'hex')::uuid)||uuid_send(encode($4,'hex')::uuid)||convert_to($5,'UTF8')),10)>3")
   .bind(i32::from(rules::RNG_RENTAL)).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(admitted.game_session_id.as_bytes().as_slice()).bind(yes.to_string()).bind(i32::from(rules::RNG_ESCAPE)).fetch_one(&h.pool).await?;
        super::super::grizzly_npc::wire::talk(
            owner,
            admitted,
            npc,
            &[
                (next, "hi"),
                (next + 1, "rent"),
                (yes, if wanted { "yes" } else { "no" }),
            ],
        )
        .await?;
        if wanted {
            acquired = true;
            break;
        }
    }
    assert!(
        acquired,
        "bounded real source confirmations find the regular horse"
    );
    let regular:(i32,i64,i64)=sqlx::query_as("SELECT mount_id,issued_seconds,expires_seconds FROM game_horse_rental_receipts WHERE character_id=encode($1,'hex')::uuid ORDER BY issued_seconds DESC LIMIT 1").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(regular.0, 22);
    assert_eq!(regular.2 - regular.1, 86400);
    let observed = owner
        .appearance_observation(actor, admitted.game_session_id, Vec::new())
        .await
        .ok_or("regular catalogue")?;
    let change = wire::Change {
        context: observed.catalogue.context,
        expected_root_revision: observed.catalogue.root_revision,
        prior: observed.catalogue.selected,
        selection: wire::Selection {
            outfit_key: outfit.clone(),
            colours: [1, 2, 3, 4],
            addons: 0,
            mount_key: Some(rules::Horse::Regular.key().into()),
        },
        content_revision: content.content_revision().into(),
    };
    let (status, reply) = super::super::pumpkin_skinning_use::exchange(
        owner,
        admitted,
        wire::COMMAND_TYPE_APPEARANCE_CHANGE,
        &change.encode().map_err(debug)?,
        None,
    )
    .await?;
    eprintln!(
        "SRV7_HORSE_REGULAR_APPEARANCE command={} mount={} palette_len={} catalogued_mount={} prior={:?} requested={:?} expected_root={} selected_caps={:?} status={status:?} reply={:?}",
        admitted.continuity.next_command_id.saturating_sub(1),
        regular.0,
        observed.catalogue.palette.len(),
        observed
            .catalogue
            .mounts
            .iter()
            .any(|key| key == rules::Horse::Regular.key()),
        change.prior,
        change.selection,
        change.expected_root_revision,
        admitted.continuity.selected_capabilities.as_slice(),
        wire::ChangeResult::decode(&reply)
    );
    assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
    assert!(matches!(
        wire::ChangeResult::decode(&reply).map_err(debug)?,
        wire::ChangeResult::Committed(_)
    ));
    assert_eq!(
        owner.current_mount(actor, admitted.game_session_id).await,
        Some(421)
    );
    let _ = super::super::pumpkin_skinning_use::take_claim_frames();
    let hi = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(
        &oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
            npc_actor: npc,
            text: "hi".into(),
        },
    )
    .map_err(debug)?;
    super::super::pumpkin_skinning_use::exchange(owner, admitted, 7, &hi, None).await?;
    let active_frames = super::super::pumpkin_skinning_use::take_claim_frames();
    clock(h, regular.2 + 1).await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(16))
        .ok_or("advance source15second owner cadence")?;
    let expired = owner
        .appearance_observation(actor, admitted.game_session_id, Vec::new())
        .await
        .ok_or("expired regular catalogue")?;
    assert!(
        expired
            .catalogue
            .selected
            .as_ref()
            .is_some_and(|s| s.selection.mount_key.as_deref() == Some(rules::Horse::Regular.key())),
        "historical before witness remains truthful"
    );
    assert!(
        !expired
            .catalogue
            .mounts
            .iter()
            .any(|k| rules::Horse::for_key(k).is_some())
    );
    assert_eq!(
        owner.current_mount(actor, admitted.game_session_id).await,
        Some(0)
    );
    let after = wire::Change {
        context: expired.catalogue.context,
        expected_root_revision: expired.catalogue.root_revision,
        prior: expired.catalogue.selected.clone(),
        selection: wire::Selection {
            outfit_key: outfit,
            colours: [2, 1, 3, 4],
            addons: 0,
            mount_key: None,
        },
        content_revision: content.content_revision().into(),
    };
    let (status, reply) = super::super::pumpkin_skinning_use::exchange_observe(
        owner,
        admitted,
        wire::COMMAND_TYPE_APPEARANCE_CHANGE,
        &after.encode().map_err(debug)?,
        None,
        std::time::Duration::from_millis(1500),
    )
    .await?;
    assert_eq!(
        status,
        oteryn_protocol_oteryn::CommandStatus::Accepted,
        "expired historical choice never locks out the next real native outfit command"
    );
    assert!(matches!(
        wire::ChangeResult::decode(&reply).map_err(debug)?,
        wire::ChangeResult::Committed(_)
    ));
    let expired_frames = super::super::pumpkin_skinning_use::take_claim_frames();
    assert_eq!(
        expiry_notices(&expired_frames)?,
        1,
        "actual native92 emits one source expiry Advance after current15s cadence"
    );
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(16))
        .ok_or("second source owner cadence")?;
    super::super::pumpkin_skinning_use::exchange_observe(
        owner,
        admitted,
        7,
        &hi,
        None,
        std::time::Duration::from_millis(1500),
    )
    .await?;
    let replay_notices = super::super::pumpkin_skinning_use::take_claim_frames();
    assert_eq!(
        expiry_notices(&replay_notices)?,
        0,
        "real native rejoin does not repeat an acknowledged expiry Advance"
    );
    let fees: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_item_fee_burns WHERE cause_kind=473")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        fees, 2,
        "expiry/native rejoin/appearance changes never renew or charge"
    );
    if let Some(path) = std::env::var_os("OTERYN_SRV7_HORSE_CAPTURE") {
        let at = owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        std::fs::write(
            path,
            serde_json::to_vec_pretty(
                &serde_json::json!({"producer_test":"srv7_horse_rental_real_npc_acquires_personal_twenty_four_hour_mount","phases":{"active":active_frames,"expired_then_clear":expired_frames},"content_revision":content.content_revision(),"selected_capabilities":admitted.continuity.selected_capabilities.as_slice(),"observer_runtime":{"x":at.x,"y":at.y,"floor":at.floor},"observer_native":{"x":at.x,"y":at.y,"floor":-i32::from(at.floor)},"actor_hex":hex(&actor.placement_identity()),"rental":{"mount":regular.0,"issued":regular.1,"expires":regular.2}}),
            )?,
        )?;
    }
    eprintln!(
        "SRV7_HORSE_EXPIRY_NATIVE25_EXECUTED elapsed={:.3}s fee_receipts=2 source_clock_verified=true",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
pub(super) async fn clock(h: &Harness, seconds: i64) -> TestResult {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS public.srv7_horse_fixture_clock(seconds BIGINT NOT NULL)",
    )
    .execute(&h.pool)
    .await?;
    sqlx::query("DELETE FROM public.srv7_horse_fixture_clock")
        .execute(&h.pool)
        .await?;
    sqlx::query("INSERT INTO public.srv7_horse_fixture_clock VALUES($1)")
        .bind(seconds)
        .execute(&h.pool)
        .await?;
    sqlx::query("CREATE OR REPLACE FUNCTION public.game_daily_effect_now()RETURNS TIMESTAMPTZ LANGUAGE SQL STABLE SET search_path=pg_catalog AS $$SELECT to_timestamp(seconds) FROM public.srv7_horse_fixture_clock$$").execute(&h.pool).await?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT extract(epoch FROM public.game_daily_effect_now())::bigint"
        )
        .fetch_one(&h.pool)
        .await?,
        seconds
    );
    Ok(())
}

fn expiry_notices(frames: &[serde_json::Value]) -> TestResult<usize> {
    let mut notices = 0;
    for frame in frames.iter().filter(|f| {
        f["kind"] == "delta"
            && f["domain"] == oteryn_protocol_oteryn::zones_nologout::STATE_DOMAIN_SERVER_MESSAGES
    }) {
        let encoded = frame["payload_hex"]
            .as_str()
            .ok_or("actual notice payload")?;
        if !encoded.len().is_multiple_of(2) {
            return Err("partial actual notice hex".into());
        }
        let bytes = encoded
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair)?, 16).map_err(Into::into))
            .collect::<TestResult<Vec<u8>>>()?;
        if oteryn_protocol_oteryn::zones_nologout::decode_server_message(&bytes).map_err(debug)?
            == oteryn_protocol_oteryn::zones_nologout::ServerMessage::Advance(
                rules::EXPIRY_TEXT.into(),
            )
        {
            notices += 1
        }
    }
    Ok(notices)
}

/// Controlled historical corruption fixture only. No current rental/Node/session
/// receipt, source clock, Character revision or fee is fabricated or changed.
async fn historical_permanent_forgeries(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (key, source) in [
        ("oteryn:rental.horse.22", "quest"),
        ("oteryn:rental.horse.25", "npc"),
        ("oteryn:rental.horse.26", "item_action"),
    ] {
        let occurrence: sqlx::types::Uuid = sqlx::query_scalar("SELECT game_character_uuid_v7()")
            .fetch_one(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_account_appearance_unlock_receipts(grant_occurrence_id,account_id,unlock_key,content_revision,granting_character_id,source_kind,source_event_id) SELECT $1,account_id,$2,$3,character_id,$4,uuid_send($1) FROM game_character_roots WHERE character_id=encode($5,'hex')::uuid")
   .bind(occurrence).bind(key).bind(rules::PROVENANCE).bind(source).bind(id(41).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_account_appearance_unlocks(account_id,unlock_key,grant_occurrence_id)SELECT account_id,$1,$2 FROM game_character_roots WHERE character_id=encode($3,'hex')::uuid")
   .bind(key).bind(occurrence).bind(id(41).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_account_appearance_unlocks WHERE unlock_key IN('oteryn:rental.horse.22','oteryn:rental.horse.25','oteryn:rental.horse.26')").fetch_one(&h.pool).await?,3,"historical corruption fixture actually exists");
    Ok(())
}
