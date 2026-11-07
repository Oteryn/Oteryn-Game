//! Small adapters around the single existing fresh-admission fixture.
use super::fixture::debug;
use super::{Dependencies, TestResult};
use crate::foundation::AuthenticatedTransportRefV1;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::FirstEntryOutcome};

pub(super) async fn admit(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    dependencies: &Dependencies,
    request: &crate::foundation::fresh_admission_durability::FreshAdmissionCommitRequestV1,
) -> TestResult {
    let (outcome, _, _) = Box::pin(owner.initialize_first_entry(
        request,
        dependencies.admitted.game_session_id,
        AuthenticatedTransportRefV1::decode(&[9; 16]).map_err(debug)?,
        dependencies.actor,
    ))
    .await;
    assert!(
        matches!(
            outcome,
            FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled
        ),
        "{outcome:?}"
    );
    Ok(())
}

/// Publish the fixture's actual native generation through the existing Content control API.
/// Admission and Premium then independently compare this SQL evidence with the runtime pin.
pub(super) async fn publish_native_activation(
    h: &super::fixture::Harness,
    pin: &crate::content::NativeEntryContentPin,
) -> TestResult {
    use super::fixture::id;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,NULL,$4,$5,$6)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(pin.activation_sequence().to_string())
        .bind(pin.identity().server_artifact_digest().as_slice())
        .bind(pin.identity().client_artifact_digest().as_slice())
        .bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
    let stored: Vec<u8> = sqlx::query_scalar("SELECT server_artifact_digest FROM game_content_activations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid ORDER BY activation_sequence DESC LIMIT 1")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        stored.as_slice(),
        pin.identity().server_artifact_digest().as_slice()
    );
    Ok(())
}

/// Normalize only historical fixture asset seeds to the admitted native definitions.
/// All subsequent admission and item transitions use the restricted runtime role.
pub(super) async fn native_asset_revisions(
    h: &super::fixture::Harness,
    active: &crate::content::ActiveGeneration,
) -> TestResult {
    use super::fixture::id;
    let index = active
        .native_gameplay()
        .and_then(|native| native.item_index())
        .ok_or("native Item index")?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT item_instance_id::text,definition_production_key FROM game_item_instances \
         WHERE world_id=encode($1,'hex')::uuid AND lifecycle=1 ORDER BY item_instance_id LIMIT 100",
    )
    .bind(id(42).as_slice())
    .fetch_all(&mut *tx)
    .await?;
    assert!(rows.len() < 100, "fixture assets fit the bounded seed");
    for (item, key) in rows {
        let revision = index.revision(&key).ok_or("native seeded item")?;
        sqlx::query("UPDATE game_item_instances SET definition_revision_ref=$2 WHERE item_instance_id=$1::uuid")
            .bind(item).bind(revision).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}
