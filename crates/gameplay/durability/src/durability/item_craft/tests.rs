use super::super::item_transfer::{ItemDefinitionFacts, ItemWeightLookup, ItemWeightRefused};
use super::*;
use crate::bestiary_postgres_harness::{self as fixture, Harness, TestResult, debug, id};

struct Content<'a>(&'a NativeGameplayState);
impl ItemWeightLookup for Content<'_> {
    fn generation_digest(&self) -> [u8; 32] {
        self.0.source_digest()
    }
    fn weight_of(&self, definition: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        self.loop_facts(definition)
            .and_then(|facts| facts.weight)
            .ok_or(ItemWeightRefused::Unknown)
    }
}
impl ItemLoopContent for Content<'_> {
    fn loop_content_digest(&self) -> [u8; 32] {
        self.0.source_digest()
    }
    fn loop_facts(&self, definition: &TypedDefinitionRef) -> Option<ItemDefinitionFacts> {
        oteryn_gameplay_items::item_ref_admission::generation_item_facts(self.0, definition).ok()
    }
}
fn definition(item: u32) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: format!("oteryn:item.tibia.i{item}"),
        revision_ref: "definition-r1".into(),
    }
}

// Only inventory DATA is seeded; sessions, guards, node and recovery come from the native harness.
async fn seed(h: &Harness, filler: u32, filler_count: u8) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (seed, item, quantity) in [(70, 2854, 1), (71, 3603, 1), (72, 2874, 1)]
        .into_iter()
        .chain((73..73 + filler_count).map(|seed| (seed, filler, 1)))
    {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',$4,1,encode($1,'hex')::uuid)")
            .bind(id(seed).as_slice()).bind(id(fixture::WORLD).as_slice()).bind(definition(item).production_key).bind(quantity).execute(&mut *tx).await?;
        if seed != 70 {
            sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,encode($1,'hex')::uuid)")
                .bind(id(seed).as_slice()).bind(id(fixture::WORLD).as_slice()).bind(id(fixture::CHARACTER).as_slice()).bind(id(70).as_slice()).bind(i64::from(seed-70)).execute(&mut *tx).await?;
        }
    }
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($2,'hex')::uuid)")
        .bind(id(fixture::CHARACTER).as_slice()).bind(id(70).as_slice()).bind(id(fixture::WORLD).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_fluid_states(item_instance_id,world_id,fluid_subtype,last_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,encode($1,'hex')::uuid)").bind(id(72).as_slice()).bind(id(fixture::WORLD).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn craft(h: &Harness, native: &NativeGameplayState) -> TestResult<Option<UseDisposition>> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let fence = fixture::item_fence()?;
    let command = CommandRef::new(
        fence.game_session_id,
        crate::foundation::CommandId::new(800).map_err(debug)?,
    );
    Ok(h.root
        .craft_item_owned::<_, ()>(
            &authority,
            &h.node,
            fence,
            ItemCraftRequest {
                command,
                source: id(71),
                tool: None,
                ingredient: Some(id(72)),
                station: Some(definition(2874)),
                intent: vec![1],
                gathering: None,
                enchanting: None,
            },
            native,
            &Content(native),
            false,
            None,
        )
        .await?)
}

fn run_case(
    tag: &'static str,
    case: impl AsyncFnOnce(&Harness, &NativeGameplayState) -> TestResult,
) -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    fixture::runtime()?.block_on(async {
        let h=Harness::create(admin,tag,true).await?;
        let world=crate::foundation::WorldId::decode(&id(fixture::WORLD)).map_err(debug)?;
        let (controller,pin)=oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(world,None);
        let native=controller.active().and_then(|active|active.native_gameplay()).ok_or("native content")?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(fixture::WORLD).as_slice()).bind(id(fixture::CHANNEL).as_slice()).execute(&h.pool).await?;
        assert_eq!(native.source_digest(),pin.identity().server_artifact_digest());
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(fixture::WORLD).as_slice()).bind(id(fixture::CHANNEL).as_slice()).bind(native.source_digest().as_slice())
            .bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let result=case(&h,native).await;
        h.cleanup().await?;
        result
    })
}

#[test]
fn carried_cooking_consumes_both_operands_in_a_full_backpack() -> TestResult {
    std::thread::Builder::new().stack_size(64<<20).spawn(|| {
        run_case("craft_full",async |h,native| {
            seed(h,3031,18).await?;
            assert_eq!(craft(h,native).await?,Some(UseDisposition::Committed));
            let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries").fetch_one(&h.pool).await?;
            assert_eq!(count,20);
            let retired:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE item_instance_id IN(encode($1,'hex')::uuid,encode($2,'hex')::uuid) AND lifecycle=2").bind(id(71).as_slice()).bind(id(72).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(retired,2);
            assert_eq!(craft(h,native).await?,Some(UseDisposition::Committed));
            assert_eq!(h.count("game_item_craft_records").await?,1);
            Ok(())
        }).map_err(|e|e.to_string())
    })?.join().map_err(|_|"craft test thread")?.map_err(Into::into)
}

#[test]
fn carried_cooking_subtracts_ingredient_weight_before_capacity() -> TestResult {
    std::thread::Builder::new().stack_size(64<<20).spawn(|| {
        run_case("craft_weight",async |h,native| {
            seed(h,3605,4).await?;
            let capacity=oteryn_gameplay_items::item_capacity::project_global_capacity_centi_oz(50,"none").map_err(debug)?;
            let filler=capacity-2480; // 2080 carried base; net conversion adds 400 centi-oz.
            let wheat=filler/1250;
            let coins=filler%1250/10;
            assert!((2..=200).contains(&wheat) && (2..=200).contains(&coins));
            let mut tx=h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
            for (seed, key, quantity) in [(73,3605,(wheat-1).min(100)),(74,3605,wheat-(wheat-1).min(100)),(75,3031,(coins-1).min(100)),(76,3031,coins-(coins-1).min(100))] {
                sqlx::query("UPDATE game_item_instances SET definition_production_key=$2,quantity=$3 WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(seed).as_slice()).bind(definition(key).production_key).bind(i64::try_from(quantity)?).execute(&mut *tx).await?;
            }
            tx.commit().await?;
            let mut tx=h.pool.begin().await?;
            let before=carried_weight(&mut tx,fixture::item_fence()?.character_id,id(fixture::WORLD),&Content(native)).await.map_err(debug)?;
            tx.rollback().await?;
            assert!(before+400<=capacity && before+580>capacity,"weight boundary {before}/{capacity}");
            assert_eq!(craft(h,native).await?,Some(UseDisposition::Committed));
            Ok(())
        }).map_err(|e|e.to_string())
    })?.join().map_err(|_|"craft weight thread")?.map_err(Into::into)
}

#[test]
fn carried_cooking_subtracts_both_retired_tree_nodes() -> TestResult {
    std::thread::Builder::new().stack_size(64<<20).spawn(|| {
        run_case("craft_tree",async |h,native| {
            seed(h,2854,18).await?;
            let mut tx=h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
            sqlx::query("CREATE TEMP TABLE craft_nodes ON COMMIT DROP AS SELECT n,game_character_uuid_v7() item FROM generate_series(1,480) n").execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT item,encode($1,'hex')::uuid,'Item',CASE WHEN n<=6 THEN 'oteryn:item.tibia.i2854' ELSE 'oteryn:item.tibia.i3031' END,'definition-r1',1,1,item FROM craft_nodes").bind(id(fixture::WORLD).as_slice()).execute(&mut *tx).await?;
            let parents:Vec<Vec<u8>>=(73..91).map(|seed|id(seed).to_vec()).collect();
            sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT item,encode($1,'hex')::uuid,CASE WHEN n<=360 THEN encode(($2::bytea[])[(n-1)/20+1],'hex')::uuid ELSE (SELECT item FROM craft_nodes p WHERE p.n=(c.n-361)/20+1) END,(n-1)%20+1,item FROM craft_nodes c").bind(id(fixture::WORLD).as_slice()).bind(parents).execute(&mut *tx).await?;
            tx.commit().await?;
            assert_eq!(craft(h,native).await?,Some(UseDisposition::Committed));
            assert_eq!(h.count("game_item_container_entries").await?,20);
            assert_eq!(h.count("game_item_nested_entries").await?,480);
            Ok(())
        }).map_err(|e|e.to_string())
    })?.join().map_err(|_|"craft tree thread")?.map_err(Into::into)
}
