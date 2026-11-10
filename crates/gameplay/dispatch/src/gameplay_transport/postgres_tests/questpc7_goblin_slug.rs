//! Fresh Slug corpses from an ordinary native kill, without seeding quest odours.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc7_goblin_native_slug_death_materializes_fresh_corpse() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    let started=std::time::Instant::now();
    let result=super::kill_reward_e2e::scenario_thread::run("questpc7-goblin-slug",||super::quest_world_callbacks::scenario("qengine-rewards-goblin-slug",Position{x:33075,y:32109,floor:-7},false));
    eprintln!("QUESTPC7_GOBLIN_SLUG POSTGRES_CASE_COMPLETED duration={:?} success={}",started.elapsed(),result.is_ok());result
}

pub(super) async fn exercise(owner:&mut ComposedFreshAdmission<'_, '_, '_>,a:&mut AdmittedSession,h:&Harness)->TestResult {
    let version:String=sqlx::query_scalar("SHOW server_version").fetch_one(&h.pool).await?;
    eprintln!("QUESTPC7_GOBLIN_SLUG POSTGRES_STARTED version={version}");
    super::questpc7_goblin::exercise(owner,a,h).await?;
    super::questpc7_native_deaths::prepare_native_hunter(owner,a).await?;
    // Travel-only fixture terrain, not an asserted Slug source-map spawn.
    super::questpc7_goblin::stalker_death(owner,a,"oteryn:creature.slug",Position{x:33075,y:32110,floor:-7},11000).await?;
    let bodies:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances i JOIN game_item_ground_locations g ON g.item_instance_id=i.item_instance_id WHERE i.definition_production_key='oteryn:item.tibia.i12526' AND i.quantity=1 AND octet_length(g.corpse_ref)>0").fetch_one(&h.pool).await?;
    assert_eq!(bodies,1,"duplicate lethal notifications materialize one fresh native corpse");
    assert_eq!(owner.runtime.lock().await.visible_corpses().len(),1);
    let native=owner.active_generation.and_then(|g|g.native_gameplay()).ok_or("native")?;
    for item in [12526,12533,12534,12535] {
        let policy=native.item_policy_for_source_id(item).ok_or_else(||format!("missing Slug corpse policy {item}"))?;
        assert_eq!(serde_json::to_value(policy.record())?["admission"]["materializable"],true);
    }
    assert_eq!(super::questpc7_goblin::receipts(h).await?,2,"ordinary corpse creation earns no quest receipt");
    for suffix in ["programme","signs","odours","captures","urine","trunks","stalkers"] {
        assert_eq!(super::questpc7_ferumbras::value(owner,a,&format!("oteryn:quest-progress/questpc7/goblin/{suffix}")).await?,0);
    }
    for item in [17672,17673,17674,17675] {
        assert_eq!(super::questpc7_outfit_items::quantity(h,item).await?,0,"no fabricated odour or vial transformation");
    }
    Ok(())
}
