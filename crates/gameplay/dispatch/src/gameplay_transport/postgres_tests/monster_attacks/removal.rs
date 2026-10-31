//! A typed Item-writer fixture supplies a real wall; the ordinary monster drain removes it.
use super::*;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::spell_items_abi::{
    CastCostBinding, QualifiedItemDefinition, SourceCasterOrigin, SpellFieldClass,
    SpellItemOperation, SpellItemTransactionOutcome, SpellItemTransactionRequest,
};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities;

pub(super) async fn scenario(harness: &Harness) -> TestResult {
    prepare_current_session(harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness.root.open_character_authority(&seal).await.map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_loop_monster(world, false);
    let active = controller.active().ok_or("active native generation")?;
    let (deps, monster) = dependencies(active, &pin, AttackCase::TheMonster)?;
    let mut owner = deps.owner(harness, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = deps.room.movement_cells();
    owner.required_capabilities = BoundRequiredCapabilities::for_channel_boot(
        &[11, 30],
        crate::foundation::RuntimeScopeRefV1::channel(world, deps.admitted.channel_id),
        1,
        "policy-1",
        crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
    ).map_err(debug)?;
    // The two unchanged source damage entries are bounded by 2800 melee + 1200
    // energy per pass (the_monster.lua:87-88). Survive every bounded attempt.
    let initial_health = 4_000_u32.checked_mul(64)
        .and_then(|damage| damage.checked_add(crate::gameplay_transport::actor_spell::tests::FACTS.max_health))
        .ok_or("test player pool overflow")?;
    initialize_player(&owner, deps.actor, deps.admitted.game_session_id, initial_health).await?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let record = native.creature_profiles().records.iter()
        .find(|record| record.profile.target.key == AttackCase::TheMonster.creature_key())
        .ok_or("The Monster source profile")?;
    let crate::content::ProjectV2AuthoringProfileData::Behavior(behavior) =
        &record.behavior.as_ref().ok_or("source behavior")?.data else {
            return Err("source behavior kind".into());
        };
    // Crystal 201189f, bosses/the_monster.lua:86-90 and
    // scripts/spells/monster/destroy_magic_walls.lua:9-24.
    assert_eq!(behavior.attacks.len(), 3);
    let attack = &behavior.attacks[2];
    assert_eq!((attack.interval_ms, attack.chance_ppm), (1_000, 500_000));
    assert_eq!(attack.ability.key, "oteryn:ability.spell.destroy_magic_walls");
    let source = native.source_item_ability(&record.profile.target, &attack.ability)
        .ok_or("canonical source removal body")?;
    let crate::content::ProjectV2AuthoringProfileData::Ability(source) = &source.data else {
        return Err("source removal Ability kind".into());
    };
    let details = source.details.as_ref().ok_or("source removal details")?;
    assert_eq!(details.range_tiles, 0);
    assert!(!details.needs_target);
    assert_eq!(details.effects.len(), 1);
    let crate::content::ProjectV2AbilityEffect::Inline(effect) = &details.effects[0] else {
        return Err("source removal inline effect".into());
    };
    let crate::content::ProjectV2InlineEffectOperation::RemoveItems { items, selection } =
        &effect.operation else {
            return Err("source removal operation".into());
        };
    assert_eq!(*selection, crate::content::ProjectV2RemoveItemsSelection::TopItemFirstTile);
    assert_eq!(items.iter().map(|item| item.key.as_str()).collect::<Vec<_>>(), [
        "oteryn:item.tibia.i10181", "oteryn:item.tibia.i2128",
        "oteryn:item.tibia.i10182", "oteryn:item.tibia.i2130",
    ]);
    let wall = seed_wall(harness, &deps, &owner).await?;
    assert_eq!(wall_state(harness, wall).await?, (1, 1, 1, 1, 1));
    assert!(removal_receipts(harness, monster, wall).await?.is_empty());
    owner.clock_origin -= Duration::from_secs(10);
    let mut committed = Vec::new();
    for _ in 0..64 {
        owner.drain_monster_melee().await;
        committed = removal_receipts(harness, monster, wall).await?;
        if !committed.is_empty() {
            break;
        }
        assert!(health(&owner, &deps).await? > 0, "real source target remains alive");
        owner.clock_origin -= Duration::from_millis(1_000);
    }
    assert_eq!(committed.len(), 1, "the ordinary source cast commits one real removal");
    assert!(committed[0].1.starts_with("ai-profile:"));
    assert!(committed[0].1.ends_with(":attack:2"));
    assert_eq!((committed[0].2, committed[0].3, committed[0].4), (2, 0, 2));
    assert_eq!((committed[0].5, committed[0].6), (0, 1));
    assert_eq!(wall_state(harness, wall).await?, (2, 0, 2, 0, 1));
    let sequence = owner.states.spell_states.lock().await.monster_ai_sequence;
    owner.drain_monster_melee().await;
    assert_eq!(owner.states.spell_states.lock().await.monster_ai_sequence, sequence);
    assert_eq!(removal_receipts(harness, monster, wall).await?, committed,
        "repeated drain cannot retire the item or append its source receipt again");
    assert_eq!(wall_state(harness, wall).await?, (2, 0, 2, 0, 1));
    Ok(())
}

async fn seed_wall(
    harness: &Harness,
    deps: &Dependencies,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult<[u8; 16]> {
    use crate::durability::spell_item_transaction::{
        apply_spell_items_in_transaction, assert_spell_item_authority_in_transaction,
    };
    let native = owner.active_generation.and_then(|active| active.native_gameplay())
        .ok_or("native gameplay")?;
    let policy = native.item_policy_for_source_id(2130).ok_or("native Wild Growth policy")?;
    let definition = crate::durability::native_world_items::qualify_source_field_chain(
        native, QualifiedItemDefinition::from_native_policy(policy).map_err(debug)?,
    ).map_err(debug)?;
    assert_ne!(definition.field_class, SpellFieldClass::NotField);
    let (blocks_movement, blocks_projectile) =
        oteryn_gameplay_items::native_world_items::source_barrier_collision(policy)
            .ok_or("native Wild Growth collision")?;
    assert_eq!((blocks_movement, blocks_projectile), (true, false));
    let (current, _) = FreshAdmissionStore::from_root(harness.root.clone())
        .current_session_at(deps.admitted.game_session_id).await?;
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("current Item fence")?;
    let command = CommandRef::new(deps.admitted.game_session_id, CommandId::new(701).map_err(debug)?);
    let permit = owner.spell_lane_permit().await.ok_or("current spell lane")?;
    let runtime = owner.runtime.lock().await;
    let position = MovementLocalPosition { x: 1, y: 12, floor: -7 };
    assert!(!runtime.position_occupied_by_other(deps.actor, position).map_err(debug)?);
    let target = crate::durability::native_world_items::SpellGroundTarget::for_native_tile_read(
        &deps.room, &runtime, position,
    ).map_err(debug)?;
    let wall = id(180);
    // Reuse the guarded SQL-writer setup from ground_cell_limit_tests. The Spell
    // and cost are explicit fixture data, not a claim that a player cast a rune.
    // Item identity, capabilities, collision, temporal policy, scope and custody
    // all come from the actual native policy and current owners; guards remain on.
    let request = SpellItemTransactionRequest {
        swing_ordinal: None,
        command,
        spell: crate::durability::item_mint::TypedDefinitionRef {
            family: "Spell".into(),
            production_key: "fixture:monster.wall-setup".into(),
            revision_ref: "definition-r1".into(),
        },
        catalog_digest: native.source_digest(),
        transaction_id: id(181),
        event_id: id(182),
        cost: CastCostBinding {
            vitals_revision_before: 1,
            vitals_revision_after: 2,
            mana_before: 200,
            mana_after: 180,
            soul_before: 100,
            soul_after: 100,
            cooldowns_before: Vec::new(),
            cooldowns_after: vec![("spell:fixture_monster_wall".into(), 100_000)],
            caster_digest_before: [11; 32],
            caster_digest_after: [12; 32],
        },
        caster_origin: Some(SourceCasterOrigin {
            actor: deps.actor,
            character_lease_generation: fence.character_lease_generation,
        }),
        operations: vec![SpellItemOperation::MintGround {
            item_instance_id: wall,
            definition,
            quantity: 1,
            placement: target.placement(b"monster-wall-removal-setup".to_vec()),
            lifetime_millis: None,
            blocks_movement,
            blocks_projectile,
            description: None,
        }],
        companion: None,
        direct_companion: None,
    };
    let mut tx = harness.pool.begin().await?;
    let authority = assert_spell_item_authority_in_transaction(
        &mut tx, &permit, owner.root, owner.character, owner.holder,
        &fence, command, native.source_digest(),
    ).await.map_err(debug)?;
    let outcome = apply_spell_items_in_transaction(
        &mut tx, &authority, &request,
        &crate::gameplay_transport::world_required_field_guard::player_origin(&owner.required_capabilities),
    ).await.map_err(debug)?;
    assert!(matches!(outcome, SpellItemTransactionOutcome::Applied(_)));
    tx.commit().await?;
    Ok(wall)
}

async fn wall_state(harness: &Harness, wall: [u8; 16]) -> TestResult<(i16, i64, i64, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT i.lifecycle, i.quantity, i.state_revision::bigint, (SELECT count(*) FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id), (SELECT count(*) FROM game_spell_item_lines l WHERE l.item_instance_id=i.item_instance_id AND l.transaction_id=i.minted_transaction_id AND l.operation_kind=1) FROM game_item_instances i WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.definition_production_key='oteryn:item.tibia.i2130'",
    ).bind(wall.as_slice()).fetch_one(&harness.pool).await?)
}

type RemovalReceipt = (String, String, i16, i64, i64, i64, i64);

async fn removal_receipts(
    harness: &Harness,
    monster: ExactActorRef,
    wall: [u8; 16],
) -> TestResult<Vec<RemovalReceipt>> {
    Ok(sqlx::query_as(
        "SELECT r.transaction_id::text, r.creature_occurrence, i.lifecycle, i.quantity, i.state_revision::bigint, (SELECT count(*) FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id), r.operation_count::bigint FROM game_spell_item_receipts r JOIN game_spell_item_lines l ON l.transaction_id=r.transaction_id JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id AND i.last_transaction_id=r.transaction_id JOIN game_spell_item_audit_outbox a ON a.transaction_id=r.transaction_id AND a.event_id=r.event_id WHERE r.cause_kind=1 AND r.creature_placement=$1 AND r.creature_definition_key='oteryn:creature.the_monster' AND r.spell_production_key='oteryn:ability.spell.destroy_magic_walls' AND l.operation_kind=2 AND i.item_instance_id=encode($2,'hex')::uuid ORDER BY r.transaction_id",
    ).bind(monster.placement_identity().as_slice()).bind(wall.as_slice())
        .fetch_all(&harness.pool).await?)
}
