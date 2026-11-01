//! M08: the squeezing gear of girlpower's jam (squeezing_gear_of_girlpower.lua:16-17,34-40;
//! items.xml:25842-25847: duration 60, decayTo 9596) is a durable database-clock deadline.
use super::fixture::{debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

const GEAR: &str = "oteryn:item.tibia.i9596";

#[test]
fn m08_gear_jam_is_durable_fenced_and_idempotent() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "tool jam panic")??;
    eprintln!("M08_TOOL_JAM_PG_CASE_EXECUTED");
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let harness = super::fixture::Harness::create_with_admission_seed(admin, "tool_jam", true, false).await?;
        super::reset();
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness.root.open_character_authority(&seal).await.map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) = crate::content::native_gameplay::test_support::activated_with_loot_tables(
            deps.admitted.world_id,
            None,
        );
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        assert!(native.item_policy(GEAR, "definition-r1").is_some(), "gear profile");
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        deps.initialize(&owner).await;
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(harness.root.clone())
            .current_session_at(deps.admitted.game_session_id)
            .await?
            .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
        super::item_look::seed_main_backpack(&harness, &[(GEAR, 1)]).await?;
        let tool: [u8; 16] = sqlx::query_scalar::<_, Vec<u8>>("SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE definition_production_key=$1")
            .bind(GEAR).fetch_one(&harness.pool).await?.try_into().map_err(|_| "tool id")?;
        let digest = native.source_digest();
        let intent = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(3).ok_or("source")?),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget { placement: vec![1; 40], expected_revision: 0 })),
            use_with_position: None,
        })
        .map_err(debug)?;
        let permit = owner.spell_lane_permit().await.ok_or("permit")?;
        let command = |n: u64| -> TestResult<crate::foundation::CommandRef> {
            Ok(crate::foundation::CommandRef::new(
                deps.admitted.game_session_id,
                crate::foundation::CommandId::new(n).map_err(debug)?,
            ))
        };
        let jam = |fence, n: u64, intent: Vec<u8>, key: String| {
            let permit = &permit;
            let authority = &authority;
            let owner = &owner;
            let harness = &harness;
            async move {
                harness
                    .root
                    .jam_tool(permit, authority, owner.holder, fence, command(n)?, intent, tool, key, "definition-r1".into(), 60_000, digest)
                    .await
                    .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { format!("{e:?}").into() })
            }
        };
        let active_now = || async {
            harness.root.tool_jam_active(&authority, *fence.character_id.as_bytes(), tool).await.map_err(debug)
        };
        assert!(!active_now().await?, "not jammed before the roll");
        assert_eq!(jam(fence, 1, intent.clone(), GEAR.into()).await?, UseDisposition::Committed);
        assert!(active_now().await?, "jammed by the database clock");
        // The deadline is 60 s ahead of the database clock, not an in-memory timer.
        let remaining: i64 = sqlx::query_scalar("SELECT jammed_until_unix_ms-floor(extract(epoch FROM clock_timestamp())*1000)::bigint FROM game_tool_jams").fetch_one(&harness.pool).await?;
        assert!((55_000..=60_000).contains(&remaining), "remaining {remaining}");
        // Exact replay is idempotent; another intent under the same CommandRef is refused.
        assert_eq!(jam(fence, 1, intent.clone(), GEAR.into()).await?, UseDisposition::Committed);
        let mut changed = intent.clone();
        changed.push(0);
        assert_eq!(jam(fence, 1, changed, GEAR.into()).await?, UseDisposition::Rejected);
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_tool_jam_receipts").fetch_one(&harness.pool).await?, 1);
        // A stale lease and a wrong tool identity cannot start a jam.
        let mut stale = fence;
        stale.character_lease_generation += 1;
        assert_eq!(jam(stale, 2, intent.clone(), GEAR.into()).await?, UseDisposition::Rejected);
        assert_eq!(jam(fence, 3, intent.clone(), "oteryn:item.tibia.i3453".into()).await?, UseDisposition::StaleState);
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_tool_jam_receipts").fetch_one(&harness.pool).await?, 1);
        // The deadline passes by the database clock: the gear works again (decayTo 9596).
        sqlx::query("UPDATE game_tool_jams SET jammed_until_unix_ms=floor(extract(epoch FROM clock_timestamp())*1000)::bigint-1").execute(&harness.pool).await?;
        assert!(!active_now().await?, "jam ended");
        Ok(())
    })
}
