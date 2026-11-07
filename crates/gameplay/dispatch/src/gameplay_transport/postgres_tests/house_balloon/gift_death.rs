//! Genuine Native Knight130 and the exact source town8/Thais temple.
use super::*;
pub(super) const TOWN: u32 = 8;
pub(super) const TEMPLE: [i32; 3] = [32369, 32241, 7];
pub(super) fn world(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::bundle;
    let source_bytes = std::fs::read(
        std::env::var_os("OTERYN_SRV7_HOUSE_DEATH_BUNDLE")
            .ok_or("actual source Temple compiler bundle")?,
    )?;
    let mut source = bundle::read(&source_bytes).map_err(debug)?;
    let temple = source
        .manifest
        .subareas
        .town(TOWN)
        .and_then(|t| t.temple)
        .ok_or("actual source town8 Temple")?;
    assert_eq!((temple.x, temple.y, temple.floor), (32369, 32241, -7));
    let actual_sector = source
        .sectors
        .iter()
        .find(|s| s.floor == -7 && s.sx == 32369 / 32 && s.sy == 32241 / 32)
        .ok_or("actual source Thais Temple sector")?
        .clone();
    assert!(
        actual_sector
            .tiles
            .iter()
            .any(|t| t.x == 32369 && t.y == 32241)
    );
    // Existing controlled House geometry tests only the House ABI. Temple tiles,
    // their palette/physics, town catalogue and compressed-mask provenance are
    // copied unchanged from the actual compiled B3 source above.
    let (fixture_bytes, _) = crate::map::boot::tests::bundle_with_items(
        "oteryn:item.tibia.i1629",
        "oteryn:item.tibia.i1000",
    );
    let fixture = bundle::read(&fixture_bytes).map_err(debug)?;
    let mut house = fixture
        .sectors
        .first()
        .ok_or("controlled House sector")?
        .clone();
    let mut entrance = house
        .tiles
        .iter()
        .find(|t| t.x == 3)
        .ok_or("existing controlled entrance")?
        .clone();
    // Only the existing walkable House ABI corridor and real source door are
    // required. Do not import synthetic terrain identities into the source map.
    house.tiles.retain(|tile| tile.x != 4 && tile.x != 5);
    let source_ground = actual_sector
        .tiles
        .iter()
        .find(|tile| tile.x == 32369 && tile.y == 32241)
        .and_then(|tile| tile.items.first())
        .ok_or("actual source Temple ground palette")?
        .palette;
    for tile in &mut house.tiles {
        if tile.x == 7 {
            tile.house = 3437;
            tile.items.truncate(1);
        }
        if tile.x == 6 {
            tile.house = 3437;
            tile.items.last_mut().ok_or("controlled door")?.attrs.door = Some(42);
        }
        tile.x += 32303;
        tile.y += 32804;
    }
    entrance.x = 32309;
    entrance.y = 32805;
    house.tiles.push(entrance);
    house.sx = 1009;
    house.sy = 1025;
    for tile in &mut house.tiles {
        for item in &mut tile.items {
            if item.palette == 0 {
                item.palette = source_ground;
                continue;
            }
            let entry = fixture
                .manifest
                .palette
                .get(item.palette as usize)
                .ok_or("controlled palette entry")?;
            let slot = source
                .manifest
                .palette
                .iter()
                .position(|p| p.key == entry.key)
                .ok_or("controlled House identity exists in source registry")?;
            item.palette = u32::try_from(slot)?;
        }
    }
    source.sectors = vec![actual_sector, house];
    source
        .sectors
        .sort_by_key(oteryn_world_bundle::bundle::order);
    // Keep actual compiled source families and their manifest counts/provenance.
    // The House ABI needs no fixture monster or NPC frame replacement.
    let bytes = oteryn_world_bundle_compiler::bundle::write_with_npcs(
        &source.manifest,
        &source.sectors,
        &source.spawns,
        &source.npcs,
    )
    .map_err(debug)?;
    let b = bundle::read(&bytes).map_err(debug)?;
    let pins = crate::map::BundlePins {
        digest: b.digest,
        project_format_version: b.manifest.identity.project_format_version.clone(),
        world_schema_version: b.manifest.identity.world_schema_version.clone(),
        content_revision: b.manifest.identity.content_revision.clone(),
        production: false,
    };
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let index = native.item_index().ok_or("native source item index")?;
    crate::map::boot::boot(&bytes,&crate::map::boot::BootPins{bundle:pins,map_revision:crate::map::overlay::map_revision(&base),start:crate::map::overlay::TilePos{x:32309,y:32805,floor:-7}},world,channel,|key|{
  let revision=index.revision(key)?;let policy=native.item_policy(key,revision).map(|p|p.record());
  Some(crate::map::facts::ItemDefinition{reference:index.definition_ref("Item",key,revision)?,solid:policy.and_then(|p|p.attributes.blocks_movement),blocks_projectile:policy.and_then(|p|p.attributes.blocks_projectile).unwrap_or(true),pickupable:policy.is_some_and(|p|matches!(&p.semantics.physical,crate::content::ReferenceItemField::Known(physical) if physical.pickupable==crate::content::ReferenceItemField::Known(true))),door:key=="oteryn:item.tibia.i1629"})
 }).map_err(debug)
}
pub(super) async fn prepare_high_level(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult {
    let mut admitted = deps.admitted;
    admitted.runtime_actor = Some(deps.actor);
    admitted.item_fence = Some(
        owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "native130 source fence")?
            .ok_or("native130 source fence")?
            .item_fence(),
    );
    owner.admit_progression_session(&mut admitted).await;
    assert!(
        owner
            .progression_binding(admitted.game_session_id)
            .is_some(),
        "real Native progression context initialized before historical level130 predecessor"
    );
    let policy = native
        .progression()
        .ok_or("death native progression")?
        .policy_for("profile-1", "ruleset-1", "content-1")
        .map_err(debug)?;
    let progression = crate::gameplay_transport::SessionProgressionBinding::from_content(policy);
    let xp = progression
        .policy
        .thresholds
        .iter()
        .find(|t| t.level == 130)
        .ok_or("level130")?
        .minimum_experience
        .get();
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_character_progression_state SET level=130,total_experience=$2,simulation_revision=$3,evidence_revision=$4,declaration_revision=$5,policy_revision=$6,reward_revision=$7 WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).bind(xp).bind(&progression.context.simulation).bind(&progression.context.evidence)
        .bind(&progression.context.declaration).bind(&progression.policy_revision).bind(&progression.reward_revision).execute(&mut *tx).await?;
    tx.commit().await?;
    let fence = owner
        .current_quest_fence(deps.admitted.game_session_id)
        .await
        .map_err(|()| "death class fence")?
        .ok_or("death class fence")?;
    let formula = native.training_formula().ok_or("death training formula")?;
    let build = h
        .root
        .read_character_build_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    h.root
        .commit_character_build(
            owner.character,
            &h.node,
            fence,
            crate::durability::character_build::BuildChangeRequest {
                occurrence: crate::durability::character_build::BuildOccurrence::from_bytes(id(91))
                    .map_err(debug)?,
                cause: crate::durability::character_build::BuildCause::VocationChoice,
                after: crate::durability::character_build::convert_vocation(
                    formula, &build, "knight",
                )
                .map_err(debug)?,
                before: build,
                pruned_stance: None,
            },
            formula,
        )
        .await
        .map_err(debug)?;
    let crate::gameplay_transport::spell_character_facts::CastFactsLoad::Ready { facts, .. } =
        crate::gameplay_transport::spell_character_facts::load_character_cast_facts(
            &h.root,
            owner.character,
            fence.character_id,
        )
        .await
    else {
        return Err("actual Knight death facts".into());
    };
    let runtime = owner.runtime.lock().await;
    assert!(
        owner
            .states
            .spell_states
            .lock()
            .await
            .initialize(
                &runtime,
                deps.actor,
                deps.admitted.game_session_id,
                facts,
                (0, 0),
                owner.owner_now()
            )
            .is_some()
    );
    Ok(())
}
