//! Native corpse-owner component proof; production encounter qualification is separate.
use super::{
    AssignmentCommand, TestResult, committed, fixture, id, readiness, request, runtime_fence,
};
use crate::durability::admission_authority_guards::GuardPublicationDisposition;
use crate::durability::item_mint::{
    CORPSE_MATERIALIZATION_PURPOSE_KEY, CorpseContainerPlacement, CorpseLootMintRequest,
    GroundPlacement, ItemMintCause, ItemMintError, ItemMintOutcome, ItemMintRequest,
    TypedDefinitionRef,
};
use crate::durability::item_mint_audit;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{
    ChannelId, CreatureDeathOccurrenceKey, RuntimeScopeRefV1, ScopeOwnershipGeneration,
};
use oteryn_gameplay_entities::carrier::CombatDeathFixture;

fn definition(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "fixture-r1".into(),
    }
}

fn physical_death(
    scope: RuntimeScopeRefV1,
    generation: u64,
) -> TestResult<(CreatureDeathOccurrenceKey, GroundPlacement)> {
    let mut native = CombatDeathFixture::new_for_scope(
        scope,
        ScopeOwnershipGeneration::new(generation).map_err(fixture::debug)?,
    )
    .map_err(fixture::debug)?;
    native
        .strike("fixture:instance-corpse.strike", CombatDeathFixture::HEALTH)
        .map_err(fixture::debug)?;
    let corpse = native.project_death_projection().map_err(fixture::debug)?;
    let death = corpse.occurrence().death_key().map_err(fixture::debug)?;
    let at = corpse.position();
    let mut spatial_position = Vec::new();
    spatial_position.extend_from_slice(&at.x.to_be_bytes());
    spatial_position.extend_from_slice(&at.y.to_be_bytes());
    spatial_position.extend_from_slice(&at.floor.to_be_bytes());
    let (frame, map, content) = corpse.context_markers();
    let mut placement = Vec::new();
    for value in [frame, map, content] {
        placement.extend_from_slice(&value.to_be_bytes());
    }
    Ok((
        death,
        GroundPlacement {
            spatial_position,
            corpse_ref: death.native_bytes(),
            map_revision: "map-1".into(),
            content_revision: "content-1".into(),
            native_room_placement_context: placement,
        },
    ))
}

fn corpse_request(
    scope: RuntimeScopeRefV1,
    generation: u64,
) -> TestResult<(CreatureDeathOccurrenceKey, ItemMintRequest)> {
    let (death, ground) = physical_death(scope, generation)?;
    let item = definition("fixture:instance-corpse");
    Ok((
        death,
        ItemMintRequest {
            cause: ItemMintCause::from_creature_death(
                death,
                item.clone(),
                CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
                0,
            )
            .map_err(fixture::debug)?,
            item,
            quantity: 1,
            ground,
            content_revision: "content-1".into(),
            ruleset_revision: "ruleset-1".into(),
            sim_revision: "sim-1".into(),
        },
    ))
}

async fn qualify(h: &fixture::Harness) -> TestResult {
    // Crystal 201189f the_primal_menace.lua:79,94 requires a real corpse even
    // when the activity boss's personal reward owner is a separate descendant.
    let character = fixture::fence(1)?.character_id;
    let world = fixture::fence(1)?.runtime_scope.world_id();
    let scope = RuntimeScopeRefV1::instance(world, id(88)).map_err(fixture::debug)?;
    sqlx::query("INSERT INTO game_control_activity_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice()).bind(id(88).as_slice()).execute(&h.pool).await?;
    committed(
        h.writer
            .submit(&request(
                144,
                AssignmentCommand::Assign {
                    scope,
                    target: h.node.fact(),
                },
            )?)
            .await,
    )?;
    assert_eq!(
        h.root
            .publish_runtime_readiness(&h.node, &readiness(h, scope, 1, 1).await?)
            .await
            .map_err(fixture::debug)?,
        GuardPublicationDisposition::Applied
    );
    assert!(runtime_fence(h, scope, 1, &h.node).await?);
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let (death, request) = corpse_request(scope, 1)?;
    let mut corpse = h
        .root
        .freeze_item_mint(&authority, &h.node, request.clone())
        .await
        .map_err(fixture::debug)?;
    let corpse_id = *corpse.item_instance_id();
    let alias =
        RuntimeScopeRefV1::channel(world, ChannelId::decode(&id(88)).map_err(fixture::debug)?);
    let wrong = SpellLanePermit::of_fresh_scope(alias).await?;
    assert!(
        h.root
            .commit_corpse_mint(
                &wrong,
                &authority,
                &h.node,
                &mut corpse,
                *character.as_bytes()
            )
            .await
            .is_err()
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_mint_receipts WHERE item_instance_id=encode($1,'hex')::uuid").bind(corpse_id.as_slice()).fetch_one(&h.pool).await?,0);
    let permit = corpse.fresh_death_lane_permit().await;
    assert!(matches!(
        h.root
            .commit_corpse_mint(
                &permit,
                &authority,
                &h.node,
                &mut corpse,
                *character.as_bytes()
            )
            .await
            .map_err(fixture::debug)?,
        ItemMintOutcome::Committed(_)
    ));
    assert!(matches!(
        h.root
            .commit_corpse_mint(
                &permit,
                &authority,
                &h.node,
                &mut corpse,
                *character.as_bytes()
            )
            .await
            .map_err(fixture::debug)?,
        ItemMintOutcome::AlreadyCommitted(_)
    ));
    let stored = h
        .root
        .read_item_instance(&authority, corpse_id)
        .await
        .map_err(fixture::debug)?
        .ok_or("missing Instance corpse")?;
    assert_eq!(stored.runtime_scope, scope);
    assert_eq!(stored.ground.corpse_ref, death.native_bytes());
    let loot_request = CorpseLootMintRequest {
        cause: ItemMintCause::from_creature_death(
            death,
            TypedDefinitionRef {
                family: "LootTable".into(),
                production_key: "fixture:instance-loot".into(),
                revision_ref: "fixture-r1".into(),
            },
            "fixture:entry".into(),
            1,
        )
        .map_err(fixture::debug)?,
        item: definition("fixture:instance-loot"),
        quantity: 1,
        placement: CorpseContainerPlacement {
            corpse_item_instance_id: corpse_id,
            placement_ordinal: 1,
            fluid_subtype: None,
        },
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
    };
    let mut loot = h
        .root
        .freeze_corpse_loot_mint(&authority, &h.node, loot_request)
        .await
        .map_err(fixture::debug)?;
    let loot_id = *loot.item_instance_id();
    assert!(matches!(
        h.root
            .commit_corpse_loot_mint(&permit, &authority, &h.node, &mut loot)
            .await
            .map_err(fixture::debug)?,
        ItemMintOutcome::Committed(_)
    ));
    assert!(matches!(
        h.root
            .commit_corpse_loot_mint(&permit, &authority, &h.node, &mut loot)
            .await
            .map_err(fixture::debug)?,
        ItemMintOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_corpse_container_entries WHERE item_instance_id=encode($1,'hex')::uuid AND parent_item_instance_id=encode($2,'hex')::uuid").bind(loot_id.as_slice()).bind(corpse_id.as_slice()).fetch_one(&h.pool).await?,1);
    for rejected in [
        corpse_request(alias, 1)?.1,
        corpse_request(scope, 2)?.1,
        corpse_request(
            RuntimeScopeRefV1::instance(world, id(89)).map_err(fixture::debug)?,
            1,
        )?
        .1,
    ] {
        assert!(matches!(
            h.root.freeze_item_mint(&authority, &h.node, rejected).await,
            Err(ItemMintError::AuthorityRejected)
        ));
    }
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_mint_receipts WHERE death_scope_key=game_activity_scope_key(encode($1,'hex')::uuid,encode($2,'hex')::uuid) AND death_scope_kind=2 AND death_channel_id IS NULL AND death_instance_id=encode($2,'hex')::uuid").bind(world.as_bytes().as_slice()).bind(id(88).as_slice()).fetch_one(&h.pool).await?,2);
    // The same migrated owner retains the legacy Channel cause and framing.
    let channel_scope = fixture::fence(1)?.runtime_scope;
    let (channel_death, channel_request) = corpse_request(channel_scope, 1)?;
    assert_eq!(channel_death.native_bytes().len(), 52);
    let mut channel_corpse = h
        .root
        .freeze_item_mint(&authority, &h.node, channel_request)
        .await
        .map_err(fixture::debug)?;
    let channel_permit = channel_corpse.fresh_death_lane_permit().await;
    assert!(matches!(
        h.root
            .commit_corpse_mint(
                &channel_permit,
                &authority,
                &h.node,
                &mut channel_corpse,
                *character.as_bytes()
            )
            .await
            .map_err(fixture::debug)?,
        ItemMintOutcome::Committed(_)
    ));
    let stored_channel = h
        .root
        .read_item_instance(&authority, *channel_corpse.item_instance_id())
        .await
        .map_err(fixture::debug)?
        .ok_or("missing Channel control corpse")?;
    assert_eq!(stored_channel.runtime_scope, channel_scope);
    assert_eq!(
        stored_channel.ground.corpse_ref,
        channel_death.native_bytes()
    );
    let channel_v1: bool = sqlx::query_scalar("SELECT death_scope_kind=1 AND death_channel_id IS NOT NULL AND death_instance_id IS NULL AND get_byte(intent_binding,0)=1 FROM game_item_mint_receipts WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(channel_corpse.item_instance_id().as_slice()).fetch_one(&h.pool).await?;
    assert!(channel_v1);
    let envelopes:Vec<Vec<u8>>=sqlx::query_scalar("SELECT envelope FROM game_item_audit_outbox WHERE uuid_send(item_instance_id)=ANY($1::bytea[]) ORDER BY item_instance_id")
        .bind(vec![corpse_id.to_vec(),loot_id.to_vec()]).fetch_all(&h.pool).await?;
    assert_eq!(envelopes.len(), 2);
    for wire in envelopes {
        let (envelope, mint) = item_mint_audit::decode_envelope(&wire).map_err(fixture::debug)?;
        assert_eq!(envelope.instance_id, Some(id(88).to_vec()));
        assert!(envelope.channel_id.is_none());
        let source = mint
            .source
            .ok_or("missing source")?
            .death_occurrence
            .ok_or("missing death")?;
        assert_eq!(source.runtime_scope().map_err(fixture::debug)?, scope);
    }
    Ok(())
}

#[test]
fn native_instance_corpse_mint_postgres_physical_death_loot_replay_and_scope_fences() -> TestResult
{
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    fixture::runtime()?.block_on(async move {
        let h=fixture::Harness::create(admin,"encx_native_corpse",true).await?;
        eprintln!("Canonical PostgreSQL 18.6; physical Instance death, native corpse/loot owner, replay and namespace fences");
        let result=qualify(&h).await;
        h.cleanup().await?;
        result
    })
}
