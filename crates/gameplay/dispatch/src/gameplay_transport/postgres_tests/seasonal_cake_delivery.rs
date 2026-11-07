//! W05d: actual Baker NPC command, carried substance cost and World progression.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::seasonal_cake::CakeWorldState;
use crate::foundation::ChannelContentPin;

#[test]
fn srv7_cake_baker_npc_consumes_only_remaining_goal_and_replays_on_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    super::kill_reward_e2e::scenario_thread::run("srv7-cake-delivery", || scenario(false))
}

#[test]
fn srv7_cake_delivery_root_consumes_carried_cost_atomically_on_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    super::kill_reward_e2e::scenario_thread::run("srv7-cake-root-delivery", || scenario(true))
}

fn scenario(direct: bool) -> TestResult {
    run(async |admin| {
        super::reset();
        let h = Harness::create_with_admission_seed(admin, "srv7_cake_delivery", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("native generation")?;
        let native = active.native_gameplay().ok_or("native")?;
        assert!(native.item_policy_for_source_id(12143).is_some(), "actual source substance policy");
        let channel_pin = ChannelContentPin::from_activation(world, pin.activation_sequence(),
            pin.identity().server_artifact_digest(), pin.identity().client_artifact_digest(), pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(), (2,0,7));
        let deps = Dependencies::with_content(channel_pin,
            crate::content::qualify_native_entry_room(world).map_err(debug)?, crate::spell::cast::v1_spell_book().map_err(debug)?)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        deps.initialize(&owner).await;
        deps.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4) ON CONFLICT DO NOTHING")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice())
            .bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;

        if direct {
            seed(&h).await?;
            let fence = owner.current_quest_fence(deps.admitted.game_session_id).await.map_err(|_| "current fence")?.ok_or("current fence")?.item_fence();
            let Some(permit) = owner.spell_lane_permit().await else { return Err("physical permit".into()); };
            let definition = native.item_policy_for_source_id(12143).ok_or("source substance")?.record().production_definition.clone();
            let request = crate::durability::seasonal_cake::CakeDeliveryRequest {
                command: crate::foundation::CommandRef::new(deps.admitted.game_session_id, crate::foundation::CommandId::new(901).map_err(debug)?),
                npc_key: "oteryn:npc.benny_the_baker".into(),
                substance: crate::durability::item_mint::TypedDefinitionRef { family: definition.family, production_key: definition.production_key, revision_ref: definition.revision_ref },
                content_digest: native.source_digest(), intent: b"source NPC Benny:deliver substance".to_vec(), now_seconds: 1_771_650_100,
            };
            let result = h.root.deliver_cake_substance(&authority, &h.node, &permit, fence, request.clone()).await?.ok_or("direct source delivery refused")?;
            assert_eq!(result.accepted, 200, "actual Root caller must consume the remaining source goal");
            assert_eq!(quantity(&h).await?, 50);
            let replay = h.root.deliver_cake_substance(&authority, &h.node, &permit, fence, request).await?.ok_or("direct replay")?;
            assert_eq!(replay, result);
            assert_eq!(quantity(&h).await?, 50);
            return Ok(());
        }
        let [npc] = super::classic_quests::install(&owner, &deps.admitted, ["oteryn:npc.benny_the_baker"]).await?;
        seed(&h).await?;
        let _clock = crate::gameplay_transport::seasonal_cake::test_clock::set(1_771_650_100);
        super::classic_quests::talk(&mut owner, &deps.admitted, npc, 900, "hi").await?;
        let lines = super::classic_quests::talk(&mut owner, &deps.admitted, npc, 901, "deliver substance").await?;
        assert!(lines.iter().any(|line| line.contains("You delivered 200 substance(s). (200/200 total delivered)")), "actual Baker reply: {lines:?}");
        assert_eq!(quantity(&h).await?, 50, "only the remaining World goal is consumed");
        let state: serde_json::Value = sqlx::query_scalar("SELECT state FROM game_seasonal_cake_worlds WHERE world_id=encode($1,'hex')::uuid AND event_start=1771650000")
            .bind(id(42).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(state["substances"], 200);
        assert_eq!(state["golems"], 200);
        assert_eq!(state["stage"], 2, "both source goals open the cake bridge");
        let replay = super::classic_quests::talk(&mut owner, &deps.admitted, npc, 901, "deliver substance").await?;
        assert_eq!(replay, lines);
        assert_eq!(quantity(&h).await?, 50);
        let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_seasonal_cake_deliveries").fetch_one(&h.pool).await?;
        assert_eq!(receipts, 1);
        let costs: i64 = sqlx::query_scalar("SELECT count(*) FROM game_srv7_item_costs WHERE cause_kind='cake_substance'").fetch_one(&h.pool).await?;
        assert_eq!(costs, 1);
        Ok(())
    })
}

async fn quantity(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE world_id=encode($1,'hex')::uuid AND definition_production_key='oteryn:item.tibia.i12143' AND lifecycle=1")
        .bind(id(42).as_slice()).fetch_one(&h.pool).await?)
}

async fn seed(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).execute(&mut *tx).await?;
    for (seed, key, quantity) in [(60,"oteryn:item.tibia.i2854",1), (62,"oteryn:item.tibia.i12143",100),
        (63,"oteryn:item.tibia.i12143",100), (64,"oteryn:item.tibia.i12143",50)] {
        sqlx::raw_sql(sqlx::AssertSqlSafe(crate::durability::item_fee_burn::test_support::item(seed,key,"definition-r1",quantity)))
            .execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(60).as_slice()).bind(id(42).as_slice()).bind(id(245).as_slice()).execute(&mut *tx).await?;
    for (ordinal, seed) in [62,63,64].into_iter().enumerate() {
        sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::bigint,encode($6,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(60).as_slice()).bind(id(seed).as_slice()).bind(id(42).as_slice())
            .bind(i64::try_from(ordinal+1)?).bind(id(246).as_slice()).execute(&mut *tx).await?;
    }
    let state = CakeWorldState { stage: 1, substances: 0, golems: 200, layer: 1, layer_progress: 0,
        window_start: None, window_end: None, bonus_from: None, bonus_until: None, revision: 1 };
    sqlx::query("INSERT INTO game_seasonal_cake_worlds(world_id,event_start,event_end,state) VALUES(encode($1,'hex')::uuid,1771650000,1772168400,$2)")
        .bind(id(42).as_slice()).bind(serde_json::to_value(state)?).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
