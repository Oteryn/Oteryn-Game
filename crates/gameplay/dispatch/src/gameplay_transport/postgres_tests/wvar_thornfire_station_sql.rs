//! Narrow real-PG proof for the existing station guard chain's Thornfire extension.
//! The sibling native station case proves actual Item debit/output/actor publication.
use super::TestResult;
use super::fixture::{Harness, id, run};
use oteryn_gameplay_world_change::variants;
use serde_json::{Value, json};

#[test]
fn wvar_thornfire_station_sql_exact_sources_and_save_boundaries() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Thornfire SQL proof thread")??;
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        let started = std::time::Instant::now();
        let h = Harness::create_with_admission_seed(admin, "thornfire_sql", false, false).await?;
        let definition = variants::definitions()?
            .iter()
            .find(|d| d.key == "thornfire")
            .ok_or("Thornfire recipe")?;
        let flames = [
            232, 5061, 5062, 5063, 5064, 5065, 5066, 5067, 6288, 6289, 7803, 19096, 32021,
        ];
        let mut witnesses = 0;
        let mut flame = None;
        let mut bog = None;
        for (variant, recipe) in definition.variants.iter().enumerate() {
            for tile in &recipe.tiles {
                for (ordinal, entry) in tile.entries.iter().enumerate() {
                    if entry.0 != 12721 && !flames.contains(&entry.0) {
                        continue;
                    }
                    let (x, y, z) = tile.position;
                    let key = (u64::from(x) << 32)
                        | (u64::from(y) << 16)
                        | (u64::try_from(z)? << 8)
                        | u64::try_from(ordinal)?;
                    let mut placement = vec![7; 32];
                    placement.extend_from_slice(&key.to_be_bytes());
                    let valid: bool =
                        sqlx::query_scalar("SELECT game_thornfire_source_station($1,$2,$3)")
                            .bind(i32::try_from(variant)?)
                            .bind(&placement)
                            .bind(i64::from(entry.0))
                            .fetch_one(&h.pool)
                            .await?;
                    assert!(valid, "authored variant{variant} source{key:016x}");
                    let mut wrong_ordinal = placement.clone();
                    wrong_ordinal[39] = 63;
                    let invalid: bool =
                        sqlx::query_scalar("SELECT game_thornfire_source_station($1,$2,$3)")
                            .bind(i32::try_from(variant)?)
                            .bind(&wrong_ordinal)
                            .bind(i64::from(entry.0))
                            .fetch_one(&h.pool)
                            .await?;
                    assert!(!invalid, "ordinal substitution");
                    witnesses += 1;
                    if variant == 0 && entry.0 == 12721 {
                        bog = Some((placement.clone(), entry.0));
                    }
                    if variant == 0 && flames.contains(&entry.0) {
                        flame = Some((placement, entry.0));
                    }
                }
            }
        }
        assert_eq!(witnesses, 722);
        let (flame_placement, appearance) = flame.ok_or("source flame")?;
        let (bog_placement, _) = bog.ok_or("source bog")?;
        // Real immutable World/reset rows; composites below isolate each guard input.
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,991,$2,$3,$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(1000),'1970-01')")
            .bind(id(42).as_slice()).bind([7u8;32].as_slice())
            .bind(format!("sha256:{}","07".repeat(32))).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,991) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=excluded.reset_epoch")
            .bind(id(42).as_slice()).execute(&h.pool).await?;
        let mut selected = vec![Value::Null; variants::CAPACITY];
        selected[15] = json!(0);
        sqlx::query("INSERT INTO game_world_change_days(world_id,reset_epoch,decision) VALUES(encode($1,'hex')::uuid,991,$2)")
            .bind(id(42).as_slice()).bind(json!({"variants":[selected,variants::recipe_digest()]}))
            .execute(&h.pool).await?;
        let world: String = sqlx::query_scalar("SELECT encode($1,'hex')::uuid::text")
            .bind(id(42).as_slice())
            .fetch_one(&h.pool)
            .await?;
        let session = "00000000-0000-7000-8000-000000000051";
        let character = "00000000-0000-7000-8000-000000000041";
        let bucket = "00000000-0000-7000-8000-000000000072";
        let channel = "00000000-0000-7000-8000-000000000043";
        let mut c = json!({"recipe":"oteryn:world-change.contribution","consume_source":true,
            "world_id":world,"character_id":character,"game_session_id":session,"command_id":1,
            "intent":"\\x01","source_item_instance_id":bucket,"occurred_at":1000001,
            "before_item":{"definition_production_key":"oteryn:item.tibia.i12722"},
            "outputs":[{"key":"oteryn:item.tibia.i12819","count":1}]});
        let mut r = json!({"world_id":world,"character_id":character,"game_session_id":session,
            "command_id":1,"intent":"\\x01","tool_item_instance_id":bucket,
            "channel_id":channel,"placement":hex_bytea(&flame_placement),"reset_epoch":991,
            "before_item_id":appearance,"transformed_item_id":0});
        assert!(pair(&h, &c, &r).await?);
        let stale_xid: bool = sqlx::query_scalar("SELECT game_thornfire_station_pair(jsonb_populate_record(NULL::game_item_craft_records,$1||jsonb_build_object('created_xact_id',(pg_current_xact_id()::text::numeric-1)::text)),jsonb_populate_record(NULL::game_terrain_tool_receipts,$2||jsonb_build_object('created_xact_id',pg_current_xact_id()::text)))")
            .bind(&c).bind(&r).fetch_one(&h.pool).await?;
        assert!(!stale_xid, "historical craft cannot prove same-XID debit");
        let mut wrong_time = c.clone();
        wrong_time["occurred_at"] = json!(1000000_u64 - 1);
        assert!(!pair(&h, &wrong_time, &r).await?);
        wrong_time["occurred_at"] = json!(999999999999999_i64);
        assert!(!pair(&h, &wrong_time, &r).await?);
        let mut bad = r.clone();
        bad["reset_epoch"] = json!(990);
        assert!(!pair(&h, &c, &bad).await?);
        bad = r.clone();
        bad["placement"] = json!(hex_bytea(&bog_placement));
        assert!(!pair(&h, &c, &bad).await?);
        bad = r.clone();
        bad["command_id"] = json!(2);
        assert!(!pair(&h, &c, &bad).await?);
        let mut t = json!({"world_id":world,"channel_id":channel,"placement":hex_bytea(&flame_placement),
            "reset_epoch":991,"state_item_id":0,"decay_step":0,"map_revision":format!("sha256:{}","07".repeat(32)),
            "decay_states":[appearance],"decay_delays":[28800000],"decay_at_unix_ms":29800000});
        assert!(decay(&h, &c, &r, &t).await?);
        let mut wrong_prefix = t.clone();
        wrong_prefix["map_revision"] = json!(format!("sha256:{}", "08".repeat(32)));
        assert!(!decay(&h, &c, &r, &wrong_prefix).await?);
        let mut wrong = t.clone();
        wrong["decay_at_unix_ms"] = json!(29800001);
        assert!(!decay(&h, &c, &r, &wrong).await?);
        c["occurred_at"] = json!(29800000);
        assert!(!decay(&h, &c, &r, &t).await?);
        t["decay_at_unix_ms"] = json!(58600000);
        assert!(decay(&h, &c, &r, &t).await?);
        c["occurred_at"] = json!(58600000);
        assert!(!decay(&h, &c, &r, &t).await?);
        t["decay_states"] = json!([]);
        t["decay_delays"] = json!([]);
        t["decay_at_unix_ms"] = Value::Null;
        assert!(decay(&h, &c, &r, &t).await?);
        c["recipe"] = json!("oteryn:world-change.refill");
        c["before_item"] = json!({"definition_production_key":"oteryn:item.tibia.i12819"});
        c["outputs"] = json!([{"key":"oteryn:item.tibia.i12722","count":1,
            "content_digest":format!("sha256:{}","07".repeat(32)),
            "decay":{"duration_millis":3600000,"target":{"family":"Item",
                "key":"oteryn:item.tibia.i12819","revision":"definition-r1"}}}]);
        r["placement"] = json!(hex_bytea(&bog_placement));
        r["before_item_id"] = json!(12721);
        r["transformed_item_id"] = json!(12721);
        assert!(pair(&h, &c, &r).await?);
        c["outputs"][0]["decay"]["duration_millis"] = json!(3600001);
        assert!(!pair(&h, &c, &r).await?);
        eprintln!(
            "WVAR_THORNFIRE_STATION_SQL_POSTGRES_EXECUTED exact722sources,ordinal,currentEpoch,sameCommand,save8h16h,refill1h; duration={:?}",
            started.elapsed()
        );
        h.cleanup().await?;
        Ok(())
    })
}
fn hex_bytea(bytes: &[u8]) -> String {
    let mut value = String::from("\\x");
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(value, "{byte:02x}");
    }
    value
}
async fn pair(h: &Harness, c: &Value, r: &Value) -> TestResult<bool> {
    Ok(sqlx::query_scalar("SELECT game_thornfire_station_pair(jsonb_populate_record(NULL::game_item_craft_records,$1||jsonb_build_object('created_xact_id',pg_current_xact_id()::text)),jsonb_populate_record(NULL::game_terrain_tool_receipts,$2||jsonb_build_object('created_xact_id',pg_current_xact_id()::text)))")
        .bind(c).bind(r).fetch_one(&h.pool).await?)
}
async fn decay(h: &Harness, c: &Value, r: &Value, t: &Value) -> TestResult<bool> {
    Ok(sqlx::query_scalar("SELECT game_thornfire_station_decay(jsonb_populate_record(NULL::game_item_craft_records,$1),jsonb_populate_record(NULL::game_terrain_tool_receipts,$2),jsonb_populate_record(NULL::game_terrain_tool_transforms,$3))")
        .bind(c).bind(r).bind(t).fetch_one(&h.pool).await?)
}
