//! Original compiled map and NPCframe; no source NPC is reseeded beside a synthetic actor.
use super::*;
use crate::content::{ReferenceItemField as F, ReferenceItemType};
pub(super) const ENTRY: (i32, i32, i16) = (32951, 32103, 5);
pub(super) fn world(
    world: crate::foundation::WorldId,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<(crate::map::boot::BundleWorld, NpcPlacementTable)> {
    let bytes = std::fs::read(
        std::env::var_os("OTERYN_SRV7_SIMON_LIFETIME_BUNDLE")
            .ok_or("actual retailer source bundle")?,
    )?;
    let b = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    let table = NpcPlacementTable {
        npcs: b
            .npcs
            .npcs
            .iter()
            .filter(|npc| {
                matches!(
                    npc.key.as_str(),
                    "oteryn:npc.hugo"
                        | "oteryn:npc.simon_the_beggar"
                        | "oteryn:npc.captain_fearless"
                )
            })
            .map(|n| NpcPlacements {
                key: n.key.clone(),
                placements: n
                    .placements
                    .iter()
                    .map(|p| NpcPlacement {
                        x: p.x,
                        y: p.y,
                        floor: p.floor,
                        direction: match p.direction {
                            oteryn_world_bundle::npc::Direction::North => MovementFacing::North,
                            oteryn_world_bundle::npc::Direction::East => MovementFacing::East,
                            oteryn_world_bundle::npc::Direction::South => MovementFacing::South,
                            oteryn_world_bundle::npc::Direction::West => MovementFacing::West,
                        },
                    })
                    .collect(),
            })
            .collect(),
    };
    let hugo = table
        .npcs
        .iter()
        .find(|n| n.key == "oteryn:npc.hugo")
        .ok_or("real Hugo/Simon source placement")?;
    assert_eq!(hugo.placements.len(), 1);
    assert_eq!(
        (
            hugo.placements[0].x,
            hugo.placements[0].y,
            hugo.placements[0].floor
        ),
        (32950, 32102, -5)
    );
    let pins = crate::map::BundlePins {
        digest: b.digest,
        project_format_version: b.manifest.identity.project_format_version.clone(),
        world_schema_version: b.manifest.identity.world_schema_version.clone(),
        content_revision: b.manifest.identity.content_revision.clone(),
        production: false,
    };
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let index = native.item_index().ok_or("retailer native Item index")?;
    let boot=crate::map::boot::boot(&bytes,&crate::map::boot::BootPins{bundle:pins,map_revision:crate::map::overlay::map_revision(&base),start:crate::map::overlay::TilePos{x:32951,y:32103,floor:-5}},world,crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,|key|{
 let rev=index.revision(key)?;let policy=native.item_policy(key,rev).map(|p|p.record());
 Some(crate::map::facts::ItemDefinition{reference:index.definition_ref("Item",key,rev)?,solid:policy.and_then(|p|p.attributes.blocks_movement),blocks_projectile:policy.and_then(|p|p.attributes.blocks_projectile).unwrap_or(true),pickupable:policy.is_some_and(|p|matches!(&p.semantics.physical,F::Known(p) if p.pickupable==F::Known(true))),door:policy.is_some_and(|p|matches!(&p.semantics.classification,F::Known(c) if c.item_type==F::Known(ReferenceItemType::Door)))})
 }).map_err(debug)?;
    Ok((boot, table))
}
#[path = "simon_boat_route.rs"]
mod boat_route;
#[path = "simon_open_exit.rs"]
mod open_exit;
#[path = "simon_ladder_route.rs"]
mod ladder_route;
#[path = "simon_walk.rs"]
mod walk;
// A new actual serve stream emits an empty NPC snapshot and clears focus.
// Every later transport episode explicitly greets; no command retry occurs.
async fn talk(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    first: u64,
    words: &[&str],
) -> TestResult {
    let commands = words
        .iter()
        .enumerate()
        .map(|(i, text)| Ok((first + u64::try_from(i)?, *text)))
        .collect::<TestResult<Vec<_>>>()?;
    super::super::grizzly_npc::wire::talk_paced(owner, admitted, npc, &commands).await
}
async fn stage(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/quest/u7_8/beggar_outfits/beggar_outfit'").bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}
pub(super) async fn exercise(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    hugo: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
) -> TestResult {
    assert!(
        owner
            .session_premium_gate(admitted.game_session_id)
            .await
            .premium_current(),
        "genuine Premium owner must admit Hugo source"
    );
    sqlx::query("CREATE TABLE srv7_simon_fixture_clock(seconds BIGINT NOT NULL)")
        .execute(&h.pool)
        .await?;
    sqlx::query("INSERT INTO srv7_simon_fixture_clock SELECT floor(extract(epoch FROM public.game_daily_effect_now()))::bigint").execute(&h.pool).await?;
    sqlx::query("CREATE OR REPLACE FUNCTION public.game_daily_effect_now() RETURNS TIMESTAMPTZ LANGUAGE SQL STABLE SET search_path=pg_catalog AS $$SELECT to_timestamp(seconds) FROM public.srv7_simon_fixture_clock$$").execute(&h.pool).await?;
    talk(owner, admitted, hugo, 1, &["hi", "outfit", "yes", "yes"]).await?;
    assert_eq!(
        stage(h).await?,
        1,
        "actual Hugo Start, never injected predecessor"
    );
    let mut command = 5;
    for (identity, key, count, keyword, expected) in [
        (
            90,
            "oteryn:item.tibia.i5913",
            20,
            "20 brown pieces of cloth",
            2,
        ),
        (91, "oteryn:item.tibia.i5878", 50, "50 minotaur leathers", 3),
        (92, "oteryn:item.tibia.i5894", 10, "10 bat wings", 4),
        (93, "oteryn:item.tibia.i5921", 30, "30 heaven blossoms", 5),
    ] {
        super::super::item_forge::support::seed_items(h, &[(identity, key, count, 0)]).await?;
        // Every new physical operand is explicit historical custody. Quest state is
        // produced solely by the real native command7 source actions.
        talk(owner, admitted, hugo, command, &["hi", keyword, "yes"]).await?;
        command += 3;
        assert_eq!(stage(h).await?, expected);
        let left:i64=sqlx::query_scalar("SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND lifecycle=1").bind(id(identity).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            left, 0,
            "source material burn must commit with its track successor"
        );
    }
    let deadline:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/quest/u7_8/beggar_outfits/beggar_outfit_timer'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    let now: i64 = sqlx::query_scalar("SELECT seconds FROM srv7_simon_fixture_clock")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(deadline, now + 86400, "actual native source deadline");
    talk(owner, admitted, hugo, command, &["hi", "outfit"]).await?;
    command += 2;
    assert_eq!(
        stage(h).await?,
        5,
        "strict source wait must refuse early acquisition"
    );
    sqlx::query("UPDATE srv7_simon_fixture_clock SET seconds=seconds+86401")
        .execute(&h.pool)
        .await?;
    talk(owner, admitted, hugo, command, &["hi", "outfit", "yes"]).await?;
    command += 3;
    assert_eq!(
        stage(h).await?,
        6,
        "actual Hugo completion produces Simon prerequisite"
    );
    let base_owned:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlocks WHERE account_id=encode($1,'hex')::uuid AND unlock_key='oteryn:appearance-grant/npc/hugo/beggar/base'").bind(id(40).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        base_owned, 1,
        "actual paired base153/157 ownership persists"
    );
    let retained:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/quest/u7_8/beggar_outfits/beggar_outfit_timer'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(retained, deadline, "only the trusted wall clock advances");
    let simon = {
        let runtime = owner.runtime.lock().await;
        let found = runtime
            .visible_entities()
            .npcs
            .into_iter()
            .find(|n| runtime.npc_key(n.actor) == Some("oteryn:npc.simon_the_beggar"))
            .ok_or("actual source Simon")?;
        oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: found.actor.placement_identity(),
            generation: found.generation,
        }
    };
    open_exit::open_hugo_exit(owner, admitted, &mut command).await?;
    boat_route::reach_simon(h, owner, admitted, &mut command).await?;
    talk(
        owner,
        admitted,
        simon,
        command,
        &["hi", "outfit", "yes", "yes"],
    )
    .await?;
    command += 4;
    assert_eq!(
        stage(h).await?,
        7,
        "Simon must accept genuinely earned source base outfit"
    );
    super::super::item_forge::support::seed_items(h, &[(94, "oteryn:item.tibia.i5883", 100, 0)])
        .await?;
    talk(owner, admitted, simon, command, &["hi", "ape fur", "yes"]).await?;
    command += 3;
    assert_eq!(
        stage(h).await?,
        7,
        "no-funds source refusal leaves stage intact"
    );
    let retained_fur:i64=sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND lifecycle=1").bind(id(94).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        retained_fur, 100,
        "insufficient funds rolls back compound fur burn"
    );
    super::super::item_forge::support::seed_items(h, &[(95, "oteryn:item.tibia.i3043", 2, 0)])
        .await?;
    talk(owner, admitted, simon, command, &["hi", "ape fur", "yes"]).await?;
    assert_eq!(
        stage(h).await?,
        8,
        "actual paid Simon source must commit fur and cash/bank child atomically"
    );
    let paid:i64=sqlx::query_scalar("SELECT count(*) FROM game_npc_cosmetic_receipts WHERE character_id=encode($1,'hex')::uuid AND source_key=ANY($2::text[])").bind(id(41).as_slice()).bind(vec!["oteryn:npc-cosmetic/simon_the_beggar/beggar-fur-pay-male","oteryn:npc-cosmetic/simon_the_beggar/beggar-fur-pay-female"]).fetch_one(&h.pool).await?;
    assert_eq!(paid, 1);
    Ok(())
}
