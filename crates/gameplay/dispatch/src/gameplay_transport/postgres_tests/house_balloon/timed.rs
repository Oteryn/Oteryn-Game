//! W18: the real House rotation owner resets the source's non-stopping timer.
use super::*;
use crate::durability::consume_transform::{ConsumeTransformRequest, HouseActionContext};
use crate::foundation::{CommandId, CommandRef};

const CYCLE: [(u32, i64, u32); 8] = [
    (35949, 200_000, 35950),
    (35950, 1_000, 35951),
    (35951, 200_000, 35952),
    (35952, 1_000, 35949),
    (35969, 200_000, 35970),
    (35970, 1_000, 35971),
    (35971, 200_000, 35972),
    (35972, 1_000, 35969),
];

pub(super) async fn scenario(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: AdmittedSession,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("House actor")?;
    let fence = admitted.item_fence.ok_or("House fence")?;
    let context = {
        let runtime = owner.runtime.lock().await;
        HouseActionContext::from_native_owner(&runtime, actor, admitted.game_session_id, fence)
            .map_err(debug)?
    };
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native timed furniture")?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&h.pool)
            .await?;
    // Historical, valid timed predecessors; production guards remain enabled for every call below.
    let old_deadline = now + 60_000;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (number, duration, _) in CYCLE {
        sqlx::query("INSERT INTO game_item_timed_definitions VALUES('Item',$1,'definition-r1',NULL,$2,true) ON CONFLICT DO NOTHING")
            .bind(format!("oteryn:item.tibia.i{number}")).bind(duration).execute(&mut *tx).await?;
    }
    sqlx::query(
        "INSERT INTO game_item_timed_states VALUES(encode($1,'hex')::uuid,NULL,200000,$2,1)",
    )
    .bind(id(71).as_slice())
    .bind(old_deadline)
    .execute(&mut *tx)
    .await?;
    for (offset, (number, duration, _)) in CYCLE.into_iter().enumerate() {
        let item = id(u8::try_from(150 + offset)?);
        let placement = id(u8::try_from(180 + offset)?);
        sqlx::query("INSERT INTO game_item_instances SELECT (jsonb_populate_record(NULL::game_item_instances,to_jsonb(i)||jsonb_build_object('item_instance_id',encode($1,'hex')::uuid,'definition_production_key',$2))).* FROM game_item_instances i WHERE i.item_instance_id=encode($3,'hex')::uuid")
            .bind(item.as_slice()).bind(format!("oteryn:item.tibia.i{number}")).bind(id(71).as_slice())
            .execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_house_interior_locations SELECT encode($1,'hex')::uuid,world_id,house_key,spatial_position,$2,encode($4,'hex')::uuid FROM game_item_house_interior_locations WHERE item_instance_id=encode($3,'hex')::uuid")
            .bind(item.as_slice()).bind(i64::try_from(offset + 2)?).bind(id(71).as_slice())
            .bind(placement.as_slice())
            .execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision,written_xact_id) SELECT encode($1,'hex')::uuid,world_id,house_key,reclaim_subject_character_id,encode($3,'hex')::uuid,provenance_revision,written_xact_id FROM game_item_house_reclaim_provenance WHERE item_instance_id=encode($2,'hex')::uuid")
            .bind(item.as_slice()).bind(id(71).as_slice()).bind(placement.as_slice()).execute(&mut *tx).await?;
        sqlx::query(
            "INSERT INTO game_item_timed_states VALUES(encode($1,'hex')::uuid,NULL,$2,$3,1)",
        )
        .bind(item.as_slice())
        .bind(duration)
        .bind(now - 100)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    let request = |command, expected| -> TestResult<ConsumeTransformRequest> {
        use oteryn_protocol_oteryn::house_rotation as wire;
        let mut intent = wire::COMMAND.to_be_bytes().to_vec();
        intent.extend(
            wire::Request::Rotate(wire::Target::Item {
                item: id(71),
                expected_revision: expected,
            })
            .encode()
            .ok_or("native rotation bytes")?,
        );
        Ok(ConsumeTransformRequest {
            command: CommandRef::new(
                admitted.game_session_id,
                CommandId::new(command).map_err(debug)?,
            ),
            source: id(71),
            target: None,
            intent,
        })
    };
    let first = owner
        .root
        .consume_transform_in_house(
            owner.holder,
            context.clone().with_rotation(1),
            request(1, 1)?,
            native,
        )
        .await?;
    assert_eq!(
        first.disposition,
        UseDisposition::Committed,
        "native rotation must admit the timed aqueduct"
    );
    assert!(first.fresh);
    let first_state = timed(h, id(71)).await?;
    let first_deadline = first_state["deadline_at"]
        .as_i64()
        .ok_or("armed deadline")?;
    assert!(
        first_deadline > old_deadline + 100_000,
        "Item::setID resets the non-stopping target's full duration"
    );
    assert_eq!(first_state["remaining_ms"], 200_000);
    assert_eq!(first_state["state_revision"], 2);
    assert_eq!(
        super::scenario::item(h).await?["definition_production_key"],
        "oteryn:item.tibia.i35969"
    );
    let second = owner
        .root
        .consume_transform_in_house(
            owner.holder,
            context.clone().with_rotation(2),
            request(2, 2)?,
            native,
        )
        .await?;
    assert_eq!(second.disposition, UseDisposition::Committed);
    assert!(second.fresh);
    let second_state = timed(h, id(71)).await?;
    assert_eq!(second_state["state_revision"], 3);
    assert!(
        second_state["deadline_at"]
            .as_i64()
            .ok_or("second deadline")?
            >= first_deadline
    );
    let replay = owner
        .root
        .consume_transform_in_house(
            owner.holder,
            context.clone().with_rotation(1),
            request(1, 1)?,
            native,
        )
        .await?;
    assert_eq!(replay.disposition, UseDisposition::Committed);
    assert!(!replay.fresh);
    assert_eq!(
        timed(h, id(71)).await?,
        second_state,
        "replay does not restart the timer"
    );
    let mut stale_fence = fence;
    stale_fence.connection_generation =
        crate::foundation::ConnectionGeneration::new(fence.connection_generation.get() + 1)
            .map_err(debug)?;
    let stale_context = {
        let runtime = owner.runtime.lock().await;
        HouseActionContext::from_native_owner(
            &runtime,
            actor,
            admitted.game_session_id,
            stale_fence,
        )
        .map_err(debug)?
    };
    let stale = owner
        .root
        .consume_transform_in_house(
            owner.holder,
            stale_context.with_rotation(3),
            request(3, 3)?,
            native,
        )
        .await?;
    assert_eq!(stale.disposition, UseDisposition::StaleState);
    assert_eq!(
        timed(h, id(71)).await?,
        second_state,
        "a retired connection cannot reset an aqueduct timer"
    );
    // A House with no observers accumulated these due rows. Its first map read must
    // advance them before publishing any of their old definitions.
    let mut map = crate::gameplay_transport::world_map::SessionMapView::default();
    let mut items =
        crate::gameplay_transport::item_view::SessionItemView::resume(Default::default());
    owner
        .observe_house_item_map(admitted, &mut map, &mut items)
        .await
        .ok_or("House observation")?
        .map_err(debug)?;
    for (offset, (number, _, _)) in CYCLE.into_iter().enumerate() {
        let item = id(u8::try_from(150 + offset)?);
        let observed: i64 = sqlx::query_scalar("SELECT observed_ms FROM game_house_timed_decoration_expiries WHERE item_instance_id=encode($1,'hex')::uuid ORDER BY observed_ms DESC LIMIT 1")
            .bind(item.as_slice()).fetch_one(&h.pool).await?;
        // Independent stepwise oracle: setup/SQL latency may cross the short 1s
        // intermediate form, so compare at the writer's captured wall clock.
        let mut expected = number;
        let mut deadline = now - 100;
        while deadline <= observed {
            expected = CYCLE
                .iter()
                .find(|(id, _, _)| *id == expected)
                .ok_or("cycle source")?
                .2;
            deadline += CYCLE
                .iter()
                .find(|(id, _, _)| *id == expected)
                .ok_or("cycle target")?
                .1;
        }
        let key: String = sqlx::query_scalar("SELECT definition_production_key FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(item.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            key,
            format!("oteryn:item.tibia.i{expected}"),
            "every source cycle stage expires through the same owner"
        );
        assert_eq!(
            timed(h, item).await?["deadline_at"]
                .as_i64()
                .ok_or("successor deadline")?,
            deadline
        );
    }
    eprintln!(
        "W18 POSTGRES_CASE_COMPLETED timed aqueduct rotations reset, replay preserves, first map advances full cycle"
    );
    Ok(())
}

async fn timed(h: &Harness, item: [u8; 16]) -> TestResult<Value> {
    Ok(sqlx::query_scalar("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).fetch_one(&h.pool).await?)
}

pub(super) async fn near_expiry_scenario(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: AdmittedSession,
) -> TestResult {
    use oteryn_protocol_oteryn::house_rotation as wire;
    let actor = admitted.runtime_actor.ok_or("House actor")?;
    let fence = admitted.item_fence.ok_or("House fence")?;
    let context = {
        let runtime = owner.runtime.lock().await;
        HouseActionContext::from_native_owner(&runtime, actor, admitted.game_session_id, fence)
            .map_err(debug)?
    };
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native timed furniture")?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (number, duration, _) in CYCLE {
        sqlx::query("INSERT INTO game_item_timed_definitions VALUES('Item',$1,'definition-r1',NULL,$2,true) ON CONFLICT DO NOTHING")
            .bind(format!("oteryn:item.tibia.i{number}")).bind(duration).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_item_timed_states SELECT encode($1,'hex')::uuid,NULL,200000,floor(extract(epoch FROM clock_timestamp())*1000)::bigint-1,1")
        .bind(id(71).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let mut intent = wire::COMMAND.to_be_bytes().to_vec();
    intent.extend(
        wire::Request::Rotate(wire::Target::Item {
            item: id(71),
            expected_revision: 1,
        })
        .encode()
        .ok_or("rotation bytes")?,
    );
    let request = ConsumeTransformRequest {
        command: CommandRef::new(admitted.game_session_id, CommandId::new(1).map_err(debug)?),
        source: id(71),
        target: None,
        intent,
    };
    let result = owner
        .root
        .consume_transform_in_house(
            owner.holder,
            context.clone().with_rotation(1),
            request.clone(),
            native,
        )
        .await?;
    assert_eq!(result.disposition, UseDisposition::Committed);
    assert!(result.fresh);
    assert_eq!(
        super::scenario::item(h).await?["definition_production_key"],
        "oteryn:item.tibia.i35970",
        "Game::transformItem chooses requested form's decayTo when remaining duration is at most1ms"
    );
    let after = timed(h, id(71)).await?;
    assert_eq!(after["remaining_ms"], 1000);
    let exact:bool=sqlx::query_scalar("SELECT deadline_after=committed_at+1000 FROM game_item_timed_state_writes WHERE item_instance_id=encode($1,'hex')::uuid AND expected_revision=1")
        .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
    assert!(exact);
    let replay = owner
        .root
        .consume_transform_in_house(owner.holder, context.with_rotation(1), request, native)
        .await?;
    assert!(!replay.fresh);
    assert_eq!(timed(h, id(71)).await?, after);
    eprintln!(
        "W18 POSTGRES_CASE_COMPLETED near-expiry rotation selects counterpart decay stage and duplicate preserves its1s deadline"
    );
    Ok(())
}
pub(super) async fn wrap_scenario(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: AdmittedSession,
) -> TestResult {
    use crate::durability::house_wrap::HouseWrapCommand;
    use oteryn_protocol_oteryn::house_wrap as wire;
    let actor = admitted.runtime_actor.ok_or("House actor")?;
    let fence = admitted.item_fence.ok_or("House fence")?;
    let context = {
        let runtime = owner.runtime.lock().await;
        HouseActionContext::from_native_owner(&runtime, actor, admitted.game_session_id, fence)
            .map_err(debug)?
    };
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (number, duration, _) in CYCLE {
        sqlx::query("INSERT INTO game_item_timed_definitions VALUES('Item',$1,'definition-r1',NULL,$2,true) ON CONFLICT DO NOTHING")
            .bind(format!("oteryn:item.tibia.i{number}")).bind(duration).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_item_timed_states SELECT encode($1,'hex')::uuid,NULL,200000,floor(extract(epoch FROM clock_timestamp())*1000)::bigint+60000,1")
        .bind(id(71).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let acl: String = sqlx::query_scalar("SELECT acl_revision::text FROM game_house_properties WHERE world_id=encode($1,'hex')::uuid AND house_key=$2")
        .bind(context.house().world_id().as_bytes().as_slice()).bind(context.house().house_key())
        .fetch_one(&h.pool).await?;
    let request = wire::Request {
        action: wire::Action::Wrap,
        item: id(71),
        expected_revision: 1,
        acl_revision: acl.parse()?,
        position: POSITION,
    };
    let command = HouseWrapCommand {
        house: context.house().clone(),
        command: 10,
        request,
    };
    let result = owner
        .root
        .wrap_house_furniture(
            owner.holder,
            fence,
            command,
            |key| {
                (key == "oteryn:item.tibia.i35949").then(|| "oteryn:item.tibia.i23398".to_owned())
            },
            |key| (key == "oteryn:item.tibia.i2854").then_some(8),
        )
        .await?;
    assert_eq!(
        result,
        wire::Reply::Committed(2),
        "active source timer must not prevent donor wrapping"
    );
    let packed = timed(h, id(71)).await?;
    assert!(packed["remaining_ms"].is_null());
    assert!(packed["deadline_at"].is_null());
    assert_eq!(packed["state_revision"], 2);
    let target = owner
        .root
        .house_furniture_kit_target(owner.character, id(71))
        .await?
        .ok_or("latest proven wrap target")?;
    assert_eq!(
        target.production_key, "oteryn:item.tibia.i35949",
        "the production unwrap reader resolves a placed kit without purchased origin"
    );
    let target_key = target.production_key;
    let catalogue = owner
        .root
        .house_furniture_catalogue(owner.holder, fence, context.house().clone())
        .await?
        .map_err(debug)?;
    assert!(
        catalogue.items.iter().any(|item| item.item == id(71)
            && item
                .target
                .as_ref()
                .is_some_and(|target| target.production_key == target_key)),
        "the native selection panel advertises the same proven unwrap target"
    );
    let request = wire::Request {
        action: wire::Action::Unwrap,
        expected_revision: 2,
        ..request
    };
    let command = HouseWrapCommand {
        house: context.house().clone(),
        command: 11,
        request,
    };
    let result = owner
        .root
        .wrap_house_furniture(
            owner.holder,
            fence,
            command,
            move |key| (key == "oteryn:item.tibia.i23398").then_some(target_key),
            |key| (key == "oteryn:item.tibia.i2854").then_some(8),
        )
        .await?;
    assert_eq!(result, wire::Reply::Committed(3));
    let unwrapped = timed(h, id(71)).await?;
    assert_eq!(unwrapped["remaining_ms"], 200000);
    assert!(unwrapped["deadline_at"].as_i64().is_some());
    assert_eq!(unwrapped["state_revision"], 3);
    let exact:bool=sqlx::query_scalar("SELECT deadline_after=committed_at+200000 FROM game_item_timed_state_writes WHERE item_instance_id=encode($1,'hex')::uuid AND expected_revision=2")
        .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
    assert!(
        exact,
        "unwrap starts a full donor timer instead of restoring the packed budget"
    );
    let command = HouseWrapCommand {
        house: context.house().clone(),
        command: 11,
        request,
    };
    let replay = owner
        .root
        .wrap_house_furniture(
            owner.holder,
            fence,
            command,
            |_| None,
            |key| (key == "oteryn:item.tibia.i2854").then_some(8),
        )
        .await?;
    assert_eq!(replay, wire::Reply::Committed(3));
    assert_eq!(
        timed(h, id(71)).await?,
        unwrapped,
        "duplicate unwrap cannot extend its deadline"
    );
    eprintln!(
        "W18 POSTGRES_CASE_COMPLETED wrap cancels active aqueduct timer, unwrap resets full duration, replay preserves"
    );
    Ok(())
}
