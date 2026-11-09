//! M49 uses real MINT/TRANSFER/move owners with every native guard enabled.
use crate::corpse_transfer_postgres_cases::{equip_backpack, typed};
use crate::durability::character_equipment::{
    assert_equipment_authority_in_transaction, initialize_equipment_in_transaction,
};
use crate::durability::item_loop_move::{
    AltarArrivalTarget, ItemLoopContent, LoopDestination, LoopGround, LoopMoveRequest,
};
use crate::durability::item_mint::{
    CORPSE_MATERIALIZATION_PURPOSE_KEY, GroundPlacement, ItemMintCause, ItemMintOutcome,
    ItemMintRequest, TypedDefinitionRef,
};
use crate::durability::item_transfer::{
    ItemDefinitionFacts, ItemStackClass, ItemTransferError, ItemTransferRefusal, ItemWeightLookup,
    ItemWeightRefused,
};
use crate::durability::spell_items_abi::{QualifiedItemDefinition, SpellFieldClass};
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{ChannelId, ScopeOwnershipGeneration, WorldId};
use crate::item_transfer_postgres_cases::{
    BACKPACK, CHANNEL, CHARACTER, Harness, SESSION, TestResult, WORLD, command, configured_admin,
    debug, fence, id, runtime, to_backpack,
};

const CORPSE: &str = "oteryn:item.tibia.i8139";
struct Content;
impl ItemWeightLookup for Content {
    fn generation_digest(&self) -> [u8; 32] {
        [1; 32]
    }
    fn weight_of(&self, d: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        self.loop_facts(d)
            .map(|_| 1)
            .ok_or(ItemWeightRefused::Unknown)
    }
}
impl ItemLoopContent for Content {
    fn loop_content_digest(&self) -> [u8; 32] {
        [1; 32]
    }
    fn loop_facts(&self, d: &TypedDefinitionRef) -> Option<ItemDefinitionFacts> {
        (d.family == "Item"
            && d.revision_ref == "definition-r1"
            && matches!(d.production_key.as_str(), CORPSE | BACKPACK))
        .then(|| ItemDefinitionFacts {
            definition: d.clone(),
            weight: Some(1),
            stack: ItemStackClass::NonStackable,
            container_capacity: if d.production_key == BACKPACK {
                Some(20)
            } else {
                Some(15)
            },
            container_slot_equip_pattern: d.production_key == BACKPACK,
        })
    }
}
fn qualified() -> QualifiedItemDefinition {
    QualifiedItemDefinition {
        definition: typed("Item", CORPSE, "definition-r1"),
        content_generation_digest: [1; 32],
        materializable: true,
        movable: true,
        stack_maximum: 1,
        container_capacity: Some(15),
        inventory_destination: true,
        ground_destination: true,
        decay: None,
        decay_chain: vec![],
        field_class: SpellFieldClass::NotField,
    }
}
fn target() -> AltarArrivalTarget {
    AltarArrivalTarget::new(
        "oteryn:item-arrival/elemental_spheres/energy_overlord",
        (1..=8).map(|key| (key, 0)).collect(),
        0,
    )
}

async fn mint_corpse(
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    actor: u32,
) -> TestResult<[u8; 16]> {
    let mut candidate = h
        .root
        .freeze_item_mint(
            &authority,
            &h.node,
            ItemMintRequest {
                cause: ItemMintCause::for_test(
                    WorldId::decode(&id(WORLD)).map_err(debug)?,
                    ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
                    ScopeOwnershipGeneration::new(1).map_err(debug)?,
                    actor,
                    1,
                    typed("LootTable", "fixture:loot.b3", "definition-r1"),
                    CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
                    0,
                ),
                item: qualified().definition,
                quantity: 1,
                ground: GroundPlacement {
                    spatial_position: vec![0, 0, 0, 1, 0, 0, 0, 2, 0, 3],
                    corpse_ref: id(3).to_vec(),
                    map_revision: "map-1".into(),
                    content_revision: "content-1".into(),
                    native_room_placement_context: id(6).to_vec(),
                },
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(debug)?;
    let permit = candidate.fresh_death_lane_permit().await;
    let ItemMintOutcome::Committed(corpse) = h
        .root
        .commit_corpse_mint(&permit, &authority, &h.node, &mut candidate, id(CHARACTER))
        .await
        .map_err(debug)?
    else {
        return Err("corpse MINT".into());
    };
    drop(permit);
    Ok(corpse.item_instance_id)
}

#[test]
fn m49_drop_retirement_audit_and_boss_latch_commit_or_roll_back_together() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64<<20).spawn(move || -> Result<(),String> {
        runtime().map_err(debug)?.block_on(async move {
            let h=Harness::create(admin,"m49_atomic").await?;
            let durations:Vec<(String,i64)>=sqlx::query_as("SELECT corpse_key,corpse_duration_ms FROM game_elemental_altar_rules ORDER BY corpse_key LIMIT 4").fetch_all(&h.pool).await?;
            assert_eq!(durations,vec![("oteryn:item.tibia.i8106".into(),300000),("oteryn:item.tibia.i8139".into(),500000),("oteryn:item.tibia.i8140".into(),500000),("oteryn:item.tibia.i8141".into(),500000)]);
            let seal=h.recovery.seal_current().map_err(debug)?;
            let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
            equip_backpack(&h,&authority,fence()?,SESSION).await?;
            let corpse_id=mint_corpse(&h,&authority,100).await?;
            let facts=Content.loop_facts(&qualified().definition).ok_or("corpse facts")?;
            assert!(matches!(h.transfer(&authority,fence()?,to_backpack(command(2)?,corpse_id,facts)).await,Err(ItemTransferError::Refused(ItemTransferRefusal::CorpseNotPickupable))));
            sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).bind([1u8;32].as_slice()).bind([2u8;32].as_slice()).bind([3u8;32].as_slice()).execute(&h.pool).await?;
            let mut tx=h.pool.begin().await?;
            let eq=assert_equipment_authority_in_transaction(&mut tx,&h.root,&authority,&h.node,&fence()?,command(90)?,[1;32]).await.map_err(debug)?;
            initialize_equipment_in_transaction(&mut tx,&eq).await.map_err(debug)?;
            tx.commit().await?;
            let request=LoopMoveRequest {item:corpse_id,count:None,pickup_cell:Some(vec![0,0,0,1,0,0,0,2,0,3]),destination:LoopDestination::Ground(LoopGround {spatial_position:vec![0,0,0,5,0,0,0,6,0,7],map_revision:"map-1".into(),content_revision:"content-1".into(),placement_context:id(6).to_vec()})};
            let permit=SpellLanePermit::of_fresh_scope(fence()?.runtime_scope).await.map_err(debug)?;
            // Fail after retirement and latch insertion: the ordinary relocation and
            // every consequence must roll back together.
            sqlx::query("CREATE FUNCTION m49_fail() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN IF EXISTS(SELECT 1 FROM game_elemental_altar_receipts WHERE transaction_id=NEW.transaction_id) THEN RAISE EXCEPTION 'M49 injected failure'; END IF; RETURN NEW; END$$").execute(&h.pool).await?;
            sqlx::query("CREATE TRIGGER m49_fail BEFORE INSERT ON game_item_audit_outbox FOR EACH ROW EXECUTE FUNCTION m49_fail()").execute(&h.pool).await?;
            assert!(h.root.commit_item_loop_altar_arrival(&permit,&authority,&h.node,fence()?,command(3)?,&Content,request.clone(),&qualified(),target()).await.is_err());
            let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_item_loop_move_receipts WHERE command_id=3),(SELECT count(*) FROM game_elemental_altar_receipts),(SELECT count(*) FROM game_elemental_boss_latches)").fetch_one(&h.pool).await?;
            assert_eq!(counts,(0,0,0));
            let lifecycle:i16=sqlx::query_scalar("SELECT lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(corpse_id.as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(lifecycle,1);
            let source:Vec<u8>=sqlx::query_scalar("SELECT spatial_position FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid").bind(corpse_id.as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(source,vec![0,0,0,1,0,0,0,2,0,3]);
            sqlx::query("DROP TRIGGER m49_fail ON game_item_audit_outbox").execute(&h.pool).await?;
            let result=h.root.commit_item_loop_altar_arrival(&permit,&authority,&h.node,fence()?,command(3)?,&Content,request.clone(),&qualified(),target()).await.map_err(debug)?.map_err(debug)?;
            assert!(result.summon.is_some());
            let proof:(i16,i64,bool,bool)=sqlx::query_as("SELECT i.lifecycle,i.quantity,r.created_xact_id=m.created_xact_id,a.created_xact_id=r.created_xact_id FROM game_elemental_altar_receipts r JOIN game_item_loop_move_receipts m ON m.transaction_id=r.move_transaction_id JOIN game_elemental_boss_latches l ON l.arrival_transaction_id=r.transaction_id JOIN game_item_instances i ON i.last_transaction_id=r.transaction_id JOIN game_item_audit_outbox a ON a.transaction_id=r.transaction_id").fetch_one(&h.pool).await?;
            assert_eq!(proof,(2,0,true,true));
            let replay=h.root.commit_item_loop_altar_arrival(&permit,&authority,&h.node,fence()?,command(3)?,&Content,request,&qualified(),target()).await.map_err(debug)?.map_err(debug)?;
            assert!(replay.moved.replayed);
            assert!(replay.summon.is_none());
            let pending=h.root.read_pending_altar_bosses(&permit,&authority,&h.node,fence()?.runtime_scope,ScopeOwnershipGeneration::new(1).map_err(debug)?,0,[1;32],"map-1").await.map_err(debug)?;
            assert_eq!(pending.len(),1);
            assert!(pending[0].actor.is_none());
            let boss=crate::foundation::ExactActorRef(crate::foundation::ActorRef::from_scoped_owner_parts(fence()?.runtime_scope,ScopeOwnershipGeneration::new(1).map_err(debug)?,crate::foundation::ActorLocalId(201),crate::foundation::ActorLocalGeneration(1)));
            assert!(h.root.record_altar_boss_realization(&permit,&authority,&h.node,pending[0].source.transaction_id,boss).await.map_err(debug)?);
            let mut stale=boss;
            stale.0.set_scope_generation(ScopeOwnershipGeneration::new(2).map_err(debug)?);
            assert!(h.root.record_altar_boss_realization(&permit,&authority,&h.node,pending[0].source.transaction_id,stale).await.is_err());
            let reloaded=h.root.read_pending_altar_bosses(&permit,&authority,&h.node,fence()?.runtime_scope,ScopeOwnershipGeneration::new(1).map_err(debug)?,0,[1;32],"map-1").await.map_err(debug)?;
            assert_eq!(reloaded[0].actor,Some((201,1)));
            drop(permit);
            mint_corpse(&h,&authority,202).await?;
            let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_elemental_boss_latches").fetch_one(&h.pool).await?;
            assert_eq!(count,1);
            mint_corpse(&h,&authority,201).await?;
            let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_elemental_boss_latches").fetch_one(&h.pool).await?;
            assert_eq!(count,0);
            drop(authority);
            drop(seal);
            h.cleanup().await?;
            Ok::<(),Box<dyn std::error::Error>>(())
        }).map_err(debug)
    })?.join().map_err(|_|"M49 test thread failed")?.map_err(Into::into)
}
