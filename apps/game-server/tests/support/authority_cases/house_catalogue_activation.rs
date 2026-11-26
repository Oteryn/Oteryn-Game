use crate::bestiary_postgres_harness::{
    CHANNEL, CHARACTER, Harness, TestResult, WORLD, debug, id, item_fence, runtime,
};
use crate::durability::content_activation::ContentActivationRequest;
use crate::durability::house_management::{HouseManagementPage, QualifiedHouse};
use crate::foundation::{ChannelId, WorldId};

const PRIVATE: &str = "oteryn:content.house.admiral_s_avenue_1";
const HALL: &str = "oteryn:content.house.blessed_shield_guildhall";

#[test]
fn content_activation_lists_authored_houses_and_preserves_owner_and_auction() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime()
                .map_err(debug)?
                .block_on(async move {
                    let harness =
                        Harness::create(admin, "house_catalogue_activation", false).await?;
                    let result = activation_case(&harness).await;
                    harness.cleanup().await?;
                    result
                })
                .map_err(debug)
        })?
        .join()
        .map_err(|_| "house catalogue activation test thread failed")?
        .map_err(Into::into)
}

async fn activation_case(h: &Harness) -> TestResult {
    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
    let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;
    let mut request = ContentActivationRequest {
        world_id: world,
        channel_id: channel,
        activation_sequence: 1,
        previous_sequence: None,
        server_artifact_digest: [41; 32],
        client_artifact_digest: [42; 32],
        frame_binding_digest: [43; 32],
    };
    assert!(!h.root.record_content_activation(&request).await?);
    assert!(
        h.root
            .read_current_content_activation(world, channel)
            .await?
            .is_none()
    );
    assert!(!sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM game_house_properties WHERE world_id=encode($1,'hex')::uuid)"
    ).bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?);
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
        .execute(&h.pool).await?;
    assert!(h.root.record_content_activation(&request).await?);

    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let page = || HouseManagementPage {
        houses: vec![
            QualifiedHouse {
                key: PRIVATE.into(),
                kind: "private_house",
            },
            QualifiedHouse {
                key: HALL.into(),
                kind: "guildhall",
            },
        ],
        cursor: None,
        auction_order: false,
    };
    let rows = h
        .root
        .read_house_management(
            &authority,
            &h.node,
            item_fence()?,
            page(),
            request.server_artifact_digest,
        )
        .await
        .map_err(debug)?;
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|row| row.house == PRIVATE
        && row.kind == "private_house"
        && row.state == 1
        && row.rent > 0));
    assert!(
        rows.iter().any(|row| row.house == HALL
            && row.kind == "guildhall"
            && row.state == 1
            && row.rent > 0)
    );

    // Seed valid owner and auction successors through the enabled persistence guards.
    let mut tx = h.pool.begin().await?;
    sqlx::query(
        "WITH property AS (
             UPDATE game_house_properties p SET state=3, owner_character_id=r.character_id,
               owner_account_id=r.account_id, ownership_generation=p.ownership_generation+1,
               revision=p.revision+1, acquisition_receipt_ref=game_character_uuid_v7(),
               paid_until=statement_timestamp()+interval '30 days'
             FROM game_character_roots r WHERE r.character_id=encode($1,'hex')::uuid
               AND p.world_id=r.world_id AND p.house_key=$2 RETURNING p.*
         ), slot AS (
             INSERT INTO game_account_housing_slots(account_id,world_id,house_key)
             SELECT owner_account_id,world_id,house_key FROM property
         )
         UPDATE game_house_ownership a SET owner_character_id=p.owner_character_id,
           ownership_revision=p.ownership_generation, acquisition_receipt_ref=p.acquisition_receipt_ref
         FROM property p WHERE (a.world_id,a.house_key)=(p.world_id,p.house_key)",
    )
    .bind(id(CHARACTER).as_slice()).bind(PRIVATE).execute(&mut *tx).await?;
    let auction: String = sqlx::query_scalar(
        "WITH property AS (
            UPDATE game_house_properties SET state=2,revision=revision+1
            WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 RETURNING *
         )
         INSERT INTO game_house_auctions(auction_id,world_id,house_key,first_rent,ends_at)
         SELECT game_character_uuid_v7(),world_id,house_key,rent_gold,
                statement_timestamp()+interval '7 days' FROM property
         RETURNING auction_id::text",
    )
    .bind(world.as_bytes().as_slice())
    .bind(HALL)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    let first = request.clone();
    for digest in [first.server_artifact_digest, [44; 32]] {
        request.previous_sequence = Some(request.activation_sequence);
        request.activation_sequence += 1;
        request.server_artifact_digest = digest;
        assert!(h.root.record_content_activation(&request).await?);
        assert!(
            h.root.record_content_activation(&request).await?,
            "current replay"
        );
        assert!(
            h.root.record_content_activation(&first).await?,
            "historical replay"
        );
        let rows = h
            .root
            .read_house_management(&authority, &h.node, item_fence()?, page(), digest)
            .await
            .map_err(debug)?;
        assert_eq!(rows.len(), 2);
        assert!(
            rows.iter().any(|row| row.house == PRIVATE
                && row.owned
                && row.state == 3
                && row.revision == 2)
        );
        assert!(rows.iter().any(|row| row.house == HALL
            && row.kind == "guildhall"
            && row.state == 2
            && row.ends_ms.is_some()
            && row.revision == 2));
        let retained: String = sqlx::query_scalar(
            "SELECT auction_id::text FROM game_house_auctions
             WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND NOT settled",
        )
        .bind(world.as_bytes().as_slice())
        .bind(HALL)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(retained, auction);
    }
    Ok(())
}
