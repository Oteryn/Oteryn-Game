//! Physical M49 successor: real MINT, carried custody, drop, retirement and replay.
mod fixture {
    pub(super) use crate::bestiary_postgres_harness::{
        CHANNEL, CHARACTER, Harness, SESSION, TestResult, WORLD, configured_admin, debug, id,
        runtime, scope,
    };
    pub const BACKPACK: &str = "fixture:b3.backpack";
    pub fn fence() -> TestResult<crate::durability::item_transfer::CurrentCharacterItemFence> {
        crate::bestiary_postgres_harness::item_fence()
    }
    pub fn definition(family: &str, key: &str) -> crate::durability::item_mint::TypedDefinitionRef {
        crate::durability::item_mint::TypedDefinitionRef {
            family: family.into(),
            production_key: key.into(),
            revision_ref: "definition-r1".into(),
        }
    }
    pub fn command(n: u64) -> TestResult<crate::foundation::CommandRef> {
        Ok(crate::foundation::CommandRef::new(
            crate::foundation::GameSessionId::decode(&id(SESSION)).map_err(debug)?,
            crate::foundation::CommandId::new(n).map_err(debug)?,
        ))
    }
    pub fn to_slot(
        command: crate::foundation::CommandRef,
        item: [u8; 16],
        facts: crate::durability::item_transfer::ItemDefinitionFacts,
    ) -> crate::durability::item_transfer::ItemTransferRequest {
        crate::durability::item_transfer::ItemTransferRequest {
            weight_generation_digest: [0xD1; 32],
            command,
            source_item_instance_id: item,
            destination: crate::durability::item_transfer::ItemTransferDestination::ContainerSlot,
            item: facts,
            backpack: None,
            content_revision: "content-1".into(),
            ruleset_revision: "ruleset-1".into(),
            sim_revision: "sim-1".into(),
        }
    }
    pub fn to_backpack(
        command: crate::foundation::CommandRef,
        item: [u8; 16],
        facts: crate::durability::item_transfer::ItemDefinitionFacts,
    ) -> crate::durability::item_transfer::ItemTransferRequest {
        crate::durability::item_transfer::ItemTransferRequest {
            destination: crate::durability::item_transfer::ItemTransferDestination::MainBackpack,
            backpack: Some(super::facts(BACKPACK)),
            ..to_slot(command, item, facts)
        }
    }
}
use crate::durability::item_loop_move::{
    ElementalArrivalTarget, ItemLoopContent, LoopDestination, LoopGround, LoopMoveRequest,
};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{
    ItemDefinitionFacts, ItemStackClass, ItemTransferOutcome, ItemWeightLookup, ItemWeightRefused,
};
use crate::durability::spell_owner_commit::SpellLanePermit;
use fixture as f;
use oteryn_gameplay_quests::quest::elemental_arrival as rules;

#[path = "qn1_elemental_consume/projection.rs"]
mod projection;

struct Content {
    ground_only: bool,
}
impl ItemWeightLookup for Content {
    fn generation_digest(&self) -> [u8; 32] {
        [0xD1; 32]
    }
    fn weight_of(&self, definition: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        if definition.family == "Item"
            && definition.revision_ref == "definition-r1"
            && matches!(
                definition.production_key.as_str(),
                f::BACKPACK | "oteryn:item.tibia.i8139"
            )
        {
            Ok(1)
        } else {
            Err(ItemWeightRefused::Unknown)
        }
    }
}
impl ItemLoopContent for Content {
    fn loop_content_digest(&self) -> [u8; 32] {
        [1; 32]
    }
    fn loop_facts(&self, definition: &TypedDefinitionRef) -> Option<ItemDefinitionFacts> {
        matches!(
            definition.production_key.as_str(),
            f::BACKPACK | "oteryn:item.tibia.i8139"
        )
        .then(|| {
            let mut facts = facts(&definition.production_key);
            if self.ground_only && definition.production_key == "oteryn:item.tibia.i8139" {
                facts.stack = ItemStackClass::Unknown;
            }
            facts
        })
    }
    fn loop_native_corpse_facts(
        &self,
        definition: &TypedDefinitionRef,
    ) -> Option<ItemDefinitionFacts> {
        (definition.production_key == "oteryn:item.tibia.i8139")
            .then(|| facts(&definition.production_key))
    }
}
fn facts(key: &str) -> ItemDefinitionFacts {
    ItemDefinitionFacts {
        definition: f::definition("Item", key),
        weight: Some(1),
        stack: ItemStackClass::NonStackable,
        container_capacity: if key == f::BACKPACK {
            Some(20)
        } else {
            Some(14)
        },
        container_slot_equip_pattern: key == f::BACKPACK,
    }
}
#[test]
fn qn1_elemental_consumption_shares_drop_transaction_and_replays_once() -> f::TestResult {
    run(false)
}
#[test]
fn qn1_elemental_native_corpse_arrival_postgres() -> f::TestResult {
    run(true)
}
fn run(native: bool) -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let start = std::time::Instant::now();
                    let h = f::Harness::create(
                        admin,
                        if native {
                            "qn1_elemental_native"
                        } else {
                            "qn1_elemental_consume"
                        },
                        true,
                    )
                    .await?;
                    let result = exercise(&h, native).await;
                    h.cleanup().await?;
                    result?;
                    assert!(start.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "QN1_ELEMENTAL_CONSUME_PG_EXECUTED duration={:?}",
                        start.elapsed()
                    );
                    if native {
                        eprintln!(
                            "QN1_ELEMENTAL_NATIVE_CORPSE_PG_EXECUTED duration={:?}",
                            start.elapsed()
                        );
                    }
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "elemental consumption PG thread failed")?
        .map_err(Into::into)
}
async fn exercise(h: &f::Harness, native: bool) -> f::TestResult {
    let content = Content {
        ground_only: native,
    };
    use crate::durability::character_equipment::{
        assert_equipment_authority_in_transaction, initialize_equipment_in_transaction,
    };
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QN1_ELEMENTAL_CONSUME_PG_READY PostgreSQL {version}");
    if native {
        eprintln!("QN1_ELEMENTAL_NATIVE_CORPSE_PG_READY PostgreSQL {version}");
    }
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    for (n, key) in [(1, f::BACKPACK), (2, "oteryn:item.tibia.i8139")] {
        if native && n == 2 {
            continue;
        }
        let item = mint(h, &authority, n, key).await?;
        let request = if n == 1 {
            f::to_slot(f::command(n)?, item, facts(key))
        } else {
            f::to_backpack(f::command(n)?, item, facts(key))
        };
        let mut candidate = h
            .root
            .freeze_item_transfer(&authority, &h.node, f::fence()?, request, &content)
            .await
            .map_err(f::debug)?;
        let outcome = h
            .root
            .commit_item_transfer(
                &SpellLanePermit::of_fresh_scope(f::scope()?)
                    .await
                    .map_err(f::debug)?,
                &authority,
                &h.node,
                f::fence()?,
                &mut candidate,
                &content,
            )
            .await
            .map_err(f::debug)?;
        assert!(matches!(outcome, ItemTransferOutcome::Committed(_)));
    }
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(f::id(f::WORLD).as_slice()).bind(f::id(f::CHANNEL).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(f::id(f::WORLD).as_slice()).bind(f::id(f::CHANNEL).as_slice()).bind([1u8;32].as_slice()).bind([2u8;32].as_slice()).bind([3u8;32].as_slice()).execute(&h.pool).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let equipment = assert_equipment_authority_in_transaction(
        &mut tx,
        &h.root,
        &authority,
        &h.node,
        &f::fence()?,
        f::command(90)?,
        [1; 32],
    )
    .await
    .map_err(f::debug)?;
    initialize_equipment_in_transaction(&mut tx, &equipment)
        .await
        .map_err(f::debug)?;
    tx.commit().await?;
    if native {
        let mut request = mint_request(
            2,
            "oteryn:item.tibia.i8139",
            crate::durability::item_mint::CORPSE_MATERIALIZATION_PURPOSE_KEY,
        )?;
        request.ground.map_revision = format!("sha256:{}", "07".repeat(32));
        request.ground.content_revision = format!("sha256:{}", "01".repeat(32));
        request.ground.native_room_placement_context = vec![3; 32];
        request.content_revision = request.ground.content_revision.clone();
        let mut candidate = h
            .root
            .freeze_item_mint(&authority, &h.node, request)
            .await
            .map_err(f::debug)?;
        let permit = candidate.fresh_death_lane_permit().await;
        let outcome = h
            .root
            .commit_corpse_mint(
                &permit,
                &authority,
                &h.node,
                &mut candidate,
                f::id(f::CHARACTER),
            )
            .await
            .map_err(f::debug)?;
        assert!(matches!(
            outcome,
            crate::durability::item_mint::ItemMintOutcome::Committed(_)
        ));
    }
    let (item,revision):(Vec<u8>,String)=sqlx::query_as("SELECT decode(replace(item_instance_id::text,'-',''),'hex'),state_revision::text FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i8139' AND lifecycle=1").fetch_one(&h.pool).await?;
    let item: [u8; 16] = item.try_into().map_err(|_| "corpse identity")?;
    let recipe = &rules::RECIPES[0];
    let position = recipe.summon;
    let target_key = crate::map::view::placement_key(
        crate::map::overlay::TilePos {
            x: position.x,
            y: position.y,
            floor: position.floor,
        },
        1,
    )
    .ok_or("altar key")?;
    let target = rules::ObjectSnapshot {
        key: target_key,
        revision: 0,
        position,
        appearance: 7738,
    };
    let corpse = rules::CorpseArrival {
        instance: item,
        state_revision: revision.parse::<u64>()? + 1,
        item: 8139,
        quantity: 1,
        position,
    };
    let cells: Vec<_> = recipe
        .positions
        .iter()
        .zip(1..=8)
        .map(|(position, key)| rules::ObjectSnapshot {
            key,
            revision: 0,
            position: *position,
            appearance: 2126,
        })
        .collect();
    let plan = rules::prepare_arrival(
        rules::ArrivalFence {
            world: f::id(f::WORLD),
            channel: f::id(f::CHANNEL),
            scope_generation: 1,
            map: [7; 32],
            content: [1; 32],
            reset_epoch: 0,
        },
        target,
        corpse,
        &cells,
        false,
    )
    .ok_or("source ring")?;
    let mut spatial = i32::from(position.x).to_be_bytes().to_vec();
    spatial.extend(i32::from(position.y).to_be_bytes());
    spatial.extend(i16::from(position.floor).to_be_bytes());
    let request = LoopMoveRequest {
        item,
        count: None,
        pickup_cell: native.then(|| vec![0, 0, 0, 1, 0, 0, 0, 2, 0, 3]),
        destination: LoopDestination::Ground(LoopGround {
            spatial_position: spatial,
            map_revision: format!("sha256:{}", "07".repeat(32)),
            content_revision: format!("sha256:{}", "01".repeat(32)),
            placement_context: vec![3; 32],
        }),
    };
    let mut permit = SpellLanePermit::of_fresh_scope(f::scope()?)
        .await
        .map_err(f::debug)?;
    if native {
        let pickup = h
            .root
            .commit_item_loop_move(
                &permit,
                &authority,
                &h.node,
                &f::fence()?,
                f::command(99)?,
                &content,
                LoopMoveRequest {
                    destination: LoopDestination::MainBackpack,
                    ..request.clone()
                },
            )
            .await
            .map_err(f::debug)?;
        assert!(matches!(
            pickup,
            Err(crate::durability::item_loop_move::LoopMoveRefusal::NotSupported)
        ));
    }
    let mut stale_connection = f::fence()?;
    stale_connection.connection_generation =
        crate::foundation::ConnectionGeneration::new(2).map_err(f::debug)?;
    let mut stale_lease = f::fence()?;
    stale_lease.character_lease_generation = 2;
    let mut stale_scope = f::fence()?;
    stale_scope.scope_ownership_generation =
        crate::foundation::ScopeOwnershipGeneration::new(2).map_err(f::debug)?;
    for stale_fence in [stale_connection, stale_lease, stale_scope] {
        let refused = h
            .root
            .item_loop_elemental_corpse(
                &permit,
                &authority,
                &h.node,
                stale_fence,
                &content,
                crate::durability::item_loop_move::ElementalArrivalProbe {
                    command: f::command(100)?,
                    item,
                    position,
                },
            )
            .await
            .map_err(f::debug)?;
        assert_eq!(refused, None);
    }
    let observed = h
        .root
        .item_loop_elemental_corpse(
            &permit,
            &authority,
            &h.node,
            f::fence()?,
            &content,
            crate::durability::item_loop_move::ElementalArrivalProbe {
                command: f::command(100)?,
                item,
                position,
            },
        )
        .await
        .map_err(f::debug)?
        .ok_or("missing physical corpse probe")?;
    assert_eq!(observed, corpse);
    let stale_plan = rules::prepare_arrival(
        plan.fence(),
        target,
        rules::CorpseArrival {
            state_revision: corpse.state_revision + 1,
            ..corpse
        },
        &cells,
        false,
    )
    .ok_or("well-formed stale physical revision")?;
    let mut window = permit.open_commit_window((), |attempt| Box::new(attempt));
    let stale = h
        .root
        .commit_item_loop_elemental_arrival(
            &mut window,
            &authority,
            &h.node,
            f::fence()?,
            f::command(100)?,
            &content,
            request.clone(),
            ElementalArrivalTarget::new(stale_plan),
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(
        stale,
        Err(crate::durability::item_loop_move::LoopMoveRefusal::Stale)
    ));
    assert!(!window.commit_called());
    window.release();
    let unchanged: (String, i64) = sqlx::query_as(
        "SELECT state_revision::text,quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid"
    ).bind(item.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(unchanged, (revision, 1));
    let intents: i64 = sqlx::query_scalar("SELECT count(*) FROM game_elemental_summon_intents")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(intents, 0);
    assert!(
        h.root
            .reconcile_item_loop_move(
                &authority,
                f::fence()?.character_id,
                f::command(100)?,
                request.clone()
            )
            .await
            .map_err(f::debug)?
            .is_none()
    );
    // Absence must wait for a writer holding this CommandRef's actual lock.
    let mut inflight = h.pool.begin().await?;
    let mut command_key = f::command(100)?.game_session_id().as_bytes().to_vec();
    command_key.extend_from_slice(&100_u64.to_be_bytes());
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:item-loop-move:' || encode($1, 'hex'), 0))")
        .bind(command_key).execute(&mut *inflight).await?;
    let pending = h.root.reconcile_item_loop_move(
        &authority,
        f::fence()?.character_id,
        f::command(100)?,
        request.clone(),
    );
    tokio::pin!(pending);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(30), &mut pending)
            .await
            .is_err()
    );
    inflight.rollback().await?;
    assert!(pending.await.map_err(f::debug)?.is_none());
    let mut window = permit.open_commit_window((), |attempt| Box::new(attempt));
    let result = h
        .root
        .commit_item_loop_elemental_arrival(
            &mut window,
            &authority,
            &h.node,
            f::fence()?,
            f::command(100)?,
            &content,
            request.clone(),
            ElementalArrivalTarget::new(plan.clone()),
        )
        .await
        .map_err(f::debug)?;
    let receipt = result.map_err(|e| format!("arrival refused {e:?}"))?;
    assert!(!receipt.replayed);
    assert!(
        window.commit_called(),
        "arrival COMMIT escaped its caller window"
    );
    window.install();
    let recovered = h
        .root
        .reconcile_item_loop_move(
            &authority,
            f::fence()?.character_id,
            f::command(100)?,
            request.clone(),
        )
        .await
        .map_err(f::debug)?
        .ok_or("missing committed arrival during reconciliation")?
        .map_err(f::debug)?;
    assert!(recovered.replayed);
    assert_eq!(recovered.transaction_id, receipt.transaction_id);
    assert_eq!(recovered.event_id, receipt.event_id);
    let mut conflicting = request.clone();
    conflicting.destination = LoopDestination::MainBackpack;
    assert_eq!(
        h.root
            .reconcile_item_loop_move(
                &authority,
                f::fence()?.character_id,
                f::command(100)?,
                conflicting
            )
            .await
            .map_err(f::debug)?,
        Some(Err(
            crate::durability::item_loop_move::LoopMoveRefusal::Rejected
        ))
    );
    let state:(i64,i16)=sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(item.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(state, (0, 2));
    let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_elemental_arrival_receipts),(SELECT count(*) FROM game_elemental_summon_intents),(SELECT count(*) FROM game_elemental_boss_leases)").fetch_one(&h.pool).await?;
    assert_eq!(counts, (1, 1, 1));
    let on_ground:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid)").bind(item.as_slice()).fetch_one(&h.pool).await?;
    assert!(!on_ground);
    let mut window = permit.open_commit_window((), |attempt| Box::new(attempt));
    let replay = h
        .root
        .commit_item_loop_elemental_arrival(
            &mut window,
            &authority,
            &h.node,
            f::fence()?,
            f::command(100)?,
            &content,
            request,
            ElementalArrivalTarget::new(plan),
        )
        .await
        .map_err(f::debug)?
        .map_err(|e| format!("replay refused {e:?}"))?;
    assert!(replay.replayed);
    assert!(
        window.commit_called(),
        "historical arrival did not retain its attempt"
    );
    window.install();
    assert_eq!(replay.transaction_id, receipt.transaction_id);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_elemental_summon_intents")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(count, 1);
    let retired = h
        .root
        .item_loop_elemental_corpse(
            &permit,
            &authority,
            &h.node,
            f::fence()?,
            &content,
            crate::durability::item_loop_move::ElementalArrivalProbe {
                command: f::command(101)?,
                item,
                position,
            },
        )
        .await
        .map_err(f::debug)?;
    assert_eq!(retired, None);
    let projection = crate::durability::item_loop_move::ElementalSummonScope {
        map: [7; 32],
        reset_epoch: 0,
    };
    let pending = h
        .root
        .read_elemental_summons(
            &permit,
            &authority,
            &h.node,
            f::fence()?,
            &content,
            projection,
        )
        .await
        .map_err(f::debug)?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].altar, recipe.altar);
    assert_eq!(pending[0].creature_key, recipe.creature);
    assert_eq!(pending[0].position, position);
    let claim = crate::durability::item_loop_move::ElementalSummonClaim {
        scope: projection,
        transaction_id: pending[0].transaction_id,
    };
    let claimed = h
        .root
        .claim_elemental_summon(&permit, &authority, &h.node, f::fence()?, &content, claim)
        .await
        .map_err(f::debug)?
        .ok_or("live arrival clock claim")?;
    assert_eq!(claimed.intent, pending[0]);
    assert_eq!(claimed.cycle, 0);
    assert_eq!(claimed.owner_generation.get(), 1);
    let mut scene = projection::start(h, &claimed)?;
    let owner = crate::durability::item_decay_retire::CorpseDecayFence {
        runtime_scope: f::scope()?,
        scope_ownership_generation: crate::foundation::ScopeOwnershipGeneration::new(1)
            .map_err(f::debug)?,
    };
    let background = crate::durability::item_loop_move::ElementalSummonProjection {
        scope: projection,
        content: [1; 32],
        frame: [3; 32],
    };
    assert_eq!(
        h.root
            .recover_elemental_summons(&permit, &authority, &h.node, owner, background)
            .await
            .map_err(f::debug)?,
        vec![claimed.clone()]
    );
    for invalid in [
        crate::durability::item_loop_move::ElementalSummonProjection {
            content: [2; 32],
            ..background
        },
        crate::durability::item_loop_move::ElementalSummonProjection {
            frame: [4; 32],
            ..background
        },
        crate::durability::item_loop_move::ElementalSummonProjection {
            scope: crate::durability::item_loop_move::ElementalSummonScope {
                map: [8; 32],
                ..projection
            },
            ..background
        },
        crate::durability::item_loop_move::ElementalSummonProjection {
            scope: crate::durability::item_loop_move::ElementalSummonScope {
                reset_epoch: 1,
                ..projection
            },
            ..background
        },
    ] {
        assert!(
            h.root
                .recover_elemental_summons(&permit, &authority, &h.node, owner, invalid)
                .await
                .is_err(),
            "invalid owner projection must not look like an empty live set"
        );
    }
    let stale_owner = crate::durability::item_decay_retire::CorpseDecayFence {
        scope_ownership_generation: crate::foundation::ScopeOwnershipGeneration::new(2)
            .map_err(f::debug)?,
        ..owner
    };
    assert!(
        h.root
            .recover_elemental_summons(&permit, &authority, &h.node, stale_owner, background)
            .await
            .is_err()
    );
    let spawn: String = sqlx::query_scalar("SELECT 'qn1.elemental:'||encode($1,'hex')")
        .bind(pending[0].transaction_id.as_slice())
        .fetch_one(&h.pool)
        .await?;
    let clock: Option<(i16, String)> = sqlx::query_as(
        "SELECT state,owner_generation::text FROM game_boss_spawn_clocks WHERE spawn_key=$1",
    )
    .bind(&spawn)
    .fetch_optional(&h.pool)
    .await?;
    assert_eq!(
        clock,
        Some((2, "1".into())),
        "arrival must persist its native boss clock"
    );
    for mismatched in [
        crate::durability::item_loop_move::ElementalSummonScope {
            map: [8; 32],
            ..projection
        },
        crate::durability::item_loop_move::ElementalSummonScope {
            reset_epoch: 1,
            ..projection
        },
    ] {
        assert!(
            h.root
                .read_elemental_summons(
                    &permit,
                    &authority,
                    &h.node,
                    f::fence()?,
                    &content,
                    mismatched,
                )
                .await
                .map_err(f::debug)?
                .is_empty()
        );
    }
    for replaced in [stale_connection, stale_lease, stale_scope] {
        assert!(
            h.root
                .read_elemental_summons(
                    &permit, &authority, &h.node, replaced, &content, projection
                )
                .await
                .map_err(f::debug)?
                .is_empty()
        );
        assert!(
            h.root
                .claim_elemental_summon(&permit, &authority, &h.node, replaced, &content, claim,)
                .await
                .map_err(f::debug)?
                .is_none()
        );
    }
    let refused = sqlx::query("DELETE FROM game_elemental_boss_leases")
        .execute(&h.pool)
        .await;
    assert!(
        matches!(refused, Err(sqlx::Error::Database(ref error)) if error.code().as_deref()==Some("23514"))
    );
    let mut unproven = h.pool.begin().await?;
    sqlx::query("UPDATE game_boss_spawn_clocks SET state=3,cycle=cycle+1,owner_generation=NULL WHERE spawn_key=$1")
        .bind(&spawn).execute(&mut *unproven).await?;
    let refused = unproven.commit().await;
    assert!(
        matches!(refused, Err(sqlx::Error::Database(ref error)) if error.code().as_deref()==Some("23514"))
    );
    scene.die()?;
    let mut death = h
        .root
        .freeze_item_mint(
            &authority,
            &h.node,
            mint_request(
                3,
                "oteryn:item.tibia.i8139",
                crate::durability::item_mint::CORPSE_MATERIALIZATION_PURPOSE_KEY,
            )?,
        )
        .await
        .map_err(f::debug)?;
    let policy = oteryn_gameplay_content::content::BossPolicy::AtReset {
        chance: oteryn_gameplay_content::content::BossChance {
            numerator: 0,
            denominator: 1,
        },
    };
    let outcome = h
        .root
        .commit_boss_corpse_mint(
            &permit,
            &authority,
            &h.node,
            &mut death,
            f::id(f::CHARACTER),
            (spawn.clone(), 0, policy),
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(
        outcome,
        crate::durability::item_mint::ItemMintOutcome::Committed(_)
    ));
    assert!(
        h.root
            .read_elemental_summons(
                &permit,
                &authority,
                &h.node,
                f::fence()?,
                &content,
                projection,
            )
            .await
            .map_err(f::debug)?
            .is_empty()
    );
    let terminal: (i16, String) =
        sqlx::query_as("SELECT state,cycle::text FROM game_boss_spawn_clocks WHERE spawn_key=$1")
            .bind(&spawn)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(terminal, (3, "1".into()));
    let finished = h
        .root
        .recover_elemental_summons(&permit, &authority, &h.node, owner, background)
        .await
        .map_err(f::debug)?;
    assert!(finished.is_empty());
    scene.finish(&finished)?;
    assert!(
        h.root
            .claim_elemental_summon(&permit, &authority, &h.node, f::fence()?, &content, claim,)
            .await
            .map_err(f::debug)?
            .is_none(),
        "a cached intent cannot resurrect its dead boss"
    );
    let outcome = h
        .root
        .commit_boss_corpse_mint(
            &permit,
            &authority,
            &h.node,
            &mut death,
            f::id(f::CHARACTER),
            (spawn.clone(), 0, policy),
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(
        outcome,
        crate::durability::item_mint::ItemMintOutcome::AlreadyCommitted(_)
    ));
    Ok(())
}

async fn mint(
    h: &f::Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    n: u64,
    key: &str,
) -> f::TestResult<[u8; 16]> {
    use crate::durability::item_mint::ItemMintOutcome;
    let mut candidate = h
        .root
        .freeze_item_mint(
            authority,
            &h.node,
            mint_request(n, key, "fixture:elemental")?,
        )
        .await
        .map_err(f::debug)?;
    let permit = candidate.fresh_death_lane_permit().await;
    match h
        .root
        .commit_item_mint(&permit, authority, &h.node, &mut candidate)
        .await
        .map_err(f::debug)?
    {
        ItemMintOutcome::Committed(committed) => Ok(committed.item_instance_id),
        other => Err(format!("corpse fixture MINT {other:?}").into()),
    }
}

fn mint_request(
    n: u64,
    key: &str,
    purpose: &str,
) -> f::TestResult<crate::durability::item_mint::ItemMintRequest> {
    use crate::durability::item_mint::{GroundPlacement, ItemMintCause, ItemMintRequest};
    use crate::foundation::{ChannelId, ScopeOwnershipGeneration, WorldId};
    Ok(ItemMintRequest {
        cause: ItemMintCause::for_test(
            WorldId::decode(&f::id(f::WORLD)).map_err(f::debug)?,
            ChannelId::decode(&f::id(f::CHANNEL)).map_err(f::debug)?,
            ScopeOwnershipGeneration::new(1).map_err(f::debug)?,
            u32::try_from(n)?,
            1,
            f::definition("LootTable", "fixture:elemental"),
            purpose.into(),
            0,
        ),
        item: f::definition("Item", key),
        quantity: 1,
        ground: GroundPlacement {
            spatial_position: vec![0, 0, 0, 1, 0, 0, 0, 2, 0, 3],
            corpse_ref: f::id(3).to_vec(),
            map_revision: "map-1".into(),
            content_revision: "content-1".into(),
            native_room_placement_context: f::id(6).to_vec(),
        },
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
    })
}
