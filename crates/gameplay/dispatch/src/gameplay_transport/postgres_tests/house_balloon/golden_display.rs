use super::*;
pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    source: [u8; 16],
    before: std::num::NonZeroU32,
    after: std::num::NonZeroU32,
    full: bool,
) -> TestResult {
    // Historical quest prerequisite fixture only. This does not claim that a
    // purchase/grant happened; the separate real royal purchase test owns that.
    if full {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,'oteryn:quest-progress/crystalserver/outfit_quest/golden_outfit','oteryn:quest.golden_outfits_quest',3,2) ON CONFLICT(character_id,track_key) DO UPDATE SET value=3").bind(id(41).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
    }
    let capture = std::sync::Arc::new(std::sync::Mutex::new(
        super::wire::HouseWireCapture::default(),
    ));
    let body = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let (_, outcome, _, _) = exchange(
        owner,
        admitted,
        1,
        &body,
        HouseExchange {
            rotate: false,
            current: Some(before),
            after: full.then_some(after),
            nested: None,
            foreign_after_snapshot: None,
            capture: Some(std::sync::Arc::clone(&capture)),
        },
    )
    .await?;
    let actual:String=sqlx::query_scalar("SELECT definition_production_key FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?;
    if full {
        assert_eq!(outcome, UseDisposition::Committed);
        assert_eq!(actual, "oteryn:item.tibia.i31502");
    } else {
        assert_ne!(outcome, UseDisposition::Committed);
        assert_eq!(actual, "oteryn:item.tibia.i31510");
    }
    let payloads = capture
        .lock()
        .map_err(|_| "Golden display capture")?
        .effects
        .clone();
    assert_eq!(
        payloads.len(),
        1,
        "source success171 or refused3 must reach actual native effect publisher"
    );
    let effects = oteryn_protocol_oteryn::presentation::decode_item_effect_batch(&payloads[0])
        .map_err(debug)?;
    assert!(effects.iter().any(|event|matches!(event.kind,oteryn_protocol_oteryn::presentation::PresentationEventKind::MagicEffect{effect_id,..}if effect_id==if full {171}else{3})));
    if !full {
        let lines = capture
            .lock()
            .map_err(|_| "Golden private feedback")?
            .chat_lines
            .clone();
        assert_eq!(lines.iter().filter(|bytes|matches!(oteryn_protocol_oteryn::chat::decode_chat_line(bytes),Ok(oteryn_protocol_oteryn::chat::ChatLine::Local{text,..})if text=="You need Full Golden Outfit to use it.")).count(),1,"actual native private feedback must retain source text");
    }
    Ok(())
}
