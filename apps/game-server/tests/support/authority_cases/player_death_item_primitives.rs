//! DEATH-DUR-1: real MINT/pickup/equip/death setup, then the two fenced item primitives.
//! Route-less lanes return early; routed runs exercise the complete transaction assertions.
use crate::domain::CharacterRevision;
use crate::domain::progression::{
    FiniteProgressionPolicy, LevelThreshold, ProgressionRevisionContext,
};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_death::{CharacterDeathRequest, DeathCell, PlayerDeathOccurrence};
use crate::durability::character_equipment::{
    EquipmentMove, EquipmentMoveContent, assert_equipment_authority_in_transaction,
    initialize_equipment_in_transaction,
};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::equipment_policy_abi::{
    EquipmentClaims, EquipmentPolicyLookup, QualifiedEquipmentPolicy, policy_registration,
};
use crate::durability::item_transfer::{ItemDefinitionFacts, ItemStackClass};
use crate::durability::player_death_items::{
    PlayerDeathAmuletCause, PlayerDeathDropCause, PlayerDeathItemError, PlayerDeathItemOutcome,
    PlayerDeathItemRequest, audit,
};
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{ChannelId, ConnectionGeneration, WorldId};
use crate::item_transfer_postgres_cases::{
    BACKPACK, CHANNEL, Harness, TestResult, WORLD, backpack_facts, command, configured_admin,
    debug, facts, fence, id, runtime, to_backpack, to_slot,
};
use oteryn_simulation_determinism::{ExactI64, RoundingMode};
use sqlx::Row;

const DIGEST: [u8; 32] = [1; 32];
async fn initialize(h: &Harness, authority: &ReconciledCharacterAuthority<'_, '_>) -> TestResult {
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).bind(DIGEST.as_slice()).bind([2_u8;32].as_slice()).bind([3_u8;32].as_slice()).execute(&h.pool).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let current = assert_equipment_authority_in_transaction(
        &mut tx,
        &h.root,
        authority,
        &h.node,
        &fence()?,
        command(90)?,
        DIGEST,
    )
    .await
    .map_err(debug)?;
    initialize_equipment_in_transaction(&mut tx, &current)
        .await
        .map_err(debug)?;
    tx.commit().await?;
    Ok(())
}
fn request(seed: u8) -> PlayerDeathItemRequest {
    PlayerDeathItemRequest {
        transaction_id: id(seed),
        event_id: id(seed + 1),
        occurred_at_unix_ms: 1_790_000_000_000,
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "simulation-1".into(),
        server_build_id: "death-test".into(),
    }
}
async fn give(
    h: &Harness,
    a: &ReconciledCharacterAuthority<'_, '_>,
    key: &str,
    cmd: u64,
) -> TestResult<[u8; 16]> {
    let item = h.mint(a, key, 1).await?;
    let mut definition: ItemDefinitionFacts = facts(key, ItemStackClass::NonStackable);
    // The shared qualified fixture weight catalogue includes this existing stone key.
    definition.weight = Some(1);
    h.committed(a, to_backpack(command(cmd)?, item, definition))
        .await?;
    Ok(item)
}
fn death() -> TestResult<CharacterDeathRequest<50>> {
    let context = ProgressionRevisionContext {
        profile: "profile-1".into(),
        ruleset: "ruleset-1".into(),
        content: "content-1".into(),
        simulation: "simulation-1".into(),
        evidence: "evidence-1".into(),
        declaration: "declaration-1".into(),
    };
    let mut thresholds = [LevelThreshold {
        level: 1,
        minimum_experience: ExactI64::new(0),
    }; 50];
    for (index, threshold) in thresholds.iter_mut().enumerate() {
        *threshold = LevelThreshold {
            level: u32::try_from(index + 1)?,
            minimum_experience: ExactI64::new(i64::try_from(index)? * 20),
        };
    }
    Ok(CharacterDeathRequest {
        occurrence: PlayerDeathOccurrence::from_bytes(id(170)).map_err(debug)?,
        context: context.clone(),
        policy_revision: "policy-1".into(),
        reward_revision: "reward-1".into(),
        held_blessings: Vec::new(),
        pvp_contributors: Vec::new(),
        death_cell: DeathCell {
            // W02 typed-scope extension preserves this pinned Channel fixture.
            runtime_scope: crate::foundation::RuntimeScopeRefV1::Channel {
                world_id: WorldId::decode(&id(WORLD)).map_err(debug)?,
                channel_id: ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
            },
            spatial_position: vec![12, 13, 14, 7],
            map_revision: "death-map-1".into(),
        },
        respawn_position: b"temple:thais".to_vec(),
        policy: FiniteProgressionPolicy {
            context,
            policy_revision: "policy-1".into(),
            reward_revision: "reward-1".into(),
            death_policy_revision: "death-1".into(),
            declared_difference_revision: "declaration-1".into(),
            thresholds,
            terminal_exclusive_experience: ExactI64::new(1020),
            death_loss_numerator: 1,
            death_loss_denominator: 1,
            death_loss_rounding: RoundingMode::Floor,
        },
    })
}
async fn run(admin: String) -> TestResult {
    let h = Harness::create(admin, "death_primitives").await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let a = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let backpack = h.mint(&a, BACKPACK, 1).await?;
    h.committed(&a, to_slot(command(1)?, backpack, backpack_facts()))
        .await?;
    // Existing fixture keys have a proven weight in the shared pickup catalogue.
    let armor = give(&h, &a, crate::item_transfer_postgres_cases::STONE, 2).await?;
    let necklace = give(&h, &a, crate::item_transfer_postgres_cases::STONE, 3).await?;
    let child = give(&h, &a, crate::item_transfer_postgres_cases::STONE, 4).await?;
    let entry_drop = give(&h, &a, crate::item_transfer_postgres_cases::STONE, 5).await?;
    // Only Content policy is synthetic: the actual pickup facts and every move remain real.
    initialize(&h, &a).await?;
    // Use a profile for the shared stone definition as a necklace, then armor; the primitive
    // consumes custody, while DEATH-3 will qualify the Amulet of Loss definition at selection.
    let equipped = EquipmentFixture;
    for (item, slot, cmd) in [(armor, 2, 91), (necklace, 7, 92)] {
        h.root
            .commit_equipment_move(
                &a,
                &h.node,
                &fence()?,
                command(cmd)?,
                &equipped,
                EquipmentMove {
                    item,
                    from_slot: None,
                    to_slot: Some(slot),
                },
            )
            .await?
            .map_err(debug)?;
    }
    let f = fence()?;
    let gameplay = CurrentCharacterGameplayFence {
        character_id: f.character_id,
        game_session_id: f.game_session_id,
        connection_generation: f.connection_generation,
        character_lease_generation: f.character_lease_generation,
        runtime_scope: f.runtime_scope,
        scope_ownership_generation: f.scope_ownership_generation,
        expected_character_revision: CharacterRevision::new(1).map_err(debug)?,
    };
    h.root
        .commit_character_death(&a, &h.node, gameplay, death()?)
        .await
        .map_err(|error| format!("death fixture: {error:?}"))?;
    let revision = h.character_revision().await?;
    let permit = SpellLanePermit::of_fresh_scope(f.runtime_scope).await?;
    let mut stale = f;
    stale.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
    let cause = PlayerDeathDropCause {
        death_occurrence: id(170),
        item_instance: armor,
    };
    let wrong_slot = PlayerDeathAmuletCause {
        death_occurrence: id(170),
        item_instance: armor,
    };
    assert!(matches!(
        h.root
            .commit_player_death_amulet(&permit, &a, &h.node, f, wrong_slot, request(178))
            .await,
        Err(PlayerDeathItemError::InvalidInput)
    ));
    assert_eq!(h.item_state(armor).await?, (1, 1));
    let before = h.footprint().await?;
    assert!(matches!(
        h.root
            .commit_player_death_drop(&permit, &a, &h.node, stale, cause, request(180))
            .await,
        Err(PlayerDeathItemError::AuthorityRejected)
    ));
    assert_eq!(h.footprint().await?, before);
    assert_eq!(h.count("game_player_death_item_records").await?, 0);
    assert_eq!(
        h.root
            .commit_player_death_drop(&permit, &a, &h.node, f, cause, request(180))
            .await?,
        PlayerDeathItemOutcome::Written
    );
    assert_eq!(
        h.root
            .commit_player_death_drop(&permit, &a, &h.node, f, cause, request(180))
            .await?,
        PlayerDeathItemOutcome::Replay
    );
    assert_eq!(
        h.root
            .commit_player_death_drop(&permit, &a, &h.node, stale, cause, request(180))
            .await?,
        PlayerDeathItemOutcome::Replay
    );
    let mut changed = request(180);
    changed.sim_revision.push('x');
    assert!(matches!(
        h.root
            .commit_player_death_drop(&permit, &a, &h.node, f, cause, changed)
            .await,
        Err(PlayerDeathItemError::ConflictingCause)
    ));
    let burn = PlayerDeathAmuletCause {
        death_occurrence: id(170),
        item_instance: necklace,
    };
    assert_eq!(
        h.root
            .commit_player_death_amulet(&permit, &a, &h.node, f, burn, request(182))
            .await?,
        PlayerDeathItemOutcome::Written
    );
    assert_eq!(
        h.root
            .commit_player_death_amulet(&permit, &a, &h.node, f, burn, request(182))
            .await?,
        PlayerDeathItemOutcome::Replay
    );
    assert!(matches!(
        h.root
            .commit_player_death_amulet(&permit, &a, &h.node, f, burn, request(184))
            .await,
        Err(PlayerDeathItemError::ConflictingCause)
    ));
    assert_eq!(h.item_state(necklace).await?, (0, 2));
    let entry = PlayerDeathDropCause {
        death_occurrence: id(170),
        item_instance: entry_drop,
    };
    h.root
        .commit_player_death_drop(&permit, &a, &h.node, f, entry, request(186))
        .await?;
    let children:Vec<(String,String)>=sqlx::query_as("SELECT item_instance_id::text,placement_ordinal::text FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid ORDER BY placement_ordinal").bind(backpack.as_slice()).fetch_all(&h.pool).await?;
    let container = PlayerDeathDropCause {
        death_occurrence: id(170),
        item_instance: backpack,
    };
    h.root
        .commit_player_death_drop(&permit, &a, &h.node, f, container, request(188))
        .await?;
    assert_eq!(
        h.root
            .commit_player_death_drop(&permit, &a, &h.node, f, container, request(188))
            .await?,
        PlayerDeathItemOutcome::Replay
    );
    let after:Vec<(String,String)>=sqlx::query_as("SELECT item_instance_id::text,placement_ordinal::text FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid ORDER BY placement_ordinal").bind(backpack.as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(children, after);
    let owner: Option<String> = sqlx::query_scalar(
        "SELECT owner_character::text FROM game_item_loop_owner(encode($1,'hex')::uuid)",
    )
    .bind(child.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(owner, None);
    let carried: i64 =
        sqlx::query_scalar("SELECT game_item_loop_tree_size(encode($1,'hex')::uuid)")
            .bind(id(crate::item_transfer_postgres_cases::CHARACTER).as_slice())
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(carried, 0);
    assert_eq!(after.len(), 1);
    assert_eq!(
        after[0].0,
        crate::item_transfer_postgres_cases::uuid_text(child)
    );
    assert_eq!(h.character_revision().await?, revision);
    for item in [armor, entry_drop, backpack] {
        let ground=sqlx::query("SELECT spatial_position,map_revision FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid").bind(item.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            ground.try_get::<Vec<u8>, _>("spatial_position")?,
            vec![12, 13, 14, 7]
        );
        assert_eq!(ground.try_get::<String, _>("map_revision")?, "death-map-1");
    }
    let rows=sqlx::query("SELECT envelope FROM game_item_audit_outbox WHERE transaction_id IN(SELECT transaction_id FROM game_player_death_item_records) ORDER BY event_id").fetch_all(&h.pool).await?;
    assert_eq!(rows.len(), 4);
    for row in rows {
        let wire: Vec<u8> = row.try_get("envelope")?;
        let (_, is_burn, event) = audit::decode(&wire).map_err(debug)?;
        if is_burn {
            assert_eq!(event.typed_sink, audit::AMULET_SINK);
            assert_eq!(event.before.ok_or("before")?.quantity, 1);
            assert_eq!(event.after.ok_or("after")?.quantity, 0);
        }
    }
    let before = h.footprint().await?;
    let mut raw = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *raw)
        .await?;
    let update=sqlx::query("UPDATE game_item_instances SET quantity=0,lifecycle=2,last_transaction_id=encode($1,'hex')::uuid WHERE item_instance_id=encode($2,'hex')::uuid").bind(id(199).as_slice()).bind(child.as_slice()).execute(&mut *raw).await;
    let rejected = match update {
        Ok(_) => raw.commit().await,
        Err(error) => {
            raw.rollback().await?;
            Err(error)
        }
    };
    let error = rejected.err().ok_or("unproven raw UPDATE committed")?;
    assert_eq!(
        error.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("23514")
    );
    assert_eq!(h.footprint().await?, before);
    assert_eq!(h.item_state(child).await?, (1, 1));
    for burn in [false, true] {
        let max = if burn {
            audit::AMULET_LIMITS
        } else {
            audit::DROP_LIMITS
        };
        assert!(audit::check_usage(burn, max).is_ok());
        for index in 0..max.len() {
            let mut over = max;
            over[index] += 1;
            assert!(audit::check_usage(burn, over).is_err());
        }
    }
    drop(a);
    drop(seal);
    h.cleanup().await?;
    eprintln!(
        "player_death_item_primitives: routed PostgreSQL assertions completed (drop, container, burn, replay, conflict, fence, raw guard, resource boundaries)"
    );
    Ok(())
}
// Qualified synthetic equipment policy admits only the shared nonstackable fixture stone;
// this keeps the real pickup's existing content weight proof intact.
struct EquipmentFixture;
impl policy_registration::Registered for EquipmentFixture {}
impl EquipmentPolicyLookup for EquipmentFixture {
    fn source_digest(&self) -> [u8; 32] {
        DIGEST
    }
    fn equipment_policy(&self, key: &str, revision: &str) -> Option<QualifiedEquipmentPolicy> {
        if ![BACKPACK, crate::item_transfer_postgres_cases::STONE].contains(&key)
            || revision != "definition-r1"
        {
            return None;
        }
        QualifiedEquipmentPolicy::from_qualified_content(
            DIGEST,
            [1, 2, 3, 4, 5, 6, 7, 8, 10]
                .into_iter()
                .map(|slot| {
                    (
                        slot,
                        if key == BACKPACK {
                            Err("backpack is not a non-container slot item")
                        } else {
                            Ok(EquipmentClaims {
                                slots: vec![slot],
                                groups: Vec::new(),
                                level: 0,
                                vocations: Vec::new(),
                            })
                        },
                    )
                })
                .collect(),
            (key == BACKPACK).then_some(20),
        )
    }
}
impl EquipmentMoveContent for EquipmentFixture {
    fn is_plain_item(&self, key: &str, _: &str) -> bool {
        key != BACKPACK
    }
    fn is_timed_item(&self, _: &str, _: &str) -> bool {
        false
    }
    fn equipment_stack_class(&self, _: &str, _: &str) -> ItemStackClass {
        ItemStackClass::NonStackable
    }
}
#[test]
fn player_death_item_primitives() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime()
                .map_err(debug)?
                .block_on(run(admin))
                .map_err(debug)
        })?
        .join()
        .map_err(|_| "death primitive test thread panicked")?
        .map_err(Into::into)
}

struct PickupContent;
impl crate::durability::item_transfer::ItemWeightLookup for PickupContent {
    fn generation_digest(&self) -> [u8; 32] {
        [0xd1; 32]
    }
    fn weight_of(
        &self,
        definition: &crate::durability::item_mint::TypedDefinitionRef,
    ) -> Result<u32, crate::durability::item_transfer::ItemWeightRefused> {
        crate::durability::item_transfer::ItemWeightLookup::weight_of(
            &crate::durability::item_weight_lookup::TEST_WEIGHTS,
            definition,
        )
    }
}
impl crate::durability::item_loop_move::ItemLoopContent for PickupContent {
    fn loop_content_digest(&self) -> [u8; 32] {
        DIGEST
    }
    fn loop_facts(
        &self,
        definition: &crate::durability::item_mint::TypedDefinitionRef,
    ) -> Option<ItemDefinitionFacts> {
        let value = match definition.production_key.as_str() {
            BACKPACK => backpack_facts(),
            crate::item_transfer_postgres_cases::STONE => {
                let mut value = facts(
                    crate::item_transfer_postgres_cases::STONE,
                    ItemStackClass::NonStackable,
                );
                value.weight = Some(1);
                value
            }
            _ => return None,
        };
        (value.definition == *definition).then_some(value)
    }
}

async fn run_death_ground_regression(admin: String, reset: bool) -> TestResult {
    use crate::durability::item_loop_move::{LoopDestination, LoopMoveRequest};
    use crate::durability::runtime_scope_assignment::{
        AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
        RuntimeScopeAssignmentWriter,
    };
    use crate::item_transfer_postgres_cases::scope;
    use oteryn_gameplay_durability::durability::world_reset::test_support as retiring_fixture;

    let h = Harness::create(admin, if reset { "death_reset" } else { "death_pickup" }).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let a = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let backpack = h.mint(&a, BACKPACK, 1).await?;
    h.committed(&a, to_slot(command(1)?, backpack, backpack_facts()))
        .await?;
    let child = give(&h, &a, crate::item_transfer_postgres_cases::STONE, 2).await?;
    initialize(&h, &a).await?;
    let f = fence()?;
    let mut death = death()?;
    let cell = vec![0, 0, 0, 12, 0, 0, 0, 13, 0, 7];
    death.death_cell.spatial_position = cell.clone();
    h.root
        .commit_character_death(
            &a,
            &h.node,
            CurrentCharacterGameplayFence {
                character_id: f.character_id,
                game_session_id: f.game_session_id,
                connection_generation: f.connection_generation,
                character_lease_generation: f.character_lease_generation,
                runtime_scope: f.runtime_scope,
                scope_ownership_generation: f.scope_ownership_generation,
                expected_character_revision: CharacterRevision::new(1).map_err(debug)?,
            },
            death,
        )
        .await
        .map_err(debug)?;
    let permit = SpellLanePermit::of_fresh_scope(f.runtime_scope).await?;
    let dropped = if reset { backpack } else { child };
    assert_eq!(
        h.root
            .commit_player_death_drop(
                &permit,
                &a,
                &h.node,
                f,
                PlayerDeathDropCause {
                    death_occurrence: id(170),
                    item_instance: dropped
                },
                request(180)
            )
            .await?,
        PlayerDeathItemOutcome::Written
    );
    assert_eq!(h.count("game_item_ground_locations").await?, 1);
    if reset {
        let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
        retiring_fixture::seed_retiring(&h.pool, world).await?;
        let reset = h.root.retiring_world_reset(world).await?.ok_or("reset")?;
        sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3")
            .execute(&h.pool)
            .await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,2)")
            .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).execute(&h.pool).await?;
        let writer = RuntimeScopeAssignmentWriter::open(h.root.clone(), "item-transfer-writer")
            .await
            .map_err(debug)?;
        let predecessor = h
            .root
            .read_runtime_scope_predecessor(scope()?)
            .await
            .map_err(debug)?
            .ok_or("predecessor")?;
        let outcome = writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([9; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                command: AssignmentCommand::Replace {
                    scope: scope()?,
                    predecessor,
                    target: h.node.fact(),
                },
            })
            .await
            .map_err(debug)?;
        assert!(matches!(outcome, AssignmentOutcome::Committed(_)));
        assert!(
            h.root
                .retire_world_reset_ground(&a, &h.node, &reset)
                .await
                .map_err(debug)?
        );
        assert!(
            h.root
                .retire_world_reset_ground(&a, &h.node, &reset)
                .await
                .map_err(debug)?
        );
        assert_eq!(h.count("game_item_ground_locations").await?, 0);
        assert_eq!(h.count("game_item_container_entries").await?, 0);
        assert_eq!(h.count("game_item_decay_retire_receipts").await?, 2);
        let (transactions, audited): (i64, i64) = sqlx::query_as("SELECT count(DISTINCT r.created_xact_id),count(a.event_id) FROM game_item_decay_retire_receipts r JOIN game_item_audit_outbox a ON a.item_instance_id=r.item_instance_id AND a.transaction_id=r.transaction_id AND a.event_id=r.event_id AND a.envelope_sha256=r.envelope_sha256 AND a.created_xact_id=r.created_xact_id WHERE r.cause_kind='WORLD_RESET' AND r.ground_root_item_instance_id=encode($1,'hex')::uuid")
            .bind(backpack.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!((transactions, audited), (1, 2));
        for item in [backpack, child] {
            assert_eq!(h.item_state(item).await?, (0, 2));
        }
    } else {
        let receipt = h
            .root
            .commit_item_loop_move(
                &permit,
                &a,
                &h.node,
                &f,
                command(200)?,
                &PickupContent,
                LoopMoveRequest {
                    item: child,
                    count: None,
                    pickup_cell: Some(cell),
                    destination: LoopDestination::Container(backpack),
                },
            )
            .await?
            .map_err(debug)?;
        assert!(!receipt.replayed);
        assert_eq!(receipt.operation, 5);
        assert_eq!(h.count("game_item_ground_locations").await?, 0);
        let placed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid AND parent_item_instance_id=encode($2,'hex')::uuid)")
            .bind(child.as_slice()).bind(backpack.as_slice()).fetch_one(&h.pool).await?;
        assert!(placed);
        assert_eq!(h.item_state(child).await?, (1, 1));
        assert_eq!(h.count("game_player_death_item_records").await?, 1);
    }
    drop(permit);
    drop(a);
    drop(seal);
    h.cleanup().await
}

fn death_ground_case(reset: bool) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime()
                .map_err(debug)?
                .block_on(run_death_ground_regression(admin, reset))
                .map_err(debug)
        })?
        .join()
        .map_err(|_| "death ground test thread panicked")?
        .map_err(Into::into)
}

#[test]
fn player_death_ground_drop_can_be_picked_up_by_item_loop() -> TestResult {
    death_ground_case(false)
}

#[test]
fn player_death_ground_backpack_and_contents_retire_on_world_reset() -> TestResult {
    death_ground_case(true)
}
