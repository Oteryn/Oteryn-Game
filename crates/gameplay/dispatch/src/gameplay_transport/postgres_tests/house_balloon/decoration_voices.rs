//! Real House USE and replay for both source-authored decay_to voices.
use super::*;
pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    before: std::num::NonZeroU32,
    after: std::num::NonZeroU32,
    key: &str,
) -> TestResult {
    let text = match key {
        "oteryn:item.tibia.i23708" => "Au au!",
        "oteryn:item.tibia.i23443" => "Grooaarr!",
        _ => return Err("voice source".into()),
    };
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
    let view = || HouseExchange {
        rotate: false,
        current: Some(before),
        after: Some(after),
        nested: None,
        foreign_after_snapshot: None,
        capture: Some(std::sync::Arc::clone(&capture)),
    };
    let (next, result, original, _) = exchange(owner, admitted, 1, &body, view()).await?;
    assert_eq!(result, UseDisposition::Committed);
    let lines = capture
        .lock()
        .map_err(|_| "voice capture")?
        .chat_lines
        .clone();
    let matched=lines.iter().filter(|bytes|matches!(oteryn_protocol_oteryn::chat::decode_chat_line(bytes),Ok(oteryn_protocol_oteryn::chat::ChatLine::Local{mode:oteryn_protocol_oteryn::chat::ChatSpeechMode::MonsterSay,text:actual,..})if actual==text)).count();
    assert_eq!(
        matched, 1,
        "fresh decoration USE must publish its exact source MonsterSay on native House wire"
    );
    capture
        .lock()
        .map_err(|_| "voice capture")?
        .chat_lines
        .clear();
    owner.states.command_ingress.forget(next.game_session_id);
    let mut next = next;
    next.continuity.next_command_id = 1;
    let (_, replay, _, _) = exchange(
        owner,
        next,
        1,
        &original,
        HouseExchange {
            current: None,
            ..view()
        },
    )
    .await?;
    assert_eq!(replay, UseDisposition::Committed);
    assert!(
        capture
            .lock()
            .map_err(|_| "voice replay")?
            .chat_lines
            .is_empty()
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_consume_transform_records WHERE source_item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&h.pool).await?,1);
    eprintln!("SRV7_HOUSE_DECORATION_VOICE_EXECUTED source={key} text={text}");
    Ok(())
}
