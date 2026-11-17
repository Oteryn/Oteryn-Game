//! Command 6214 uses the selected v2 route and the real fee, item and receipt owners.
use super::*;
use wire::v2;

fn request(before: &wire::AtelierReply, occurrence: u8) -> TestResult<v2::ReshapeRequest> {
    let gem = before
        .gems
        .iter()
        .find(|gem| gem.quality == 1)
        .ok_or("regular gem")?;
    Ok(v2::ReshapeRequest {
        occurrence: id(occurrence),
        expected_revision: before.revision,
        gem: gem.id,
        position: 1,
    })
}

pub(super) async fn refused_without_premium(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    before: &wire::AtelierReply,
) -> TestResult {
    let footprint = scope_footprint(h).await?;
    advance_ex_action(owner)?;
    let payload = v2::encode_reshape(&request(before, 242)?).map_err(debug)?;
    let reply = v2::decode_reply(
        &send_atelier(
            owner,
            admitted,
            v2::COMMAND_TYPE_ATELIER_V2_RESHAPE,
            &payload,
        )
        .await?,
    )
    .map_err(debug)?;
    assert!(reply.refused && !reply.premium);
    assert_eq!(reply.revision, before.revision);
    assert_eq!(
        scope_footprint(h).await?,
        footprint,
        "a fresh non-Premium reshape changes no custody or balance"
    );
    Ok(())
}

pub(super) async fn exercise(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    before: &wire::AtelierReply,
) -> TestResult {
    assert!(
        admitted
            .continuity
            .selected_capabilities
            .contains(v2::CAPABILITY_GEM_ATELIER_V2)
    );
    let query = v2::decode_reply(
        &send_atelier(owner, admitted, v2::COMMAND_TYPE_ATELIER_V2_QUERY, &[]).await?,
    )
    .map_err(debug)?;
    assert!(
        query.premium && query.in_temple,
        "the real route must qualify location and Premium"
    );
    let request = request(before, 243)?;
    let original = query
        .gems
        .iter()
        .find(|gem| gem.id == request.gem)
        .ok_or("queried gem")?;
    let fragments: i64 = sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(229).as_slice())
    .fetch_one(&h.pool)
    .await?;
    let counts = footprint(h).await?;
    advance_ex_action(owner)?;
    let payload = v2::encode_reshape(&request).map_err(debug)?;
    let bytes = send_atelier(
        owner,
        admitted,
        v2::COMMAND_TYPE_ATELIER_V2_RESHAPE,
        &payload,
    )
    .await?;
    let reply = v2::decode_reply(&bytes).map_err(debug)?;
    assert!(!reply.refused);
    assert_eq!(reply.revision, before.revision + 1);
    let changed = reply
        .gems
        .iter()
        .find(|gem| gem.id == request.gem)
        .ok_or("reshaped gem")?;
    assert_eq!(
        (changed.quality, changed.locked, changed.slot),
        (original.quality, original.locked, original.slot)
    );
    assert_eq!(changed.modifiers.len(), original.modifiers.len());
    assert_ne!(changed.modifiers[0], original.modifiers[0]);
    assert_eq!(changed.modifiers[1], original.modifiers[1]);
    assert_ne!(changed.modifiers[0], changed.modifiers[1]);
    let remaining: i64 = sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(229).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(remaining, fragments - 1);
    let paid = footprint(h).await?;
    assert_eq!((paid.0, paid.1), (counts.0 + 1, counts.1 + 1));
    let fee: i64 = sqlx::query_scalar("SELECT fee_gold_units FROM game_item_fee_burns WHERE cause_kind=483 AND cause_occurrence_id=encode($1,'hex')::uuid")
        .bind(request.occurrence.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(fee, 125_000);
    let committed = scope_footprint(h).await?;
    let replay = send_atelier(
        owner,
        admitted,
        v2::COMMAND_TYPE_ATELIER_V2_RESHAPE,
        &payload,
    )
    .await?;
    assert_eq!(
        replay, bytes,
        "the same occurrence returns its retained result"
    );
    assert_eq!(
        scope_footprint(h).await?,
        committed,
        "replay cannot burn another fragment or fee"
    );
    Ok(())
}
