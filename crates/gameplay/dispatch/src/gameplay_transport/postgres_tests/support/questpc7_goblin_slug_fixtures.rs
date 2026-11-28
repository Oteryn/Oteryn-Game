use super::fixture::Harness;

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_view::ItemKey;

use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseDisposition, UseIntent, UseTarget};

pub(super) async fn exercise(owner:&mut ComposedFreshAdmission<'_, '_, '_>,a:&mut AdmittedSession,h:&Harness)->TestResult {
    let version:String=sqlx::query_scalar("SHOW server_version").fetch_one(&h.pool).await?;
    eprintln!("QUESTPC7_GOBLIN_SLUG POSTGRES_STARTED version={version}");
    super::questpc7_goblin_fixtures::exercise(owner,a,h).await?;
    super::questpc7_native_deaths_fixtures::prepare_native_hunter(owner,a).await?;
    // Travel-only fixture terrain, not an asserted Slug source-map spawn.
    super::questpc7_goblin_fixtures::stalker_death(owner,a,"oteryn:creature.slug",Position{x:33075,y:32110,floor:-7},11000).await?;
    let bodies:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances i JOIN game_item_ground_locations g ON g.item_instance_id=i.item_instance_id WHERE i.definition_production_key='oteryn:item.tibia.i12526' AND i.quantity=1 AND octet_length(g.corpse_ref)>0").fetch_one(&h.pool).await?;
    assert_eq!(bodies,1,"duplicate lethal notifications materialize one fresh native corpse");
    assert_eq!(owner.runtime.lock().await.visible_corpses().len(),1);
    let native=owner.active_generation.and_then(|g|g.native_gameplay()).ok_or("native")?;
    for item in [12526,12533,12534,12535] {
        let policy=native.item_policy_for_source_id(item).ok_or_else(||format!("missing Slug corpse policy {item}"))?;
        assert_eq!(serde_json::to_value(policy.record())?["admission"]["materializable"],true);
    }
    assert_eq!(super::questpc7_goblin_fixtures::receipts(h).await?,2,"ordinary corpse creation earns no quest receipt");
    for suffix in ["programme","signs","odours","captures","urine","trunks","stalkers"] {
        assert_eq!(super::questpc7_ferumbras_fixtures::value(owner,a,&format!("oteryn:quest-progress/questpc7/goblin/{suffix}")).await?,0);
    }
    for item in [17672,17673,17674,17675] {
        assert_eq!(super::questpc7_outfit_items_fixtures::quantity(h,item).await?,0,"no fabricated odour or vial transformation");
    }
    let corpse: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(item_instance_id) FROM game_item_instances
         WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i12526'",
    ).fetch_one(&h.pool).await?;
    let kit = native.item_index().and_then(|index| {
        index.definition_ref("Item", "oteryn:item.tibia.i17670", "definition-r1")
    }).ok_or("Rafzan's issued gatherer")?;
    let command = UseCommand { game_session_id: a.game_session_id, command_id: 11001,
                               item_fence: a.item_fence };
    let result = owner.use_quest_tool(
        a.runtime_actor.ok_or("collector actor")?,
        command,
        UseIntent {
            target: UseTarget::ItemByDefinition(kit),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Item(std::num::NonZeroU64::MIN)),
            use_with_position: None,
        },
        None,
        Some(ItemKey::Instance(corpse.try_into().map_err(|_| "corpse instance")?)),
    ).await.map_err(|()| "Rafzan corpse dispatch")?.ok_or("Rafzan gatherer owner")?;
    assert_eq!(result.disposition, UseDisposition::Committed);
    assert_eq!(super::questpc7_outfit_items_fixtures::quantity(h,17672).await?,1);
    let odours = super::questpc7_ferumbras_fixtures::value(owner,a,
        "oteryn:quest-progress/questpc7/goblin/odours").await?;
    assert_eq!(odours,1,"the freshly accepted perfume task qualifies the first gas site");
    Ok(())
}
