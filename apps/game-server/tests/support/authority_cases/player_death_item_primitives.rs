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
    initialize_for(h, authority, &fence()?, command(90)?).await
}

async fn initialize_for(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    current_fence: &crate::durability::item_transfer::CurrentCharacterItemFence,
    current_command: crate::foundation::CommandRef,
) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let current = assert_equipment_authority_in_transaction(
        &mut tx,
        &h.root,
        authority,
        &h.node,
        current_fence,
        current_command,
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
        public_cause: None,
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
    let after:Vec<(String,String)>=sqlx::query_as("SELECT item_instance_id::text,placement_ordinal::text FROM game_item_nested_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid ORDER BY placement_ordinal").bind(backpack.as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(h.count("game_item_container_entries").await?, 0);
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

const DEATH_QUIVER: &str = "oteryn:item.tibia.i35562";
const DEATH_ARROW: &str = "oteryn:item.tibia.i3447";

/// Test-only qualified Content: the ordinary quiver has six entries, accepts
/// ammunition and can occupy left-hand/extra-ammunition slots. All persistence
/// below still uses the real mint, transfer, loop and equipment owners.
struct QuiverDeathContent;
impl policy_registration::Registered for QuiverDeathContent {}

impl crate::durability::item_transfer::ItemWeightLookup for QuiverDeathContent {
    fn generation_digest(&self) -> [u8; 32] {
        [0xd1; 32]
    }
    fn weight_of(
        &self,
        definition: &crate::durability::item_mint::TypedDefinitionRef,
    ) -> Result<u32, crate::durability::item_transfer::ItemWeightRefused> {
        if definition.family == "Item"
            && definition.revision_ref == "definition-r1"
            && [DEATH_QUIVER, DEATH_ARROW].contains(&definition.production_key.as_str())
        {
            Ok(1)
        } else {
            crate::durability::item_transfer::ItemWeightLookup::weight_of(
                &PickupContent,
                definition,
            )
        }
    }
}

impl crate::durability::item_loop_move::ItemLoopContent for QuiverDeathContent {
    fn loop_content_digest(&self) -> [u8; 32] {
        DIGEST
    }
    fn loop_facts(
        &self,
        definition: &crate::durability::item_mint::TypedDefinitionRef,
    ) -> Option<ItemDefinitionFacts> {
        let mut value = match definition.production_key.as_str() {
            DEATH_QUIVER => {
                let mut value = facts(DEATH_QUIVER, ItemStackClass::NonStackable);
                value.container_capacity = Some(6);
                value
            }
            DEATH_ARROW => facts(
                DEATH_ARROW,
                ItemStackClass::Stackable {
                    proven_maximum: Some(100),
                },
            ),
            _ => {
                return crate::durability::item_loop_move::ItemLoopContent::loop_facts(
                    &PickupContent,
                    definition,
                );
            }
        };
        value.weight = Some(1);
        (value.definition == *definition).then_some(value)
    }
    fn is_quiver(&self, definition: &crate::durability::item_mint::TypedDefinitionRef) -> bool {
        definition.family == "Item"
            && self.is_quiver_item(&definition.production_key, &definition.revision_ref)
    }
    fn is_ammunition(&self, definition: &crate::durability::item_mint::TypedDefinitionRef) -> bool {
        definition.family == "Item"
            && self.is_ammunition_item(&definition.production_key, &definition.revision_ref)
    }
}

impl EquipmentPolicyLookup for QuiverDeathContent {
    fn source_digest(&self) -> [u8; 32] {
        DIGEST
    }
    fn equipment_policy(&self, key: &str, revision: &str) -> Option<QualifiedEquipmentPolicy> {
        if !self.is_quiver_item(key, revision) {
            return EquipmentPolicyLookup::equipment_policy(&EquipmentFixture, key, revision);
        }
        QualifiedEquipmentPolicy::from_qualified_content(
            DIGEST,
            [1, 2, 3, 4, 5, 6, 7, 8, 10]
                .into_iter()
                .map(|slot| {
                    (
                        slot,
                        if matches!(slot, 6 | 10) {
                            Ok(EquipmentClaims {
                                slots: vec![slot],
                                groups: Vec::new(),
                                level: 0,
                                vocations: Vec::new(),
                            })
                        } else {
                            Err("quiver requires a supported equipment slot")
                        },
                    )
                })
                .collect(),
            Some(6),
        )
    }
}

impl EquipmentMoveContent for QuiverDeathContent {
    fn is_plain_item(&self, key: &str, _: &str) -> bool {
        key != DEATH_QUIVER && key != BACKPACK
    }
    fn is_quiver_item(&self, key: &str, revision: &str) -> bool {
        key == DEATH_QUIVER && revision == "definition-r1"
    }
    fn is_ammunition_item(&self, key: &str, revision: &str) -> bool {
        key == DEATH_ARROW && revision == "definition-r1"
    }
    fn is_timed_item(&self, _: &str, _: &str) -> bool {
        false
    }
    fn equipment_stack_class(&self, key: &str, _: &str) -> ItemStackClass {
        if key == DEATH_ARROW {
            ItemStackClass::Stackable {
                proven_maximum: Some(100),
            }
        } else {
            ItemStackClass::NonStackable
        }
    }
}

async fn pickup_quiver_fixture_item(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    key: &str,
    command_id: u64,
) -> TestResult<[u8; 16]> {
    use crate::durability::item_loop_move::ItemLoopContent;
    use crate::durability::item_transfer::ItemTransferOutcome;
    let content = QuiverDeathContent;
    let item = h.mint(authority, key, 1).await?;
    let definition = facts(key, ItemStackClass::NonStackable).definition;
    let facts = content
        .loop_facts(&definition)
        .ok_or("qualified quiver fixture facts")?;
    let f = fence()?;
    let mut candidate = h
        .root
        .freeze_item_transfer(
            authority,
            &h.node,
            f,
            to_backpack(command(command_id)?, item, facts),
            &content,
        )
        .await
        .map_err(debug)?;
    let permit = SpellLanePermit::of_fresh_scope(f.runtime_scope).await?;
    assert!(matches!(
        h.root
            .commit_item_transfer(&permit, authority, &h.node, f, &mut candidate, &content)
            .await
            .map_err(debug)?,
        ItemTransferOutcome::Committed(_)
    ));
    Ok(item)
}

fn quiver_loss_occurrence(
    main: [u8; 16],
    quiver: [u8; 16],
    slot: u8,
) -> TestResult<PlayerDeathOccurrence> {
    use crate::domain::death::{EquipmentSlot, EquippedItem, PveDeathInput, calculate_pve_death};
    use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
    let equipment = [
        EquippedItem {
            slot: EquipmentSlot::Container,
            item: main,
        },
        EquippedItem {
            slot: if slot == 6 {
                EquipmentSlot::LeftHand
            } else {
                EquipmentSlot::Ammo
            },
            item: quiver,
        },
    ];
    // Choose a bounded deterministic occurrence with the actual native sampler.
    // The real death transaction independently reloads and selects its equipment.
    for nonce in 0_u16..1024 {
        let mut occurrence = id(170);
        occurrence[14..].copy_from_slice(&nonce.to_be_bytes());
        let selected = calculate_pve_death(
            &PveDeathInput {
                level: 50,
                total_experience: ExactI64::new(0),
                regular_blessings: 0,
                promotion_benefit_current: false,
                has_vocation: true,
                equipment: &equipment,
                necklace_is_amulet_of_loss: false,
            },
            &GameplayDecisionRoot::from_bytes(DIGEST),
            DecisionOccurrenceId::from_bytes(occurrence),
        )
        .map_err(debug)?;
        if selected.lost_items.iter().any(|item| item.item == quiver) {
            assert!(selected.lost_items.iter().any(|item| item.item == main));
            return PlayerDeathOccurrence::from_bytes(occurrence)
                .map_err(|error| debug(error).into());
        }
    }
    Err("no quiver-loss occurrence in the bounded native sampler fixture".into())
}

async fn nested_carried_tree(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult<Vec<[u8; 16]>> {
    let depth = crate::durability::item_loop_move::ITEM_LOOP_DEPTH_MAX;
    assert_eq!(depth, 8);
    carried_chain(h, authority, depth).await
}

async fn carried_chain(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    depth_limit: i32,
) -> TestResult<Vec<[u8; 16]>> {
    use crate::durability::item_loop_move::{
        ITEM_LOOP_DEPTH_MAX, LoopDestination, LoopMoveRequest,
    };

    assert!((2..=ITEM_LOOP_DEPTH_MAX).contains(&depth_limit));
    let backpack = h.mint(authority, BACKPACK, 1).await?;
    h.committed(authority, to_slot(command(1)?, backpack, backpack_facts()))
        .await?;
    initialize(h, authority).await?;
    let mut items = vec![backpack];
    // Every edge is admitted by its normal pickup or loop-move owner.
    for depth in 1..=depth_limit {
        let leaf = depth == depth_limit;
        let item = h
            .mint(
                authority,
                if leaf {
                    crate::item_transfer_postgres_cases::STONE
                } else {
                    BACKPACK
                },
                1,
            )
            .await?;
        let definition = if leaf {
            facts(
                crate::item_transfer_postgres_cases::STONE,
                ItemStackClass::NonStackable,
            )
        } else {
            backpack_facts()
        };
        h.committed(
            authority,
            to_backpack(command(10 + depth as u64)?, item, definition),
        )
        .await?;
        if depth > 1 {
            let f = fence()?;
            let permit = SpellLanePermit::of_fresh_scope(f.runtime_scope).await?;
            let moved = h
                .root
                .commit_item_loop_move(
                    &permit,
                    authority,
                    &h.node,
                    &f,
                    command(30 + depth as u64)?,
                    &PickupContent,
                    LoopMoveRequest {
                        item,
                        count: None,
                        pickup_cell: None,
                        destination: LoopDestination::Container(*items.last().ok_or("parent bag")?),
                    },
                )
                .await?
                .map_err(debug)?;
            assert!(!moved.replayed);
            assert_eq!(moved.operation, 1);
        }
        let owner: (Option<String>, Option<i32>) = sqlx::query_as(
            "SELECT owner_character::text,depth FROM game_item_loop_owner(encode($1,'hex')::uuid)",
        )
        .bind(item.as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            owner,
            (
                Some(crate::item_transfer_postgres_cases::uuid_text(id(
                    crate::item_transfer_postgres_cases::CHARACTER
                ))),
                Some(depth)
            )
        );
        items.push(item);
    }
    assert_eq!(items.len(), usize::try_from(depth_limit)? + 1);
    Ok(items)
}

#[derive(Debug, PartialEq, Eq)]
struct NestedDeathSnapshot {
    items: Vec<String>,
    placements: Vec<(String, String)>,
}

/// ItemInstance rows stay byte-identical. Custody compares the complete logical
/// edge: child, world, parent, ordinal and placement transaction. Death may move
/// a main backpack's direct entries into the owner-independent nested table;
/// the former Character projection is not part of that logical edge.
async fn nested_death_snapshot(h: &Harness, selected: [u8; 16]) -> TestResult<NestedDeathSnapshot> {
    let items = sqlx::query_scalar("SELECT to_jsonb(i)::text FROM game_item_instances i WHERE item_instance_id<>encode($1,'hex')::uuid ORDER BY item_instance_id LIMIT 502")
        .bind(selected.as_slice()).fetch_all(&h.pool).await?;
    let placements = sqlx::query_as("SELECT 'slot'::text AS kind,to_jsonb(s)::text AS data FROM game_item_container_slots s WHERE item_instance_id<>encode($1,'hex')::uuid UNION ALL SELECT 'entry',(to_jsonb(e)-'character_id')::text FROM game_item_container_entries e WHERE item_instance_id<>encode($1,'hex')::uuid UNION ALL SELECT 'entry',to_jsonb(n)::text FROM game_item_nested_entries n WHERE item_instance_id<>encode($1,'hex')::uuid ORDER BY 1,2 LIMIT 502")
        .bind(selected.as_slice()).fetch_all(&h.pool).await?;
    Ok(NestedDeathSnapshot { items, placements })
}

async fn run_nested_death_regression(admin: String, direct_bag: bool, reset: bool) -> TestResult {
    use crate::durability::character_build::{
        BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
    };
    use crate::durability::character_progression::CharacterProgressionError;
    use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
    use crate::durability::{DurabilityError, DurabilityRoot, SqlxFailureClass};
    use oteryn_gameplay_spell::spell::mana_training::CompiledTrainingFormula;

    let h = Harness::create(
        admin,
        if reset && direct_bag {
            "death_nested_bag_reset"
        } else if reset {
            "death_nested_selected_reset"
        } else if direct_bag {
            "death_nested_bag"
        } else {
            "death_nested_selected"
        },
    )
    .await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let items = nested_carried_tree(&h, &authority).await?;
    let selected = items[usize::from(direct_bag)];
    let before = nested_death_snapshot(&h, selected).await?;
    assert_eq!(before.items.len(), 8);
    assert_eq!(before.placements.len(), 8);
    let f = fence()?;
    let mut gameplay = CurrentCharacterGameplayFence {
        character_id: f.character_id,
        game_session_id: f.game_session_id,
        connection_generation: f.connection_generation,
        character_lease_generation: f.character_lease_generation,
        runtime_scope: f.runtime_scope,
        scope_ownership_generation: f.scope_ownership_generation,
        expected_character_revision: CharacterRevision::new(1).map_err(debug)?,
    };
    let formula = CompiledTrainingFormula::from_profile(
        include_bytes!("../../../../../tools/content-schema/native-gameplay/build-training.json"),
        "content-1",
    )
    .map_err(debug)?;
    let build = DurableBuildState::default();
    h.root
        .commit_character_build(
            &authority,
            &h.node,
            gameplay,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(160)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                after: convert_vocation(&formula, &build, "druid").map_err(debug)?,
                before: build,
                pruned_stance: None,
            },
            &formula,
        )
        .await
        .map_err(debug)?;
    gameplay.expected_character_revision =
        CharacterRevision::new(h.character_revision().await?.parse()?).map_err(debug)?;
    let sequencer = CharacterRevisionSequencer::new();
    let mut intent = death()?;
    intent.death_cell.spatial_position = vec![0, 0, 0, 12, 0, 0, 0, 13, 0, 7];
    // The normal death selects the equipped backpack. The sibling direct-entry
    // primitive also remains valid for historical receipts without a selection.
    let committed = h
        .root
        .settle_runtime_player_death_with_items(
            &sequencer,
            &authority,
            &h.node,
            gameplay,
            intent,
            Some(&formula),
            None,
            (!direct_bag).then_some(DIGEST),
        )
        .await
        .map_err(debug)?;
    let pending_fence = CurrentCharacterGameplayFence {
        expected_character_revision: committed.committed_character_revision,
        ..gameplay
    };
    let planned = if direct_bag {
        assert!(committed.items.is_none());
        request(180)
    } else {
        let selection = committed
            .items
            .as_ref()
            .ok_or("persisted death selection")?;
        assert_eq!(selection.root, DIGEST);
        assert_eq!(selection.amulet, None);
        assert_eq!(selection.lost, vec![items[0]]);
        assert!(
            h.root
                .pending_respawn(&authority, &h.node, pending_fence)
                .await
                .map_err(debug)?
                .is_some()
        );
        assert!(matches!(
            h.root
                .consume_pending_respawn(
                    &authority,
                    &h.node,
                    pending_fence,
                    Some(committed.occurrence)
                )
                .await,
            Err(CharacterProgressionError::Unavailable(
                DurabilityError::Database(SqlxFailureClass::Database(_))
            ))
        ));
        assert_eq!(h.count("game_character_pending_respawns").await?, 1);
        PlayerDeathItemRequest::selected(&committed, selected, false)?
    };
    assert_eq!(h.count("game_player_death_item_records").await?, 0);
    drop(authority);
    drop(seal);

    // Reopen the real durable root and reconcile independently before resuming
    // the saved selection; no actor or in-memory request supplies new authority.
    let reopened = DurabilityRoot::connect_test_runtime(&h.database.url)?;
    assert!(reopened.maintain_ready_once().await?);
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = reopened
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let retained = reopened
        .reconcile_character_death(&authority, committed.occurrence)
        .await
        .map_err(debug)?
        .ok_or("retained death")?;
    assert_eq!(retained, committed);
    if !direct_bag {
        assert_eq!(
            PlayerDeathItemRequest::selected(&retained, selected, false)?,
            planned
        );
    }
    let cause = PlayerDeathDropCause {
        death_occurrence: *retained.occurrence.as_bytes(),
        item_instance: selected,
    };
    let permit = SpellLanePermit::of_fresh_scope(f.runtime_scope).await?;
    let audits_before = h.count("game_item_audit_outbox").await?;
    assert_eq!(
        reopened
            .commit_player_death_drop(&permit, &authority, &h.node, f, cause, planned.clone())
            .await?,
        PlayerDeathItemOutcome::Written
    );
    assert_eq!(nested_death_snapshot(&h, selected).await?, before);
    assert_eq!(h.count("game_item_container_entries").await?, 0);
    assert_eq!(
        h.count("game_item_nested_entries").await?,
        if direct_bag { 7 } else { 8 }
    );
    assert!(h.on_ground(selected).await?);
    let written: String = sqlx::query_scalar("SELECT to_jsonb(i)::text FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(selected.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        reopened
            .commit_player_death_drop(&permit, &authority, &h.node, f, cause, planned)
            .await?,
        PlayerDeathItemOutcome::Replay
    );
    let replayed: String = sqlx::query_scalar("SELECT to_jsonb(i)::text FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(selected.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(written, replayed);
    assert_eq!(nested_death_snapshot(&h, selected).await?, before);
    assert_eq!(h.count("game_player_death_item_records").await?, 1);
    assert_eq!(h.count("game_item_audit_outbox").await?, audits_before + 1);
    for item in &items[usize::from(direct_bag)..] {
        let owner: Option<String> = sqlx::query_scalar(
            "SELECT owner_character::text FROM game_item_loop_owner(encode($1,'hex')::uuid)",
        )
        .bind(item.as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(owner, None);
        assert_eq!(h.item_state(*item).await?, (1, 1));
    }
    let consumed = reopened
        .consume_pending_respawn(
            &authority,
            &h.node,
            pending_fence,
            Some(retained.occurrence),
        )
        .await
        .map_err(debug)?
        .ok_or("terminal respawn")?;
    assert_eq!(consumed.occurrence, retained.occurrence);
    assert_eq!(consumed.respawn_position, retained.respawn_position);
    assert!(
        reopened
            .consume_pending_respawn(
                &authority,
                &h.node,
                pending_fence,
                Some(retained.occurrence)
            )
            .await
            .map_err(debug)?
            .is_none()
    );
    drop(permit);
    if reset {
        retire_nested_death_ground(&h, &reopened, &authority, &items, direct_bag).await?;
    }
    assert_eq!(
        h.character_revision().await?,
        retained.committed_character_revision.get().to_string()
    );
    drop(authority);
    drop(seal);
    drop(reopened);
    h.cleanup().await
}

fn nested_death_case(direct_bag: bool, reset: bool) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime()
                .map_err(debug)?
                .block_on(run_nested_death_regression(admin, direct_bag, reset))
                .map_err(debug)
        })?
        .join()
        .map_err(|_| "nested death test thread panicked")?
        .map_err(Into::into)
}

#[test]
fn player_death_nested_selected_backpack_depth_eight_survives_restart() -> TestResult {
    nested_death_case(false, false)
}

#[test]
fn player_death_nested_direct_bag_preserves_descendants() -> TestResult {
    nested_death_case(true, false)
}

#[test]
fn player_death_nested_selected_backpack_reset_is_atomic_and_postordered() -> TestResult {
    nested_death_case(false, true)
}

#[test]
fn player_death_nested_direct_bag_reset_preserves_main_backpack() -> TestResult {
    nested_death_case(true, true)
}

async fn prepare_nested_reset(
    h: &Harness,
    root: &crate::durability::DurabilityRoot,
) -> TestResult<crate::durability::world_reset::RetiringWorldReset> {
    use crate::durability::runtime_scope_assignment::{
        AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
        RuntimeScopeAssignmentWriter,
    };
    use crate::item_transfer_postgres_cases::scope;
    use oteryn_gameplay_durability::durability::world_reset::test_support as retiring_fixture;

    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
    retiring_fixture::seed_retiring(&h.pool, world).await?;
    let reset = root
        .retiring_world_reset(world)
        .await?
        .ok_or("nested reset")?;
    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3")
        .execute(&h.pool)
        .await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,2)")
        .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).execute(&h.pool).await?;
    let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "item-transfer-writer")
        .await
        .map_err(debug)?;
    let predecessor = root
        .read_runtime_scope_predecessor(scope()?)
        .await
        .map_err(debug)?
        .ok_or("reset predecessor")?;
    assert!(matches!(
        writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([9; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                command: AssignmentCommand::Replace {
                    scope: scope()?,
                    predecessor,
                    target: h.node.fact()
                },
            })
            .await
            .map_err(debug)?,
        AssignmentOutcome::Committed(_)
    ));
    Ok(reset)
}

async fn observe_nested_retirement_order(h: &Harness) -> TestResult {
    sqlx::raw_sql("CREATE TABLE test_nested_retire_order(position BIGSERIAL PRIMARY KEY,item UUID NOT NULL);
        GRANT INSERT ON test_nested_retire_order TO oteryn_game_runtime;
        GRANT USAGE ON SEQUENCE test_nested_retire_order_position_seq TO oteryn_game_runtime;
        CREATE FUNCTION test_log_nested_retire() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
          INSERT INTO test_nested_retire_order(item) VALUES(NEW.item_instance_id); RETURN NEW; END $$;
        CREATE TRIGGER a_test_nested_retire_order AFTER INSERT ON game_item_decay_retire_receipts
          FOR EACH ROW WHEN(NEW.cause_kind='WORLD_RESET') EXECUTE FUNCTION test_log_nested_retire();")
        .execute(&h.pool).await?;
    Ok(())
}

async fn retire_nested_death_ground(
    h: &Harness,
    root: &crate::durability::DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    items: &[[u8; 16]],
    direct_bag: bool,
) -> TestResult {
    use crate::durability::item_decay_retire::DecayRetireError;
    use crate::durability::{DurabilityError, SqlxFailureClass};
    use crate::item_transfer_postgres_cases::uuid_text;

    let selected = items[usize::from(direct_bag)];
    let reset = prepare_nested_reset(h, root).await?;
    let before = nested_death_snapshot(h, selected).await?;
    let root_before: String = sqlx::query_scalar("SELECT to_jsonb(i)::text FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(selected.as_slice()).fetch_one(&h.pool).await?;
    let ground_before: String = sqlx::query_scalar("SELECT to_jsonb(g)::text FROM game_item_ground_locations g WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(selected.as_slice()).fetch_one(&h.pool).await?;
    let audits_before = h.count("game_item_audit_outbox").await?;
    // Observe the actual receipt insertion order. Reject the final root receipt
    // once, after all descendants were processed, without corrupting the graph.
    observe_nested_retirement_order(h).await?;
    sqlx::raw_sql("CREATE SEQUENCE test_nested_retire_root_reached_seq;
        GRANT USAGE ON SEQUENCE test_nested_retire_root_reached_seq TO oteryn_game_runtime;
        CREATE FUNCTION test_fail_nested_retire_root() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
          PERFORM nextval('test_nested_retire_root_reached_seq');
          RAISE EXCEPTION 'injected failure after nested descendants' USING ERRCODE='23514'; END $$;
        CREATE TRIGGER z_test_nested_retire_root AFTER INSERT ON game_item_decay_retire_receipts
          FOR EACH ROW WHEN(NEW.cause_kind='WORLD_RESET' AND NEW.item_instance_id=NEW.ground_root_item_instance_id)
          EXECUTE FUNCTION test_fail_nested_retire_root();")
        .execute(&h.pool).await?;
    assert!(matches!(
        root.retire_world_reset_ground(authority, &h.node, &reset)
            .await,
        Err(DecayRetireError::Unavailable(DurabilityError::Database(
            SqlxFailureClass::Database(None)
        )))
    ));
    // The owner keeps only registered OT* SQLSTATEs; 23514 maps to None.
    // Sequence increments survive rollback and prove this exact failure
    // trigger ran once, after every descendant and the root receipt insert.
    let reached: (i64, bool) =
        sqlx::query_as("SELECT last_value,is_called FROM test_nested_retire_root_reached_seq")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(reached, (1, true));
    let inserted: (i64, bool) =
        sqlx::query_as("SELECT last_value,is_called FROM test_nested_retire_order_position_seq")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        inserted,
        (i64::try_from(items.len() - usize::from(direct_bag))?, true)
    );
    assert_eq!(nested_death_snapshot(h, selected).await?, before);
    let root_after: String = sqlx::query_scalar("SELECT to_jsonb(i)::text FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(selected.as_slice()).fetch_one(&h.pool).await?;
    let ground_after: String = sqlx::query_scalar("SELECT to_jsonb(g)::text FROM game_item_ground_locations g WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(selected.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(root_after, root_before);
    assert_eq!(ground_after, ground_before);
    assert_eq!(h.count("game_item_decay_retire_receipts").await?, 0);
    assert_eq!(h.count("game_item_audit_outbox").await?, audits_before);
    assert_eq!(h.count("test_nested_retire_order").await?, 0);
    // Rolled-back retirement proofs cannot authorize a separate raw mutation
    // of the still-live tree before the genuine owner retries the reset.
    reject_unproven_custody_change(h, *items.last().ok_or("deepest child")?, false).await?;
    sqlx::raw_sql(
        "DROP TRIGGER z_test_nested_retire_root ON game_item_decay_retire_receipts;
        DROP FUNCTION test_fail_nested_retire_root();",
    )
    .execute(&h.pool)
    .await?;
    assert!(
        root.retire_world_reset_ground(authority, &h.node, &reset)
            .await
            .map_err(debug)?
    );
    let retired = &items[usize::from(direct_bag)..];
    let expected_order: Vec<_> = retired.iter().rev().map(|item| uuid_text(*item)).collect();
    let order: Vec<String> = sqlx::query_scalar(
        "SELECT item::text FROM test_nested_retire_order ORDER BY position LIMIT 10",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(
        order, expected_order,
        "deepest descendants precede their parent root"
    );
    let expected_count = i64::try_from(retired.len())?;
    let (transactions, audited): (i64, i64) = sqlx::query_as("SELECT count(DISTINCT r.created_xact_id),count(a.event_id) FROM game_item_decay_retire_receipts r JOIN game_item_audit_outbox a ON a.item_instance_id=r.item_instance_id AND a.transaction_id=r.transaction_id AND a.event_id=r.event_id AND a.envelope_sha256=r.envelope_sha256 AND a.created_xact_id=r.created_xact_id WHERE r.cause_kind='WORLD_RESET' AND r.ground_root_item_instance_id=encode($1,'hex')::uuid")
        .bind(selected.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!((transactions, audited), (1, expected_count));
    let edges: Vec<_> = retired.windows(2).map(|pair| (pair[1], pair[0])).collect();
    assert_nested_retirement_audits(h, selected, &edges).await?;
    assert_eq!(
        h.count("game_item_decay_retire_receipts").await?,
        expected_count
    );
    assert_eq!(
        h.count("game_item_audit_outbox").await?,
        audits_before + expected_count
    );
    assert_eq!(h.count("game_item_ground_locations").await?, 0);
    assert_eq!(h.count("game_item_nested_entries").await?, 0);
    assert_eq!(h.count("game_item_container_entries").await?, 0);
    assert_eq!(
        h.count("game_item_container_slots").await?,
        i64::from(direct_bag)
    );
    for item in retired {
        assert_eq!(h.item_state(*item).await?, (0, 2));
    }
    if direct_bag {
        assert_eq!(h.item_state(items[0]).await?, (1, 1));
        let main: String = sqlx::query_scalar("SELECT to_jsonb(i)::text FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(items[0].as_slice()).fetch_one(&h.pool).await?;
        assert!(
            before.items.contains(&main),
            "carried main backpack remains byte-identical"
        );
        let slot: String = sqlx::query_scalar("SELECT to_jsonb(s)::text FROM game_item_container_slots s WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(items[0].as_slice()).fetch_one(&h.pool).await?;
        assert!(before.placements.contains(&("slot".into(), slot)));
    }
    assert!(
        root.retire_world_reset_ground(authority, &h.node, &reset)
            .await
            .map_err(debug)?
    );
    assert_eq!(
        h.count("game_item_decay_retire_receipts").await?,
        expected_count
    );
    assert_eq!(
        h.count("game_item_audit_outbox").await?,
        audits_before + expected_count
    );
    let replay_order: Vec<String> = sqlx::query_scalar(
        "SELECT item::text FROM test_nested_retire_order ORDER BY position LIMIT 10",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(replay_order, order);
    Ok(())
}

async fn assert_nested_retirement_audits(
    h: &Harness,
    root: [u8; 16],
    edges: &[([u8; 16], [u8; 16])],
) -> TestResult {
    let parents: std::collections::BTreeMap<_, _> = edges.iter().copied().collect();
    assert_eq!(parents.len(), edges.len());
    let rows = sqlx::query("SELECT uuid_send(r.item_instance_id) AS item,a.envelope,e.placement_ordinal::text AS edge_ordinal FROM game_item_decay_retire_receipts r JOIN game_item_audit_outbox a ON a.item_instance_id=r.item_instance_id AND a.transaction_id=r.transaction_id AND a.event_id=r.event_id LEFT JOIN game_player_death_retirement_edges e ON e.item_instance_id=r.item_instance_id WHERE r.ground_root_item_instance_id=encode($1,'hex')::uuid AND r.cause_kind='WORLD_RESET' ORDER BY r.item_instance_id LIMIT 502")
        .bind(root.as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(rows.len(), edges.len() + 1);
    for row in rows {
        let item: [u8; 16] = row
            .try_get::<Vec<u8>, _>("item")?
            .try_into()
            .map_err(|_| "audit item UUID")?;
        let wire: Vec<u8> = row.try_get("envelope")?;
        let (_, event) =
            crate::durability::item_decay_retire_audit::decode_decay_retire_envelope(&wire)
                .map_err(debug)?;
        assert_eq!(
            event
                .before
                .as_ref()
                .ok_or("retirement before")?
                .item_instance_id,
            item
        );
        assert_eq!(
            event
                .after
                .as_ref()
                .ok_or("retirement after")?
                .item_instance_id,
            item
        );
        assert_eq!(
            event
                .world_reset
                .as_ref()
                .ok_or("reset cause")?
                .item_instance_id,
            item
        );
        assert!(event.activity_retirement.is_none());
        let ground = if item == root {
            assert!(event.ground_container_entry.is_none());
            assert_eq!(row.try_get::<Option<String>, _>("edge_ordinal")?, None);
            event.ground.as_ref().ok_or("retirement root Ground")?
        } else {
            assert!(event.ground.is_none());
            let entry = event
                .ground_container_entry
                .as_ref()
                .ok_or("retired edge")?;
            assert_eq!(
                entry.parent_item_instance_id,
                *parents.get(&item).ok_or("audit parent")?
            );
            assert_eq!(entry.tree_root_item_instance_id, Some(root.to_vec()));
            let ordinal: String = row.try_get("edge_ordinal")?;
            assert_eq!(entry.placement_ordinal, ordinal.parse::<u64>()?);
            entry
                .parent_ground
                .as_ref()
                .ok_or("retired edge root Ground")?
        };
        assert_eq!(ground.world_id, id(WORLD));
        assert_eq!(ground.channel_id, id(CHANNEL));
        assert_eq!(ground.instance_id, None);
    }
    Ok(())
}

async fn nested_reset_terminal_snapshot(h: &Harness) -> TestResult<Vec<(String, String)>> {
    let rows = sqlx::query_as(
        "SELECT 'item'::text AS kind,to_jsonb(i)::text AS data FROM game_item_instances i \
         UNION ALL SELECT 'ground',to_jsonb(g)::text FROM game_item_ground_locations g \
         UNION ALL SELECT 'slot',to_jsonb(s)::text FROM game_item_container_slots s \
         UNION ALL SELECT 'direct',to_jsonb(e)::text FROM game_item_container_entries e \
         UNION ALL SELECT 'nested',to_jsonb(e)::text FROM game_item_nested_entries e \
         UNION ALL SELECT 'retirement_edge',to_jsonb(e)::text FROM game_player_death_retirement_edges e \
         UNION ALL SELECT 'receipt',to_jsonb(r)::text FROM game_item_decay_retire_receipts r \
         UNION ALL SELECT 'audit',to_jsonb(a)::text FROM game_item_audit_outbox a \
         UNION ALL SELECT 'ground_removal',to_jsonb(e)::text FROM game_item_ground_removal_evidence e \
         UNION ALL SELECT 'corpse_removal',to_jsonb(e)::text FROM game_item_corpse_entry_removal_evidence e \
         UNION ALL SELECT 'quantity',to_jsonb(e)::text FROM game_item_transfer_quantity_evidence e \
         UNION ALL SELECT 'reset',to_jsonb(j)::text FROM game_world_reset_jobs j \
         ORDER BY 1,2 LIMIT 128",
    )
    .fetch_all(&h.pool)
    .await?;
    assert!(rows.len() < 128, "capture the complete small-tree footprint");
    Ok(rows)
}

async fn run_nested_reset_root_bounds_refusal(admin: String) -> TestResult {
    use crate::durability::DurabilityError;
    use crate::durability::item_decay_retire::DecayRetireError;
    use crate::item_transfer_postgres_cases::uuid_text;

    let h = Harness::create(admin, "death_nested_root_bounds").await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let items = carried_chain(&h, &authority, 2).await?;
    let selected = items[0];
    drop_unselected_death_root(&h, &authority, selected, false).await?;
    let reset = prepare_nested_reset(&h, &h.root).await?;
    let before = nested_reset_terminal_snapshot(&h).await?;
    let audits_before = h.count("game_item_audit_outbox").await?;
    assert_eq!(h.count("game_item_decay_retire_reservations").await?, 0);

    // Inject one false result into this disposable schema's real bounds helper,
    // retaining its OID, security attributes and original body for other roots.
    // This proves the terminal gate is mandatory, not its numerical bounds or
    // maximum-tree performance. Every existing custody/fence guard stays active.
    let (definition, body): (String, String) = sqlx::query_as(
        "SELECT pg_get_functiondef(p.oid),p.prosrc FROM pg_proc p \
         WHERE p.oid='game_player_death_retirement_root_bounds_proven(uuid)'::regprocedure",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(definition.matches(body.as_str()).count(), 1);
    sqlx::raw_sql(
        "CREATE SEQUENCE test_nested_root_bounds_reached_seq; \
         GRANT USAGE ON SEQUENCE test_nested_root_bounds_reached_seq TO oteryn_game_runtime;",
    )
    .execute(&h.pool)
    .await?;
    let injected_body = format!(
        "SELECT CASE WHEN p_root='{}'::uuid THEN CASE WHEN ( \
           SELECT count(*)={} FROM game_item_decay_retire_receipts r \
           JOIN game_item_audit_outbox a ON a.item_instance_id=r.item_instance_id \
            AND a.transaction_id=r.transaction_id AND a.event_id=r.event_id \
            AND a.envelope_sha256=r.envelope_sha256 AND a.created_xact_id=r.created_xact_id \
           JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id \
            AND i.quantity=0 AND i.lifecycle=2 AND i.last_transaction_id=r.transaction_id \
           WHERE r.cause_kind='WORLD_RESET' AND r.ground_root_item_instance_id=p_root \
            AND r.created_xact_id=pg_current_xact_id() \
         ) THEN (nextval('test_nested_root_bounds_reached_seq'::regclass) IS NULL) \
           ELSE FALSE END ELSE ({}) END",
        uuid_text(selected),
        items.len(),
        body.trim().trim_end_matches(';'),
    );
    let injected_definition = definition.replacen(body.as_str(), &injected_body, 1);
    // The captured server definition and canonical fixture UUID/count contain
    // no external input; SQLx's dynamic-script boundary is explicit here.
    sqlx::raw_sql(sqlx::AssertSqlSafe(injected_definition))
        .execute(&h.pool)
        .await?;
    let rejected = h
        .root
        .retire_world_reset_ground(&authority, &h.node, &reset)
        .await;
    // Restore before checking the refusal, including when an earlier owner
    // unexpectedly rejects; assertion failures must not leave the stub installed.
    sqlx::raw_sql(sqlx::AssertSqlSafe(definition))
        .execute(&h.pool)
        .await?;
    assert!(matches!(
        rejected,
        Err(DecayRetireError::Unavailable(DurabilityError::CommitRejected))
    ));
    let reached: (i64, bool) =
        sqlx::query_as("SELECT last_value,is_called FROM test_nested_root_bounds_reached_seq")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        reached,
        (1, true),
        "the root gate must see every retired item, receipt and audit before refusing COMMIT"
    );
    assert_eq!(nested_reset_terminal_snapshot(&h).await?, before);
    // The independently committed frozen root reservation survives for retry;
    // descendant reservations belong to the rejected physical transaction.
    assert_eq!(h.count("game_item_decay_retire_reservations").await?, 1);
    let frozen: (String, String, String) = sqlx::query_as(
        "SELECT item_instance_id::text,transaction_id::text,event_id::text \
         FROM game_item_decay_retire_reservations",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(frozen.0, uuid_text(selected));
    assert!(
        h.root
            .retire_world_reset_ground(&authority, &h.node, &reset)
            .await
            .map_err(debug)?
    );
    let committed: (String, String, String) = sqlx::query_as(
        "SELECT item_instance_id::text,transaction_id::text,event_id::text \
         FROM game_item_decay_retire_receipts WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(selected.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(committed, frozen, "retry retains the frozen root identity");
    let expected = i64::try_from(items.len())?;
    let (transactions, audited): (i64, i64) = sqlx::query_as("SELECT count(DISTINCT r.created_xact_id),count(a.event_id) FROM game_item_decay_retire_receipts r JOIN game_item_audit_outbox a ON a.item_instance_id=r.item_instance_id AND a.transaction_id=r.transaction_id AND a.event_id=r.event_id AND a.envelope_sha256=r.envelope_sha256 AND a.created_xact_id=r.created_xact_id WHERE r.cause_kind='WORLD_RESET' AND r.ground_root_item_instance_id=encode($1,'hex')::uuid")
        .bind(selected.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!((transactions, audited), (1, expected));
    assert_eq!(h.count("game_item_decay_retire_receipts").await?, expected);
    assert_eq!(h.count("game_player_death_retirement_edges").await?, expected - 1);
    assert_eq!(h.count("game_item_audit_outbox").await?, audits_before + expected);
    let edges: Vec<_> = items.windows(2).map(|pair| (pair[1], pair[0])).collect();
    assert_nested_retirement_audits(&h, selected, &edges).await?;
    for item in &items {
        assert_eq!(h.item_state(*item).await?, (0, 2));
    }
    for relation in [
        "game_item_ground_locations",
        "game_item_container_slots",
        "game_item_container_entries",
        "game_item_nested_entries",
    ] {
        assert_eq!(h.count(relation).await?, 0);
    }
    let retired = nested_reset_terminal_snapshot(&h).await?;
    assert!(
        h.root
            .retire_world_reset_ground(&authority, &h.node, &reset)
            .await
            .map_err(debug)?
    );
    assert_eq!(nested_reset_terminal_snapshot(&h).await?, retired);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

#[test]
fn player_death_nested_reset_requires_terminal_root_bounds() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime()
                .map_err(debug)?
                .block_on(run_nested_reset_root_bounds_refusal(admin))
                .map_err(debug)
        })?
        .join()
        .map_err(|_| "nested root bounds test thread panicked")?
        .map_err(Into::into)
}

struct MaximumDeathTree {
    items: Vec<[u8; 16]>,
    edges: Vec<([u8; 16], [u8; 16])>,
}

async fn append_maximum_tree_item(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    tree: &mut MaximumDeathTree,
    parent: Option<[u8; 16]>,
    key: &str,
) -> TestResult<[u8; 16]> {
    use crate::durability::item_loop_move::{LoopDestination, LoopMoveRequest};

    let item = h.mint(authority, key, 1).await?;
    let ordinal = u64::try_from(tree.items.len())?;
    let definition = if key == BACKPACK {
        backpack_facts()
    } else {
        facts(key, ItemStackClass::NonStackable)
    };
    h.committed(
        authority,
        to_backpack(command(1_000 + ordinal * 2)?, item, definition),
    )
    .await?;
    if let Some(parent) = parent {
        let f = fence()?;
        let permit = SpellLanePermit::of_fresh_scope(f.runtime_scope).await?;
        let moved = h
            .root
            .commit_item_loop_move(
                &permit,
                authority,
                &h.node,
                &f,
                command(1_001 + ordinal * 2)?,
                &PickupContent,
                LoopMoveRequest {
                    item,
                    count: None,
                    pickup_cell: None,
                    destination: LoopDestination::Container(parent),
                },
            )
            .await?
            .map_err(debug)?;
        assert_eq!(moved.operation, 1);
        assert!(!moved.replayed);
    }
    tree.edges.push((item, parent.unwrap_or(tree.items[0])));
    tree.items.push(item);
    Ok(item)
}

async fn maximum_carried_death_tree(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult<MaximumDeathTree> {
    use crate::durability::item_loop_move::ITEM_LOOP_TREE_ITEMS_MAX;
    use crate::item_transfer_postgres_cases::{CHARACTER, STONE};

    assert_eq!(ITEM_LOOP_TREE_ITEMS_MAX, 500);
    let main = h.mint(authority, BACKPACK, 1).await?;
    h.committed(authority, to_slot(command(1)?, main, backpack_facts()))
        .await?;
    initialize(h, authority).await?;
    let mut tree = MaximumDeathTree {
        items: vec![main],
        edges: Vec::new(),
    };
    let mut parents = Vec::new();
    for _ in 0..2 {
        parents.push(append_maximum_tree_item(h, authority, &mut tree, None, BACKPACK).await?);
    }
    let mut leaf_bags = Vec::new();
    for index in 0..24 {
        leaf_bags.push(
            append_maximum_tree_item(h, authority, &mut tree, Some(parents[index / 20]), BACKPACK)
                .await?,
        );
    }
    // Two direct bags + twenty-four inner bags + 474 stones = 500 descendants.
    // Each parent has at most twenty entries; the main keeps a free staging
    // slot for every normal MINT -> pickup -> loop-move sequence.
    for index in 0..474 {
        append_maximum_tree_item(h, authority, &mut tree, Some(leaf_bags[index / 20]), STONE)
            .await?;
    }
    assert_eq!((tree.items.len(), tree.edges.len()), (501, 500));
    let size: i64 = sqlx::query_scalar("SELECT game_item_loop_tree_size(encode($1,'hex')::uuid)")
        .bind(id(CHARACTER).as_slice())
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(size, ITEM_LOOP_TREE_ITEMS_MAX);
    let (owned, deepest): (i64, Option<i32>) = sqlx::query_as("SELECT count(*),max(o.depth) FROM game_item_instances i CROSS JOIN LATERAL game_item_loop_owner(i.item_instance_id) o WHERE o.owner_character=encode($1,'hex')::uuid")
        .bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!((owned, deepest), (501, Some(3)));
    assert_eq!(h.count("game_item_container_entries").await?, 2);
    assert_eq!(h.count("game_item_nested_entries").await?, 498);
    Ok(tree)
}

async fn run_maximum_nested_death_reset(admin: String) -> TestResult {
    use crate::item_transfer_postgres_cases::uuid_text;

    let h = Harness::create(admin, "death_nested_maximum_reset").await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let tree = maximum_carried_death_tree(&h, &authority).await?;
    let selected = tree.items[0];
    let before_drop = nested_death_snapshot(&h, selected).await?;
    assert_eq!(
        (before_drop.items.len(), before_drop.placements.len()),
        (500, 500)
    );
    drop_unselected_death_root(&h, &authority, selected, false).await?;
    assert_eq!(nested_death_snapshot(&h, selected).await?, before_drop);
    assert_eq!(h.count("game_item_container_entries").await?, 0);
    assert_eq!(h.count("game_item_nested_entries").await?, 500);
    let reset = prepare_nested_reset(&h, &h.root).await?;
    let audits_before = h.count("game_item_audit_outbox").await?;
    // The production owner retains its existing two-second semantic-pass
    // deadline and retry ceiling. A timeout is a test failure, never a skip or
    // permission to widen those limits for the largest source-legal graph.
    let started = std::time::Instant::now();
    assert!(
        h.root
            .retire_world_reset_ground(&authority, &h.node, &reset)
            .await
            .map_err(debug)?
    );
    eprintln!(
        "DUR-003-MAX-RESET: 501 items completed in {:?}",
        started.elapsed()
    );
    // These immutable receipts are inserted by separate commands after each
    // item's mutation. Within one physical transaction, cmin orders those
    // INSERTs without adding observer writes to the measured production pass.
    let order: Vec<(String, i64, String)> = sqlx::query_as(
        "SELECT r.item_instance_id::text,r.cmin::text::bigint,r.created_xact_id::text FROM game_item_decay_retire_receipts r WHERE r.cause_kind='WORLD_RESET' AND r.ground_root_item_instance_id=encode($1,'hex')::uuid ORDER BY r.cmin::text::bigint,r.item_instance_id LIMIT 502",
    )
    .bind(selected.as_slice())
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(order.len(), 501);
    let physical_transactions: std::collections::BTreeSet<_> =
        order.iter().map(|(_, _, transaction)| transaction).collect();
    assert_eq!(physical_transactions.len(), 1);
    assert_eq!(order.last().map(|(item, _, _)| item), Some(&uuid_text(selected)));
    let commands: std::collections::BTreeMap<_, _> = order
        .iter()
        .map(|(item, command, _)| (item.clone(), *command))
        .collect();
    assert_eq!(commands.len(), 501);
    for (child, parent) in &tree.edges {
        let child_command = commands.get(&uuid_text(*child)).ok_or("retired child")?;
        let parent_command = commands.get(&uuid_text(*parent)).ok_or("retired parent")?;
        assert!(
            child_command < parent_command,
            "each child receipt must precede its own parent receipt"
        );
    }
    let (transactions, audited): (i64, i64) = sqlx::query_as("SELECT count(DISTINCT r.created_xact_id),count(a.event_id) FROM game_item_decay_retire_receipts r JOIN game_item_audit_outbox a ON a.item_instance_id=r.item_instance_id AND a.transaction_id=r.transaction_id AND a.event_id=r.event_id AND a.envelope_sha256=r.envelope_sha256 AND a.created_xact_id=r.created_xact_id WHERE r.cause_kind='WORLD_RESET' AND r.ground_root_item_instance_id=encode($1,'hex')::uuid")
        .bind(selected.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!((transactions, audited), (1, 501));
    assert_nested_retirement_audits(&h, selected, &tree.edges).await?;
    assert_eq!(h.count("game_item_decay_retire_receipts").await?, 501);
    assert_eq!(
        h.count("game_item_audit_outbox").await?,
        audits_before + 501
    );
    let (retired, other): (i64, i64) = sqlx::query_as("SELECT count(*) FILTER(WHERE quantity=0 AND lifecycle=2),count(*) FILTER(WHERE quantity<>0 OR lifecycle<>2) FROM game_item_instances")
        .fetch_one(&h.pool).await?;
    assert_eq!((retired, other), (501, 0));
    for relation in [
        "game_item_ground_locations",
        "game_item_container_slots",
        "game_item_container_entries",
        "game_item_nested_entries",
    ] {
        assert_eq!(h.count(relation).await?, 0);
    }
    let retired_rows = nested_death_snapshot(&h, [0; 16]).await?;
    assert_eq!(retired_rows.items.len(), 501);
    assert!(
        h.root
            .retire_world_reset_ground(&authority, &h.node, &reset)
            .await
            .map_err(debug)?
    );
    assert_eq!(nested_death_snapshot(&h, [0; 16]).await?, retired_rows);
    assert_eq!(h.count("game_item_decay_retire_receipts").await?, 501);
    assert_eq!(
        h.count("game_item_audit_outbox").await?,
        audits_before + 501
    );
    let replay_order: Vec<(String, i64, String)> = sqlx::query_as(
        "SELECT r.item_instance_id::text,r.cmin::text::bigint,r.created_xact_id::text FROM game_item_decay_retire_receipts r WHERE r.cause_kind='WORLD_RESET' AND r.ground_root_item_instance_id=encode($1,'hex')::uuid ORDER BY r.cmin::text::bigint,r.item_instance_id LIMIT 502",
    )
    .bind(selected.as_slice())
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(replay_order, order);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

#[test]
fn player_death_nested_maximum_501_item_reset_makes_bounded_atomic_progress() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime()
                .map_err(debug)?
                .block_on(run_maximum_nested_death_reset(admin))
                .map_err(debug)
        })?
        .join()
        .map_err(|_| "maximum nested death reset thread panicked")?
        .map_err(Into::into)
}

/// Qualified test Content keeps the same weights for the whole scenario. One
/// additional carried bag makes the complete incoming tree exactly one unit
/// overweight; dropping that bag admits the same tree at the exact capacity.
struct WeightedDeathPickup {
    stone_weight: u32,
}

impl crate::durability::item_transfer::ItemWeightLookup for WeightedDeathPickup {
    fn generation_digest(&self) -> [u8; 32] {
        [0xd1; 32]
    }

    fn weight_of(
        &self,
        definition: &crate::durability::item_mint::TypedDefinitionRef,
    ) -> Result<u32, crate::durability::item_transfer::ItemWeightRefused> {
        use crate::durability::item_transfer::ItemWeightLookup;
        let ordinary = ItemWeightLookup::weight_of(&PickupContent, definition)?;
        Ok(
            if definition.production_key == crate::item_transfer_postgres_cases::STONE {
                self.stone_weight
            } else {
                ordinary
            },
        )
    }
}

impl crate::durability::item_loop_move::ItemLoopContent for WeightedDeathPickup {
    fn loop_content_digest(&self) -> [u8; 32] {
        DIGEST
    }

    fn loop_facts(
        &self,
        definition: &crate::durability::item_mint::TypedDefinitionRef,
    ) -> Option<ItemDefinitionFacts> {
        use crate::durability::item_loop_move::ItemLoopContent;
        let mut value = ItemLoopContent::loop_facts(&PickupContent, definition)?;
        if definition.production_key == crate::item_transfer_postgres_cases::STONE {
            value.weight = Some(self.stone_weight);
        }
        Some(value)
    }
}

fn nested_death_cell() -> Vec<u8> {
    vec![0, 0, 0, 12, 0, 0, 0, 13, 0, 7]
}

async fn drop_unselected_death_root(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    selected: [u8; 16],
    historical: bool,
) -> TestResult {
    let f = fence()?;
    let mut gameplay = CurrentCharacterGameplayFence {
        character_id: f.character_id,
        game_session_id: f.game_session_id,
        connection_generation: f.connection_generation,
        character_lease_generation: f.character_lease_generation,
        runtime_scope: f.runtime_scope,
        scope_ownership_generation: f.scope_ownership_generation,
        expected_character_revision: CharacterRevision::new(h.character_revision().await?.parse()?)
            .map_err(debug)?,
    };
    let mut intent = death()?;
    intent.death_cell.spatial_position = nested_death_cell();
    h.root
        .commit_character_death(authority, &h.node, gameplay, intent)
        .await
        .map_err(debug)?;
    if historical {
        // A disposable database models the previously valid writer version.
        // Only the new representation-conversion trigger is absent while the
        // real death writer runs; every existing custody/fence guard remains on.
        sqlx::query("ALTER TABLE game_player_death_item_records DISABLE TRIGGER game_player_death_container_normalize")
            .execute(&h.pool).await?;
    }
    let permit = SpellLanePermit::of_fresh_scope(f.runtime_scope).await?;
    let dropped = h
        .root
        .commit_player_death_drop(
            &permit,
            authority,
            &h.node,
            f,
            PlayerDeathDropCause {
                death_occurrence: id(170),
                item_instance: selected,
            },
            request(180),
        )
        .await;
    if historical {
        sqlx::query("ALTER TABLE game_player_death_item_records ENABLE TRIGGER game_player_death_container_normalize")
            .execute(&h.pool).await?;
    }
    assert_eq!(dropped?, PlayerDeathItemOutcome::Written);
    drop(permit);
    gameplay.expected_character_revision =
        CharacterRevision::new(h.character_revision().await?.parse()?).map_err(debug)?;
    assert!(
        h.root
            .consume_pending_respawn(
                authority,
                &h.node,
                gameplay,
                Some(PlayerDeathOccurrence::from_bytes(id(170)).map_err(debug)?)
            )
            .await
            .map_err(debug)?
            .is_some()
    );
    Ok(())
}

async fn reject_unproven_custody_change(
    h: &Harness,
    child: [u8; 16],
    normalization: bool,
) -> TestResult {
    let before = nested_death_snapshot(h, [0; 16]).await?;
    let footprint = h.footprint().await?;
    let direct = h.count("game_item_container_entries").await?;
    let nested = h.count("game_item_nested_entries").await?;
    let ground: Vec<String> = sqlx::query_scalar("SELECT to_jsonb(g)::text FROM game_item_ground_locations g ORDER BY item_instance_id LIMIT 10")
        .fetch_all(&h.pool).await?;
    let receipts = h.count("game_item_loop_move_receipts").await?;
    let lines = h.count("game_item_loop_move_lines").await?;
    let mutations: &[&str] = if normalization {
        &[
            "WITH moved AS (DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid RETURNING item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id FROM moved",
        ]
    } else {
        &[
            "DELETE FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid",
            "UPDATE game_item_nested_entries SET placement_ordinal=placement_ordinal+1 WHERE item_instance_id=encode($1,'hex')::uuid",
        ]
    };
    for statement in mutations {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        let write = sqlx::query(sqlx::AssertSqlSafe(*statement))
            .bind(child.as_slice())
            .execute(&mut *tx)
            .await;
        let result = match write {
            Ok(_) => tx.commit().await,
            Err(error) => {
                tx.rollback().await?;
                Err(error)
            }
        };
        assert!(
            result.is_err(),
            "private conversion evidence must not authorize an unproven custody write"
        );
        assert_eq!(nested_death_snapshot(h, [0; 16]).await?, before);
        assert_eq!(h.footprint().await?, footprint);
        assert_eq!(h.count("game_item_container_entries").await?, direct);
        assert_eq!(h.count("game_item_nested_entries").await?, nested);
        assert_eq!(h.count("game_item_loop_move_receipts").await?, receipts);
        assert_eq!(h.count("game_item_loop_move_lines").await?, lines);
        let after: Vec<String> = sqlx::query_scalar("SELECT to_jsonb(g)::text FROM game_item_ground_locations g ORDER BY item_instance_id LIMIT 10")
            .fetch_all(&h.pool).await?;
        assert_eq!(after, ground);
    }
    Ok(())
}

async fn run_nested_death_pickup(
    admin: String,
    other_character: bool,
    historical: bool,
) -> TestResult {
    use crate::corpse_transfer_postgres_cases::{
        SECOND_CHARACTER, SECOND_SESSION, command_of, fence_of, seed_second_character,
    };
    use crate::durability::item_loop_move::{
        LoopDestination, LoopGround, LoopMoveRefusal, LoopMoveRequest,
    };
    use crate::durability::item_transfer::ItemTransferOutcome;
    use crate::item_transfer_postgres_cases::{CHARACTER, SESSION, uuid_text};

    // Each routed test owns a distinct retained recovery directory, including
    // when the test harness runs all four scenarios concurrently in one PID.
    let tag = match (other_character, historical) {
        (false, false) => "death_nested_pickup_same",
        (true, false) => "death_nested_pickup_other",
        (false, true) => "death_nested_pickup_historical_same",
        (true, true) => "death_nested_pickup_historical_other",
    };
    let h = Harness::create(admin, tag).await?;
    if other_character {
        seed_second_character(&h.pool).await?;
    }
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    // A main root of height two can legally become an entry at depth one.
    // The depth-eight death/reset cases intentionally cannot use that target.
    let items = carried_chain(&h, &authority, 2).await?;
    let selected = items[0];
    let before_drop = nested_death_snapshot(&h, selected).await?;
    assert_eq!(
        (before_drop.items.len(), before_drop.placements.len()),
        (2, 2)
    );
    reject_unproven_custody_change(&h, items[1], true).await?;
    drop_unselected_death_root(&h, &authority, selected, historical).await?;
    assert_eq!(nested_death_snapshot(&h, selected).await?, before_drop);
    assert_eq!(
        h.count("game_item_container_entries").await?,
        i64::from(historical)
    );
    assert_eq!(
        h.count("game_item_nested_entries").await?,
        if historical { 1 } else { 2 }
    );
    if !historical {
        reject_unproven_custody_change(&h, items[1], false).await?;
    }
    let (receiver, session) = if other_character {
        (SECOND_CHARACTER, SECOND_SESSION)
    } else {
        (CHARACTER, SESSION)
    };
    let f = fence_of(receiver, session)?;
    let destination = h.mint(&authority, BACKPACK, 1).await?;
    assert!(matches!(
        h.transfer(
            &authority,
            f,
            to_slot(command_of(session, 200)?, destination, backpack_facts())
        )
        .await
        .map_err(debug)?,
        ItemTransferOutcome::Committed(_)
    ));
    initialize_for(&h, &authority, &f, command_of(session, 202)?).await?;
    let ballast = h.mint(&authority, BACKPACK, 1).await?;
    assert!(matches!(
        h.transfer(
            &authority,
            f,
            to_backpack(command_of(session, 201)?, ballast, backpack_facts())
        )
        .await
        .map_err(debug)?,
        ItemTransferOutcome::Committed(_)
    ));
    let (level, vocation): (i64, Option<String>) = sqlx::query_as("SELECT p.level,b.vocation FROM game_character_progression_state p LEFT JOIN game_character_build_state b USING(character_id) WHERE p.character_id=encode($1,'hex')::uuid")
        .bind(id(receiver).as_slice()).fetch_one(&h.pool).await?;
    let capacity = oteryn_gameplay_items::item_capacity::project_global_capacity_centi_oz(
        u32::try_from(level)?,
        vocation.as_deref().unwrap_or("none"),
    )
    .map_err(debug)?;
    // Destination main, incoming main and its inner bag weigh one each.
    let content = WeightedDeathPickup {
        stone_weight: u32::try_from(capacity.checked_sub(3).ok_or("capacity")?)?,
    };
    let movement = LoopMoveRequest {
        item: selected,
        count: None,
        pickup_cell: Some(nested_death_cell()),
        destination: LoopDestination::MainBackpack,
    };
    let permit = SpellLanePermit::of_fresh_scope(f.runtime_scope).await?;
    let before_refusal = nested_death_snapshot(&h, [0; 16]).await?;
    let footprint = h.footprint().await?;
    let receipts = h.count("game_item_loop_move_receipts").await?;
    let ground_before: String = sqlx::query_scalar("SELECT to_jsonb(g)::text FROM game_item_ground_locations g WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(selected.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        h.root
            .commit_item_loop_move(
                &permit,
                &authority,
                &h.node,
                &f,
                command_of(session, 210)?,
                &content,
                movement.clone()
            )
            .await?,
        Err(LoopMoveRefusal::Overweight)
    );
    assert_eq!(nested_death_snapshot(&h, [0; 16]).await?, before_refusal);
    assert_eq!(h.footprint().await?, footprint);
    assert_eq!(h.count("game_item_loop_move_receipts").await?, receipts);
    let ground_after: String = sqlx::query_scalar("SELECT to_jsonb(g)::text FROM game_item_ground_locations g WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(selected.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(ground_after, ground_before);
    // The occupied death tile admits only its existing placement pins. Reuse
    // those actual facts when removing the ballast through the real Ground owner.
    let (ballast_position, ballast_map, ballast_content, ballast_context): (
        Vec<u8>,
        String,
        String,
        Vec<u8>,
    ) = sqlx::query_as("SELECT spatial_position,map_revision,content_revision,native_room_placement_context FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(selected.as_slice()).fetch_one(&h.pool).await?;
    let moved = h
        .root
        .commit_item_loop_move(
            &permit,
            &authority,
            &h.node,
            &f,
            command_of(session, 211)?,
            &content,
            LoopMoveRequest {
                item: ballast,
                count: None,
                pickup_cell: None,
                destination: LoopDestination::Ground(LoopGround {
                    spatial_position: ballast_position,
                    map_revision: ballast_map,
                    content_revision: ballast_content,
                    placement_context: ballast_context,
                }),
            },
        )
        .await?
        .map_err(debug)?;
    assert_eq!(moved.operation, 4);
    assert!(!moved.replayed);
    let before_pickup = nested_death_snapshot(&h, selected).await?;
    let audits_before = h.count("game_item_audit_outbox").await?;
    let receipts_before = h.count("game_item_loop_move_receipts").await?;
    let lines_before = h.count("game_item_loop_move_lines").await?;
    let pickup = h
        .root
        .commit_item_loop_move(
            &permit,
            &authority,
            &h.node,
            &f,
            command_of(session, 212)?,
            &content,
            movement.clone(),
        )
        .await?
        .map_err(debug)?;
    assert_eq!(pickup.operation, 5);
    assert!(!pickup.replayed);
    assert!(!h.on_ground(selected).await?);
    assert_eq!(nested_death_snapshot(&h, selected).await?, before_pickup);
    assert_eq!(h.count("game_item_container_entries").await?, 1);
    assert_eq!(h.count("game_item_nested_entries").await?, 2);
    assert_eq!(
        h.count("game_item_loop_move_receipts").await?,
        receipts_before + 1
    );
    // Ordinary loop moves preserve the Type2 outbox and write their own receipt/line.
    assert_eq!(h.count("game_item_audit_outbox").await?, audits_before);
    assert_eq!(h.count("game_item_loop_move_lines").await?, lines_before + 1);
    let pickup_rows: Vec<(String, String)> = sqlx::query_as("SELECT to_jsonb(r)::text,to_jsonb(l)::text FROM game_item_loop_move_receipts r JOIN game_item_loop_move_lines l USING(transaction_id) WHERE r.transaction_id=encode($1,'hex')::uuid AND r.event_id=encode($2,'hex')::uuid AND r.character_id=encode($3,'hex')::uuid AND r.operation=5 AND r.line_count=1 AND l.line_ordinal=1 AND l.item_instance_id=encode($4,'hex')::uuid AND l.from_kind=3 AND l.to_kind=1 AND l.to_parent_item_instance_id=encode($5,'hex')::uuid AND l.quantity_before=1 AND l.quantity_after=1 AND l.created_xact_id=r.created_xact_id")
        .bind(pickup.transaction_id.as_slice())
        .bind(pickup.event_id.as_slice())
        .bind(id(receiver).as_slice())
        .bind(selected.as_slice())
        .bind(destination.as_slice())
        .fetch_all(&h.pool).await?;
    assert_eq!(pickup_rows.len(), 1, "one exact Ground-to-main root line");
    for (index, item) in items.iter().enumerate() {
        let actual: (Option<String>, Option<i32>) = sqlx::query_as(
            "SELECT owner_character::text,depth FROM game_item_loop_owner(encode($1,'hex')::uuid)",
        )
        .bind(item.as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            actual,
            (
                Some(uuid_text(id(receiver))),
                Some(i32::try_from(index)? + 1)
            )
        );
    }
    for (container, parent, child) in [
        (destination, None, selected),
        (selected, Some(destination), items[1]),
        (items[1], Some(selected), items[2]),
    ] {
        let view = h
            .root
            .read_item_loop_container(&authority, f.character_id, container)
            .await?
            .ok_or("carried death container view")?;
        assert_eq!(view.container.item_instance_id, container);
        assert_eq!(view.parent, parent);
        assert_eq!(view.entries.len(), 1);
        assert_eq!(view.entries[0].item.item_instance_id, child);
        assert_eq!(view.entries[0].item.quantity, 1);
        let ordinal: String = sqlx::query_scalar("SELECT placement_ordinal::text FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid UNION ALL SELECT placement_ordinal::text FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(child.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(view.entries[0].placement_ordinal, ordinal.parse::<u64>()?);
        if other_character {
            assert!(
                h.root
                    .read_item_loop_container(&authority, fence()?.character_id, container)
                    .await?
                    .is_none()
            );
        }
    }
    let written = nested_death_snapshot(&h, [0; 16]).await?;
    let replay = h
        .root
        .commit_item_loop_move(
            &permit,
            &authority,
            &h.node,
            &f,
            command_of(session, 212)?,
            &content,
            movement,
        )
        .await?
        .map_err(debug)?;
    assert!(replay.replayed);
    assert_eq!(
        (replay.transaction_id, replay.event_id, replay.operation),
        (pickup.transaction_id, pickup.event_id, pickup.operation)
    );
    assert_eq!(replay.floor_effect, None);
    assert_eq!(nested_death_snapshot(&h, [0; 16]).await?, written);
    assert_eq!(
        h.count("game_item_loop_move_receipts").await?,
        receipts_before + 1
    );
    assert_eq!(h.count("game_item_audit_outbox").await?, audits_before);
    assert_eq!(h.count("game_item_loop_move_lines").await?, lines_before + 1);
    let replay_rows: Vec<(String, String)> = sqlx::query_as("SELECT to_jsonb(r)::text,to_jsonb(l)::text FROM game_item_loop_move_receipts r JOIN game_item_loop_move_lines l USING(transaction_id) WHERE r.transaction_id=encode($1,'hex')::uuid ORDER BY l.line_ordinal")
        .bind(pickup.transaction_id.as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(replay_rows, pickup_rows);
    drop(permit);
    reject_unproven_custody_change(&h, items[1], false).await?;
    drop(authority);
    drop(seal);
    h.cleanup().await
}

fn nested_death_pickup_case(other_character: bool, historical: bool) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime()
                .map_err(debug)?
                .block_on(run_nested_death_pickup(admin, other_character, historical))
                .map_err(debug)
        })?
        .join()
        .map_err(|_| "nested death pickup thread panicked")?
        .map_err(Into::into)
}

#[test]
fn player_death_nested_backpack_pickup_same_owner_honors_complete_weight() -> TestResult {
    nested_death_pickup_case(false, false)
}

#[test]
fn player_death_nested_backpack_pickup_changes_every_descendant_owner() -> TestResult {
    nested_death_pickup_case(true, false)
}

#[test]
fn player_death_nested_historical_backpack_pickup_same_owner_is_atomic() -> TestResult {
    nested_death_pickup_case(false, true)
}

#[test]
fn player_death_nested_historical_backpack_pickup_changes_every_descendant_owner() -> TestResult {
    nested_death_pickup_case(true, true)
}

async fn run_historical_slot_zero_death(admin: String) -> TestResult {
    use crate::corpse_transfer_postgres_cases::{
        SECOND_CHARACTER, SECOND_SESSION, command_of, fence_of, seed_second_character,
    };
    use crate::durability::character_death::CharacterDeathOutcome;
    use crate::durability::item_loop_move::{LoopDestination, LoopMoveRequest};
    use crate::durability::item_transfer::ItemTransferOutcome;
    use crate::item_transfer_postgres_cases::uuid_text;

    let h = Harness::create(admin, "death_historical_slot_zero").await?;
    seed_second_character(&h.pool).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let items = carried_chain(&h, &authority, 2).await?;
    let selected = items[0];
    drop_unselected_death_root(&h, &authority, selected, true).await?;
    let f = fence_of(SECOND_CHARACTER, SECOND_SESSION)?;
    let destination = h.mint(&authority, BACKPACK, 1).await?;
    assert!(matches!(
        h.transfer(
            &authority,
            f,
            to_slot(
                command_of(SECOND_SESSION, 200)?,
                destination,
                backpack_facts()
            )
        )
        .await
        .map_err(debug)?,
        ItemTransferOutcome::Committed(_)
    ));
    initialize_for(&h, &authority, &f, command_of(SECOND_SESSION, 202)?).await?;
    // Preserve only the old writer's representation in this disposable DB.
    // The real historical pickup keeps all existing custody/fence guards;
    // its new normalization trigger is restored before the tested death.
    sqlx::query("ALTER TABLE game_item_loop_move_lines DISABLE TRIGGER game_player_death_container_normalize")
        .execute(&h.pool).await?;
    let permit = SpellLanePermit::of_fresh_scope(f.runtime_scope).await?;
    let picked = h
        .root
        .commit_item_loop_move(
            &permit,
            &authority,
            &h.node,
            &f,
            command_of(SECOND_SESSION, 210)?,
            &PickupContent,
            LoopMoveRequest {
                item: selected,
                count: None,
                pickup_cell: Some(nested_death_cell()),
                destination: LoopDestination::MainBackpack,
            },
        )
        .await;
    sqlx::query("ALTER TABLE game_item_loop_move_lines ENABLE TRIGGER game_player_death_container_normalize")
        .execute(&h.pool).await?;
    let picked = picked?.map_err(debug)?;
    assert_eq!(picked.operation, 5);
    assert!(!picked.replayed);
    drop(permit);
    assert_eq!(h.count("game_item_container_entries").await?, 2);
    assert_eq!(h.count("game_item_nested_entries").await?, 1);
    let source: (String, String) = sqlx::query_as("SELECT character_id::text,parent_item_instance_id::text FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(selected.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        source,
        (uuid_text(id(SECOND_CHARACTER)), uuid_text(destination))
    );
    let before = nested_death_snapshot(&h, selected).await?;
    let mut intent = death()?;
    intent.occurrence = PlayerDeathOccurrence::from_bytes(id(171)).map_err(debug)?;
    let second_cell = vec![0, 0, 0, 14, 0, 0, 0, 15, 0, 7];
    intent.death_cell.spatial_position = second_cell.clone();
    let revision: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(SECOND_CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    let gameplay = CurrentCharacterGameplayFence {
        character_id: f.character_id,
        game_session_id: f.game_session_id,
        connection_generation: f.connection_generation,
        character_lease_generation: f.character_lease_generation,
        runtime_scope: f.runtime_scope,
        scope_ownership_generation: f.scope_ownership_generation,
        expected_character_revision: CharacterRevision::new(revision.parse()?).map_err(debug)?,
    };
    // Slot zero is an existing historical primitive; the current native
    // selector chooses equipment/main roots, not arbitrary direct entries.
    let committed = match h
        .root
        .commit_character_death(&authority, &h.node, gameplay, intent)
        .await
        .map_err(debug)?
    {
        CharacterDeathOutcome::Committed(value) => value,
        other => {
            return Err(format!("expected new historical primitive death, got {other:?}").into());
        }
    };
    assert!(committed.items.is_none());
    let permit = SpellLanePermit::of_fresh_scope(f.runtime_scope).await?;
    let cause = PlayerDeathDropCause {
        death_occurrence: id(171),
        item_instance: selected,
    };
    let audits_before = h.count("game_item_audit_outbox").await?;
    assert_eq!(
        h.root
            .commit_player_death_drop(&permit, &authority, &h.node, f, cause, request(184))
            .await?,
        PlayerDeathItemOutcome::Written
    );
    assert_eq!(nested_death_snapshot(&h, selected).await?, before);
    let source_slot: i16 = sqlx::query_scalar("SELECT source_slot FROM game_player_death_item_records WHERE death_occurrence_id=encode($1,'hex')::uuid AND item_instance_id=encode($2,'hex')::uuid")
        .bind(id(171).as_slice()).bind(selected.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(source_slot, 0);
    assert_eq!(h.count("game_player_death_item_records").await?, 2);
    assert_eq!(h.count("game_item_container_entries").await?, 0);
    assert_eq!(h.count("game_item_nested_entries").await?, 2);
    let position: Vec<u8> = sqlx::query_scalar("SELECT spatial_position FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(selected.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(position, second_cell);
    for item in &items {
        let owner: Option<String> = sqlx::query_scalar(
            "SELECT owner_character::text FROM game_item_loop_owner(encode($1,'hex')::uuid)",
        )
        .bind(item.as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(owner, None);
    }
    let written = nested_death_snapshot(&h, [0; 16]).await?;
    assert_eq!(
        h.root
            .commit_player_death_drop(&permit, &authority, &h.node, f, cause, request(184))
            .await?,
        PlayerDeathItemOutcome::Replay
    );
    assert_eq!(nested_death_snapshot(&h, [0; 16]).await?, written);
    assert_eq!(h.count("game_player_death_item_records").await?, 2);
    assert_eq!(h.count("game_item_audit_outbox").await?, audits_before + 1);
    drop(permit);
    assert!(
        h.root
            .consume_pending_respawn(
                &authority,
                &h.node,
                CurrentCharacterGameplayFence {
                    expected_character_revision: committed.committed_character_revision,
                    ..gameplay
                },
                Some(committed.occurrence)
            )
            .await
            .map_err(debug)?
            .is_some()
    );
    drop(authority);
    drop(seal);
    h.cleanup().await
}

#[test]
fn player_death_nested_historical_repickup_remains_a_legal_slot_zero_primitive() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime()
                .map_err(debug)?
                .block_on(run_historical_slot_zero_death(admin))
                .map_err(debug)
        })?
        .join()
        .map_err(|_| "historical slot-zero death thread panicked")?
        .map_err(Into::into)
}

async fn run_selected_quiver_death_reset(admin: String, slot: u8) -> TestResult {
    use crate::durability::character_build::{
        BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
    };
    use crate::durability::character_progression::CharacterProgressionError;
    use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
    use crate::durability::item_loop_move::{LoopDestination, LoopMoveRequest};
    use crate::durability::{DurabilityError, SqlxFailureClass};
    use crate::item_transfer_postgres_cases::uuid_text;
    use oteryn_gameplay_spell::spell::mana_training::CompiledTrainingFormula;

    assert!(matches!(slot, 6 | 10));
    let h = Harness::create(
        admin,
        if slot == 6 {
            "death_selected_quiver_left"
        } else {
            "death_selected_quiver_ammo"
        },
    )
    .await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let main = h.mint(&authority, BACKPACK, 1).await?;
    h.committed(&authority, to_slot(command(1)?, main, backpack_facts()))
        .await?;
    initialize(&h, &authority).await?;
    let quiver = pickup_quiver_fixture_item(&h, &authority, DEATH_QUIVER, 2).await?;
    let arrow = pickup_quiver_fixture_item(&h, &authority, DEATH_ARROW, 3).await?;
    let f = fence()?;
    let content = QuiverDeathContent;
    let permit = SpellLanePermit::of_fresh_scope(f.runtime_scope).await?;
    let moved = h
        .root
        .commit_item_loop_move(
            &permit,
            &authority,
            &h.node,
            &f,
            command(40)?,
            &content,
            LoopMoveRequest {
                item: arrow,
                count: None,
                pickup_cell: None,
                destination: LoopDestination::Container(quiver),
            },
        )
        .await?
        .map_err(debug)?;
    assert_eq!(moved.operation, 1);
    assert!(!moved.replayed);
    drop(permit);
    let equipped = h
        .root
        .commit_equipment_move(
            &authority,
            &h.node,
            &f,
            command(41)?,
            &content,
            EquipmentMove {
                item: quiver,
                from_slot: None,
                to_slot: Some(slot),
            },
        )
        .await?
        .map_err(debug)?;
    assert!(!equipped.replayed);
    let equipped_id: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_character_equipment_slots WHERE character_id=encode($1,'hex')::uuid AND slot=$2")
        .bind(f.character_id.as_bytes().as_slice()).bind(i16::from(slot)).fetch_one(&h.pool).await?;
    assert_eq!(equipped_id, quiver);
    let arrow_before: String = sqlx::query_scalar("SELECT to_jsonb(i)::text FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(arrow.as_slice()).fetch_one(&h.pool).await?;
    let edge_before: String = sqlx::query_scalar("SELECT to_jsonb(e)::text FROM game_item_nested_entries e WHERE item_instance_id=encode($1,'hex')::uuid AND parent_item_instance_id=encode($2,'hex')::uuid")
        .bind(arrow.as_slice()).bind(quiver.as_slice()).fetch_one(&h.pool).await?;
    let formula = CompiledTrainingFormula::from_profile(
        include_bytes!("../../../../../tools/content-schema/native-gameplay/build-training.json"),
        "content-1",
    )
    .map_err(debug)?;
    let mut gameplay = CurrentCharacterGameplayFence {
        character_id: f.character_id,
        game_session_id: f.game_session_id,
        connection_generation: f.connection_generation,
        character_lease_generation: f.character_lease_generation,
        runtime_scope: f.runtime_scope,
        scope_ownership_generation: f.scope_ownership_generation,
        expected_character_revision: CharacterRevision::new(h.character_revision().await?.parse()?)
            .map_err(debug)?,
    };
    let build = DurableBuildState::default();
    h.root
        .commit_character_build(
            &authority,
            &h.node,
            gameplay,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(160)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                after: convert_vocation(&formula, &build, "druid").map_err(debug)?,
                before: build,
                pruned_stance: None,
            },
            &formula,
        )
        .await
        .map_err(debug)?;
    gameplay.expected_character_revision =
        CharacterRevision::new(h.character_revision().await?.parse()?).map_err(debug)?;
    let mut intent = death()?;
    intent.occurrence = quiver_loss_occurrence(main, quiver, slot)?;
    intent.death_cell.spatial_position = nested_death_cell();
    let committed = h
        .root
        .settle_runtime_player_death_with_items(
            &CharacterRevisionSequencer::new(),
            &authority,
            &h.node,
            gameplay,
            intent,
            Some(&formula),
            None,
            Some(DIGEST),
        )
        .await
        .map_err(debug)?;
    let selection = committed.items.as_ref().ok_or("selected quiver death")?;
    assert_eq!(selection.root, DIGEST);
    assert_eq!(selection.amulet, None);
    assert_eq!(selection.lost, vec![main, quiver]);
    let pending = CurrentCharacterGameplayFence {
        expected_character_revision: committed.committed_character_revision,
        ..gameplay
    };
    let quiver_cause = PlayerDeathDropCause {
        death_occurrence: *committed.occurrence.as_bytes(),
        item_instance: quiver,
    };
    let main_cause = PlayerDeathDropCause {
        item_instance: main,
        ..quiver_cause
    };
    let quiver_request = PlayerDeathItemRequest::selected(&committed, quiver, false)?;
    let main_request = PlayerDeathItemRequest::selected(&committed, main, false)?;
    let permit = SpellLanePermit::of_fresh_scope(f.runtime_scope).await?;
    let before_quiver_drop = nested_death_snapshot(&h, quiver).await?;
    let audits_before = h.count("game_item_audit_outbox").await?;
    assert_eq!(
        h.root
            .commit_player_death_drop(
                &permit,
                &authority,
                &h.node,
                f,
                quiver_cause,
                quiver_request.clone()
            )
            .await?,
        PlayerDeathItemOutcome::Written
    );
    assert_eq!(nested_death_snapshot(&h, quiver).await?, before_quiver_drop);
    let actual_slot: i16 = sqlx::query_scalar("SELECT source_slot FROM game_player_death_item_records WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(quiver.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(actual_slot, i16::from(slot));
    assert!(matches!(
        h.root
            .consume_pending_respawn(&authority, &h.node, pending, Some(committed.occurrence))
            .await,
        Err(CharacterProgressionError::Unavailable(
            DurabilityError::Database(SqlxFailureClass::Database(_))
        ))
    ));
    assert_eq!(h.count("game_character_pending_respawns").await?, 1);
    assert_eq!(
        h.root
            .commit_player_death_drop(
                &permit,
                &authority,
                &h.node,
                f,
                main_cause,
                main_request.clone()
            )
            .await?,
        PlayerDeathItemOutcome::Written
    );
    let arrow_after: String = sqlx::query_scalar("SELECT to_jsonb(i)::text FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(arrow.as_slice()).fetch_one(&h.pool).await?;
    let edge_after: String = sqlx::query_scalar("SELECT to_jsonb(e)::text FROM game_item_nested_entries e WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(arrow.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(arrow_after, arrow_before);
    assert_eq!(edge_after, edge_before);
    let owner: Option<String> = sqlx::query_scalar(
        "SELECT owner_character::text FROM game_item_loop_owner(encode($1,'hex')::uuid)",
    )
    .bind(arrow.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(owner, None);
    let written = nested_death_snapshot(&h, [0; 16]).await?;
    let grounds: Vec<String> = sqlx::query_scalar("SELECT to_jsonb(g)::text FROM game_item_ground_locations g ORDER BY item_instance_id LIMIT 3")
        .fetch_all(&h.pool).await?;
    assert_eq!(grounds.len(), 2);
    for (cause, request) in [(quiver_cause, quiver_request), (main_cause, main_request)] {
        assert_eq!(
            h.root
                .commit_player_death_drop(&permit, &authority, &h.node, f, cause, request)
                .await?,
            PlayerDeathItemOutcome::Replay
        );
    }
    assert_eq!(nested_death_snapshot(&h, [0; 16]).await?, written);
    let replay_grounds: Vec<String> = sqlx::query_scalar("SELECT to_jsonb(g)::text FROM game_item_ground_locations g ORDER BY item_instance_id LIMIT 3")
        .fetch_all(&h.pool).await?;
    assert_eq!(replay_grounds, grounds);
    assert_eq!(h.count("game_item_audit_outbox").await?, audits_before + 2);
    drop(permit);
    assert!(
        h.root
            .consume_pending_respawn(&authority, &h.node, pending, Some(committed.occurrence))
            .await
            .map_err(debug)?
            .is_some()
    );
    let reset = prepare_nested_reset(&h, &h.root).await?;
    observe_nested_retirement_order(&h).await?;
    let audits_before_reset = h.count("game_item_audit_outbox").await?;
    assert!(
        h.root
            .retire_world_reset_ground(&authority, &h.node, &reset)
            .await
            .map_err(debug)?
    );
    let order: Vec<String> = sqlx::query_scalar(
        "SELECT item::text FROM test_nested_retire_order ORDER BY position LIMIT 4",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(order.len(), 3);
    assert!(
        order
            .iter()
            .position(|item| item == &uuid_text(arrow))
            .ok_or("retired arrow")?
            < order
                .iter()
                .position(|item| item == &uuid_text(quiver))
                .ok_or("retired quiver")?
    );
    let (transactions, receipts): (i64, i64) = sqlx::query_as("SELECT count(DISTINCT created_xact_id),count(*) FROM game_item_decay_retire_receipts WHERE ground_root_item_instance_id=encode($1,'hex')::uuid")
        .bind(quiver.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!((transactions, receipts), (1, 2));
    assert_nested_retirement_audits(&h, quiver, &[(arrow, quiver)]).await?;
    assert_nested_retirement_audits(&h, main, &[]).await?;
    for item in [main, quiver, arrow] {
        assert_eq!(h.item_state(item).await?, (0, 2));
    }
    for relation in [
        "game_item_ground_locations",
        "game_item_container_slots",
        "game_character_equipment_slots",
        "game_item_container_entries",
        "game_item_nested_entries",
    ] {
        assert_eq!(h.count(relation).await?, 0);
    }
    assert_eq!(h.count("game_item_decay_retire_receipts").await?, 3);
    assert_eq!(
        h.count("game_item_audit_outbox").await?,
        audits_before_reset + 3
    );
    let retired = nested_death_snapshot(&h, [0; 16]).await?;
    assert!(
        h.root
            .retire_world_reset_ground(&authority, &h.node, &reset)
            .await
            .map_err(debug)?
    );
    assert_eq!(nested_death_snapshot(&h, [0; 16]).await?, retired);
    assert_eq!(h.count("game_item_decay_retire_receipts").await?, 3);
    assert_eq!(
        h.count("game_item_audit_outbox").await?,
        audits_before_reset + 3
    );
    assert_eq!(h.count("test_nested_retire_order").await?, 3);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

fn selected_quiver_death_case(slot: u8) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime()
                .map_err(debug)?
                .block_on(run_selected_quiver_death_reset(admin, slot))
                .map_err(debug)
        })?
        .join()
        .map_err(|_| "selected quiver death/reset thread panicked")?
        .map_err(Into::into)
}

#[test]
fn player_death_nested_selected_left_quiver_retires_with_ammunition() -> TestResult {
    selected_quiver_death_case(6)
}

#[test]
fn player_death_nested_selected_extra_quiver_retires_with_ammunition() -> TestResult {
    selected_quiver_death_case(10)
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
