use super::TestResult;
use super::fixture::{Harness, debug, id};
use super::server_modal_bed::command;
use crate::content::LogicalCell;
use crate::durability::bed_sleep::BedUseOutcome;
use crate::durability::consume_transform::HouseActionContext;
use crate::foundation::AuthenticatedTransportRefV1;
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, IDLE_LIVENESS, serve_admitted},
    tcp_tls::read_frame,
};
use crate::map::{overlay::TilePos, view::placement_key};
use oteryn_protocol_oteryn::{
    CommandStatus, MessageType, decode_snapshot_chunk, decode_wire_envelope,
    server_modal::{COMMAND_TYPE_SERVER_MODAL_QUERY, QUERY_WITH_TERMINAL_REPLY, Reply},
    world_map::{MapOrigin, STATE_DOMAIN_WORLD_MAP_VIEW, decode_world_map_snapshot},
    world_object::{
        COMMAND_TYPE_USE_INTENT, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
        decode_use_result, encode_item_use_intent,
    },
};

const HOUSE: &str = "oteryn:content.house.admiral_s_avenue_1";

const FIRST_SLEEPER: u8 = 121;
const NEXT_SLEEPER: u8 = 122;

pub(super) async fn scenario(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    // Only the historical sleepers are seeded. The USE actor's session, lease
    // and House custody come from genuine fresh admission and native handoff.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    for (sleeper, name) in [
        (FIRST_SLEEPER, "First Sleeper"),
        (NEXT_SLEEPER, "Second Sleeper"),
    ] {
        let record: String = sqlx::query_scalar(
            "SELECT (to_jsonb(r) || jsonb_build_object('character_id',encode($1,'hex')::uuid,'name',$2::text))::text \
             FROM game_character_roots r WHERE character_id=encode($3,'hex')::uuid",
        )
        .bind(id(sleeper).as_slice()).bind(name).bind(id(41).as_slice())
        .fetch_one(&mut *tx).await?;
        super::hireling_lifecycle_fixtures::insert_character_root_copy(&mut tx, &record).await?;
    }
    tx.commit().await?;
    seed_sleeper(h, FIRST_SLEEPER).await?;

    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let playing = async move {
        let (head, foot) = bed_targets(&mut client).await?;
        let mut next = admitted.continuity.next_command_id;
        let mut stale = admitted;
        stale
            .controller
            .as_mut()
            .ok_or("owner transport")?
            .transport = AuthenticatedTransportRefV1::decode(&id(123)).map_err(debug)?;
        let revision = bed_revision(h).await?;
        let refused = owner
            .offer_bed_modal(stale, next, foot.clone(), None)
            .await
            .map_err(debug)?
            .ok_or("stale transport must reach the bed owner")?;
        assert_eq!(refused.disposition, UseDisposition::StaleState);
        assert_eq!(bed_revision(h).await?, revision);

        let (status, bytes) = command(
            &mut client,
            admitted,
            &mut next,
            COMMAND_TYPE_SERVER_MODAL_QUERY,
            QUERY_WITH_TERMINAL_REPLY,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(Reply::decode(&bytes).map_err(debug)?, Reply::None);
        let wake_occurrence = next;
        assert_eq!(
            use_bed(&mut client, admitted, &mut next, foot.clone()).await?,
            (CommandStatus::Accepted, UseDisposition::BedWoken)
        );
        assert_eq!(bed_revision(h).await?, revision + 1);
        let first_free = freed(h, FIRST_SLEEPER).await?;
        let freed_at = first_free.1.ok_or("owner wake timestamp")?;
        let observed_at: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                .fetch_one(&h.pool)
                .await?;
        assert!((first_free.0 + 31_000..=observed_at).contains(&freed_at));
        assert_eq!(first_free.2.as_deref(), Some("owner_wake"));

        seed_sleeper(h, NEXT_SLEEPER).await?;
        let revision = bed_revision(h).await?;
        // The in-order connection returns its cached reply to the original wire
        // occurrence. Then exercise the durable replay below that cache as well.
        let mut repeated = wake_occurrence;
        assert_eq!(
            use_bed(&mut client, admitted, &mut repeated, foot).await?,
            (CommandStatus::DuplicateReplay, UseDisposition::BedWoken)
        );
        let context = {
            let runtime = owner.runtime.lock().await;
            HouseActionContext::from_native_owner(
                &runtime,
                admitted.runtime_actor.ok_or("House actor")?,
                admitted.game_session_id,
                admitted.item_fence.ok_or("House custody fence")?,
            )?
        };
        let replay = owner
            .production_bed_use(
                admitted,
                wake_occurrence,
                context,
                LogicalCell {
                    x: 32310,
                    y: 32804,
                    z: 7,
                },
            )
            .await
            .map_err(debug)?;
        assert!(matches!(replay, BedUseOutcome::Woken));
        assert_eq!(bed_revision(h).await?, revision);
        assert_eq!(freed(h, NEXT_SLEEPER).await?.1, None);
        assert_eq!(freed(h, FIRST_SLEEPER).await?, first_free);
        let (status, bytes) = command(
            &mut client,
            admitted,
            &mut next,
            COMMAND_TYPE_SERVER_MODAL_QUERY,
            QUERY_WITH_TERMINAL_REPLY,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(Reply::decode(&bytes).map_err(debug)?, Reply::None);

        assert_eq!(
            use_bed(&mut client, admitted, &mut next, head.clone()).await?,
            (CommandStatus::Accepted, UseDisposition::BedWoken)
        );
        assert_eq!(bed_revision(h).await?, revision + 1);
        assert_eq!(
            freed(h, NEXT_SLEEPER).await?.2.as_deref(),
            Some("owner_wake")
        );

        make_guest(h).await?;
        sqlx::query(
            "DELETE FROM game_house_bed_sleepers WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(NEXT_SLEEPER).as_slice())
        .execute(&h.pool)
        .await?;
        seed_sleeper(h, NEXT_SLEEPER).await?;
        let revision = bed_revision(h).await?;
        assert_eq!(
            use_bed(&mut client, admitted, &mut next, head).await?,
            (CommandStatus::Accepted, UseDisposition::BedOccupied)
        );
        assert_eq!(bed_revision(h).await?, revision);
        assert_eq!(freed(h, NEXT_SLEEPER).await?.1, None);
        let (status, bytes) = command(
            &mut client,
            admitted,
            &mut next,
            COMMAND_TYPE_SERVER_MODAL_QUERY,
            QUERY_WITH_TERMINAL_REPLY,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(Reply::decode(&bytes).map_err(debug)?, Reply::None);
        let receipts: Vec<String> = sqlx::query_scalar(
            "SELECT outcome FROM game_house_bed_use_receipts ORDER BY command_id",
        )
        .fetch_all(&h.pool)
        .await?;
        assert_eq!(receipts, ["woken", "woken", "occupied"]);
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let (_, result) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::fixture::join_two(serving, playing),
    )
    .await?;
    result
}

async fn bed_targets(client: &mut tokio::io::DuplexStream) -> TestResult<(UseTarget, UseTarget)> {
    let mut head = None;
    let mut foot = None;
    loop {
        let frame = read_frame(client).await?;
        let envelope = decode_wire_envelope(&frame).map_err(debug)?;
        match envelope.message_type() {
            MessageType::SnapshotChunk => {
                for domain in decode_snapshot_chunk(envelope.payload()).map_err(debug)?.1 {
                    if domain.domain_id != STATE_DOMAIN_WORLD_MAP_VIEW {
                        continue;
                    }
                    let map = decode_world_map_snapshot(domain.payload).map_err(debug)?;
                    for tile in &map.tiles {
                        let target = match (tile.position.x, tile.position.y, tile.position.floor) {
                            (32310, 32804, 7) => &mut head,
                            (32311, 32804, 7) => &mut foot,
                            _ => continue,
                        };
                        let MapOrigin::BaseOrdinal {
                            ordinal,
                            object_revision,
                        } = tile.items.get(1).ok_or("visible bed part")?.origin
                        else {
                            return Err("fixed bed part must have a base ordinal".into());
                        };
                        let key = placement_key(
                            TilePos {
                                x: u16::try_from(tile.position.x)?,
                                y: u16::try_from(tile.position.y)?,
                                floor: -7,
                            },
                            ordinal,
                        )
                        .ok_or("bed placement")?;
                        let mut placement = map.header.bundle_digest.to_vec();
                        placement.extend_from_slice(&key.to_be_bytes());
                        *target = Some(UseTarget::WorldObject(WorldObjectTarget {
                            placement,
                            expected_revision: object_revision,
                        }));
                    }
                }
            }
            MessageType::SnapshotCommit => {
                return Ok((
                    head.ok_or("bed head from House snapshot")?,
                    foot.ok_or("bed foot from House snapshot")?,
                ));
            }
            _ => {}
        }
    }
}

async fn use_bed(
    client: &mut tokio::io::DuplexStream,
    admitted: AdmittedSession,
    next: &mut u64,
    target: UseTarget,
) -> TestResult<(CommandStatus, UseDisposition)> {
    let payload = encode_item_use_intent(&UseIntent {
        target,
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let (status, bytes) =
        command(client, admitted, next, COMMAND_TYPE_USE_INTENT, &payload).await?;
    Ok((status, decode_use_result(&bytes).map_err(debug)?))
}

async fn seed_sleeper(h: &Harness, sleeper: u8) -> TestResult {
    sqlx::query(
        "INSERT INTO game_house_bed_sleepers \
         (character_id,world_id,house_key,bed_x,bed_y,bed_z,lease_generation,occupied_variant,slept_at) \
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,32310,32804,7,1,'male',\
         floor(extract(epoch FROM clock_timestamp())*1000)::bigint - 31000)",
    )
    .bind(id(sleeper).as_slice()).bind(id(42).as_slice()).bind(HOUSE)
    .execute(&h.pool).await?;
    Ok(())
}

async fn freed(h: &Harness, sleeper: u8) -> TestResult<(i64, Option<i64>, Option<String>)> {
    Ok(sqlx::query_as("SELECT slept_at,freed_at,freed_cause FROM game_house_bed_sleepers WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(sleeper).as_slice()).fetch_one(&h.pool).await?)
}

async fn bed_revision(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT revision::bigint FROM game_house_bed_revisions WHERE world_id=encode($1,'hex')::uuid AND house_key=$2")
        .bind(id(42).as_slice()).bind(HOUSE).fetch_one(&h.pool).await?)
}

async fn make_guest(h: &Harness) -> TestResult {
    // Import a settled change of landlord and guest grant after both sleepers
    // were freed. The original owner's runtime custody and transport stay live.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_house_properties SET owner_character_id=encode($1,'hex')::uuid,ownership_generation=ownership_generation+1,revision=revision+1,acl_revision=acl_revision+1 WHERE world_id=encode($2,'hex')::uuid AND house_key=$3")
        .bind(id(FIRST_SLEEPER).as_slice()).bind(id(42).as_slice()).bind(HOUSE).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_house_ownership SET owner_character_id=encode($1,'hex')::uuid,ownership_revision=ownership_revision+1 WHERE world_id=encode($2,'hex')::uuid AND house_key=$3")
        .bind(id(FIRST_SLEEPER).as_slice()).bind(id(42).as_slice()).bind(HOUSE).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_house_acl_entries(world_id,house_key,list_key,character_id) VALUES(encode($1,'hex')::uuid,$2,'guest',encode($3,'hex')::uuid)")
        .bind(id(42).as_slice()).bind(HOUSE).bind(id(41).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
