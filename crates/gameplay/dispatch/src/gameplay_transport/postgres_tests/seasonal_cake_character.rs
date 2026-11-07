//! Cake marks and the native empty-container reward commit through real PostgreSQL owners.
use super::TestResult;
use super::fixture::{Harness, debug, fence, id, run};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::reward_claim_mint::{
    RewardClaimMintError, RewardClaimMintOutcome, RewardClaimMintRequest, RewardClaimRefusal,
};
use crate::durability::seasonal_cake::character::{
    CakeCharacterEffect, CakeCharacterEvent, CakeCharacterProgress, CakeCity, reward_claim_key,
    reward_source,
};
use crate::foundation::{CommandId, CommandRef, ConnectionGeneration};
use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};

const START: i64 = 1_771_650_000; // 2026-02-21 06:00 Europe/Berlin, source server save.
const NOW: i64 = START + 86_340;
const END: i64 = START + 6 * 86_400;

#[test]
fn srv7_cake_five_city_marks_receipts_stale_fence_and_two_annual_rewards() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("cake-character-pg", scenario)
}

async fn event(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    ordinal: u8,
    action: CakeCharacterEvent,
) -> TestResult<CakeCharacterProgress> {
    Ok(h.root
        .commit_cake_character_event(
            authority,
            &h.node,
            fence(1)?.item_fence(),
            [ordinal; 32],
            action,
            NOW,
        )
        .await
        .map_err(debug)?
        .ok_or("active Cake character")?)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create(admin, "cake_character", true).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let started = std::time::Instant::now();
        let mut seed = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *seed)
            .await?;
        sqlx::query("INSERT INTO game_seasonal_cake_worlds(world_id,event_start,event_end,state) VALUES(encode($1,'hex')::uuid,$2,$3,$4)")
            .bind(id(42).as_slice()).bind(START).bind(END)
            .bind(serde_json::json!({"stage":3,"substances":200,"golems":200,"layer":4,"layer_progress":0,
                "window_start":null,"window_end":null,"bonus_from":NOW+60,"bonus_until":NOW+60+604800,"revision":10}))
            .execute(&mut *seed).await?;
        seed.commit().await?;
        assert!(matches!(
            event(
                &h,
                &authority,
                1,
                CakeCharacterEvent::Extract(CakeCity::Carlin)
            )
            .await?
            .effect,
            CakeCharacterEffect::NoMark
        ));
        let armed = event(&h, &authority, 2, CakeCharacterEvent::Isle).await?;
        assert!(armed.isle_armed);
        let first = event(
            &h,
            &authority,
            3,
            CakeCharacterEvent::Extract(CakeCity::Carlin),
        )
        .await?;
        assert_eq!(first.visited_count(), 1);
        assert!(!first.isle_armed);
        // A raw runtime writer has neither the Node-possession source arm nor its receipt.
        let mut forged = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *forged)
            .await?;
        sqlx::query("UPDATE game_seasonal_cake_characters SET visited_cities=31 WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).execute(&mut *forged).await?;
        assert!(
            forged.commit().await.is_err(),
            "raw runtime cannot forge five city marks"
        );

        // Replaying a lost old Isle reply cannot re-arm after a later extraction committed.
        assert_eq!(
            event(&h, &authority, 2, CakeCharacterEvent::Isle).await?,
            armed
        );
        let stored: bool = sqlx::query_scalar("SELECT isle_armed FROM game_seasonal_cake_characters WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert!(!stored);
        assert_eq!(
            event(
                &h,
                &authority,
                4,
                CakeCharacterEvent::Extract(CakeCity::Thais)
            )
            .await?
            .effect,
            CakeCharacterEffect::NoMark
        );
        event(&h, &authority, 5, CakeCharacterEvent::Isle).await?;
        let duplicate = event(
            &h,
            &authority,
            6,
            CakeCharacterEvent::Extract(CakeCity::Carlin),
        )
        .await?;
        assert_eq!(duplicate.effect, CakeCharacterEffect::AlreadyVisited);
        assert!(duplicate.isle_armed);
        assert_eq!(
            event(
                &h,
                &authority,
                7,
                CakeCharacterEvent::Extract(CakeCity::Thais)
            )
            .await?
            .visited_count(),
            2
        );
        for (ordinal, city) in [
            (8, CakeCity::Edron),
            (10, CakeCity::Darashia),
            (12, CakeCity::LibertyBay),
        ] {
            event(&h, &authority, ordinal, CakeCharacterEvent::Isle).await?;
            event(
                &h,
                &authority,
                ordinal + 1,
                CakeCharacterEvent::Extract(city),
            )
            .await?;
        }
        let completed = event(&h, &authority, 14, CakeCharacterEvent::Inspect).await?;
        assert!(completed.completed());
        assert!(!completed.isle_armed);
        let completed_help = event(&h, &authority, 16, CakeCharacterEvent::Help).await?;
        assert!(completed_help.completed());
        assert!(
            !completed_help.started,
            "source completed Help preserves its previous started flag"
        );

        let mut forged = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *forged)
            .await?;
        sqlx::query("UPDATE game_seasonal_cake_characters SET item_reward_given=true,cupcake_reward_given=true WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).execute(&mut *forged).await?;
        assert!(
            forged.commit().await.is_err(),
            "raw runtime cannot forge annual reward flags"
        );
        let mut forged = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *forged)
            .await?;
        assert!(sqlx::query("UPDATE game_seasonal_cake_character_receipts SET result=result WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).execute(&mut *forged).await.is_err(),"source receipts are immutable");
        forged.rollback().await?;

        let mut stale = fence(1)?.item_fence();
        stale.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        assert!(
            h.root
                .commit_cake_character_event(
                    &authority,
                    &h.node,
                    stale,
                    [15; 32],
                    CakeCharacterEvent::Help,
                    NOW
                )
                .await
                .is_err()
        );
        assert!(
            h.root
                .commit_cake_character_event(
                    &authority,
                    &h.node,
                    fence(1)?.item_fence(),
                    [2; 32],
                    CakeCharacterEvent::Help,
                    NOW
                )
                .await
                .is_err()
        );
        deliver(&h, &authority, fence(1)?.item_fence()).await?;
        let grants: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key IN ('oteryn:achievement/cake_conqueror','oteryn:achievement/i_did_my_part')")
            .fetch_one(&h.pool).await?;
        assert_eq!(grants, 2);
        eprintln!(
            "SRV7_CAKE_CHARACTER_POSTGRES_CASE_COMPLETED elapsed={:?}",
            started.elapsed()
        );
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn deliver(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    fence: CurrentCharacterItemFence,
) -> TestResult {
    let (controller, _) = crate::content::native_gameplay::test_support::activated_with_loot_tables(
        fence.runtime_scope.world_id(),
        None,
    );
    let native = controller
        .active()
        .and_then(|a| a.native_gameplay())
        .ok_or("native")?;
    let index = native.item_index().ok_or("index")?;
    let facts = |source: u16| {
        let key = format!("oteryn:item.tibia.i{source}");
        let revision = index.revision(&key).ok_or("item revision")?;
        generation_item_facts(
            native,
            &TypedDefinitionRef {
                family: "Item".into(),
                production_key: key.clone(),
                revision_ref: revision.into(),
            },
        )
        .map_err(debug)
    };
    let backpack = facts(2854)?;
    let cake_backpack = facts(20347)?;
    assert_eq!(cake_backpack.container_capacity, Some(20));
    let request = |command, source, cupcake| -> TestResult<RewardClaimMintRequest> {
        Ok(RewardClaimMintRequest {
            command: CommandRef::new(
                fence.game_session_id,
                CommandId::new(command).map_err(debug)?,
            ),
            claim: TypedDefinitionRef {
                family: "RewardClaim".into(),
                production_key: reward_claim_key(START, cupcake),
                revision_ref: "cake-201189f8-r1".into(),
            },
            source_placement: reward_source(START),
            item: facts(source)?,
            quantity: 1,
            contents: Vec::new(),
            root_text: None,
            backpack: backpack.clone(),
            content_revision: format!(
                "sha256:{}",
                native
                    .source_digest()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            ),
            ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION.into(),
            sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
            achievement: None,
            quest_transition: None,
            key_number: None,
        })
    };
    // Marks are available immediately in stage 3, but Brutus' reward starts next save.
    assert!(
        h.root
            .claim_cake_character_reward(
                authority,
                &h.node,
                fence,
                request(100, 20347, false)?,
                NOW,
                &NativeItemWeights(native)
            )
            .await
            .map_err(debug)?
            .is_none()
    );
    // A missing destination rolls back native reservation and claim along with source flags.
    let missing = h
        .root
        .claim_cake_character_reward(
            authority,
            &h.node,
            fence,
            request(101, 20347, false)?,
            NOW + 60,
            &NativeItemWeights(native),
        )
        .await;
    assert!(
        matches!(
            missing,
            Err(RewardClaimMintError::Refused(
                RewardClaimRefusal::NoMainBackpack
            ))
        ),
        "{missing:?}"
    );
    let claims: i64 = sqlx::query_scalar("SELECT count(*) FROM game_reward_claims")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(claims, 0);
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(
        crate::durability::item_fee_burn::test_support::item(
            170,
            &backpack.definition.production_key,
            &backpack.definition.revision_ref,
            1,
        ),
    ))
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(170).as_slice()).bind(id(42).as_slice()).bind(id(171).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let original = request(102, 20347, false)?;
    let first = h
        .root
        .claim_cake_character_reward(
            authority,
            &h.node,
            fence,
            original.clone(),
            NOW + 60,
            &NativeItemWeights(native),
        )
        .await
        .map_err(debug)?
        .ok_or("Cake Backpack")?;
    let RewardClaimMintOutcome::Committed(first) = first else {
        return Err("first claim not fresh".into());
    };
    let replay = h
        .root
        .claim_cake_character_reward(
            authority,
            &h.node,
            fence,
            original,
            END + 1,
            &NativeItemWeights(native),
        )
        .await
        .map_err(debug)?;
    assert_eq!(
        replay,
        Some(RewardClaimMintOutcome::AlreadyCommitted(first))
    );
    assert!(
        h.root
            .claim_cake_character_reward(
                authority,
                &h.node,
                fence,
                request(103, 14756, false)?,
                NOW + 60,
                &NativeItemWeights(native)
            )
            .await
            .map_err(debug)?
            .is_none()
    );
    assert!(
        h.root
            .claim_cake_character_reward(
                authority,
                &h.node,
                fence,
                request(104, 28484, true)?,
                NOW + 60,
                &NativeItemWeights(native)
            )
            .await
            .map_err(debug)?
            .is_some()
    );
    assert!(
        h.root
            .claim_cake_character_reward(
                authority,
                &h.node,
                fence,
                request(105, 28486, true)?,
                NOW + 60,
                &NativeItemWeights(native)
            )
            .await
            .map_err(debug)?
            .is_none()
    );
    let flags:(bool,bool)=sqlx::query_as("SELECT item_reward_given,cupcake_reward_given FROM game_seasonal_cake_characters WHERE character_id=encode($1,'hex')::uuid AND event_start=$2")
        .bind(id(41).as_slice()).bind(START).fetch_one(&h.pool).await?;
    assert_eq!(flags, (true, true));
    let items:Vec<(String,i64)>=sqlx::query_as("SELECT definition_production_key,quantity FROM game_item_instances WHERE definition_production_key IN ('oteryn:item.tibia.i20347','oteryn:item.tibia.i28484') ORDER BY definition_production_key")
        .fetch_all(&h.pool).await?;
    assert_eq!(
        items,
        vec![
            ("oteryn:item.tibia.i20347".into(), 1),
            ("oteryn:item.tibia.i28484".into(), 1)
        ]
    );
    Ok(())
}

#[test]
fn srv7_brutus_native_npc_help_container_and_cupcake_replay() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("brutus-native-pg", npc_scenario)
}

fn npc_scenario() -> TestResult {
    use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
    use crate::foundation::{
        CharacterLease, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
    };
    use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
    use crate::gameplay_transport::npc_talk::NpcTalkBook;
    use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
    use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
    use std::sync::Arc;
    run(async |admin| {
        super::reset();
        let h = Harness::create_with_admission_seed(admin, "brutus_native", false, false).await?;
        super::prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = fence(1)?.runtime_scope.world_id();
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let room = pin.qualified_room().clone();
        let deps = super::Dependencies::with_content(
            pin.into_channel_pin(),
            room,
            native.spell_book().clone(),
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        let actor = deps.actor;
        let session = deps.admitted.game_session_id;
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    actor,
                    session,
                    CharacterLease::new(
                        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                        2,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
        }
        deps.initialize(&owner).await;
        let item_fence = owner
            .current_quest_fence(session)
            .await
            .map_err(debug)?
            .ok_or("quest fence")?
            .item_fence();
        let mut seed = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *seed)
            .await?;
        sqlx::query("INSERT INTO game_seasonal_cake_worlds(world_id,event_start,event_end,state) VALUES(encode($1,'hex')::uuid,$2,$3,$4)")
            .bind(id(42).as_slice()).bind(START).bind(END)
            .bind(serde_json::json!({"stage":3,"substances":200,"golems":200,"layer":4,"layer_progress":0,
                "window_start":null,"window_end":null,"bonus_from":NOW+60,"bonus_until":NOW+60+604800,"revision":10}))
            .execute(&mut *seed).await?;
        sqlx::query("INSERT INTO game_seasonal_cake_characters(character_id,world_id,event_start,visited_cities) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,0)")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(START).execute(&mut *seed).await?;
        seed.commit().await?;
        let index = native.item_index().ok_or("index")?;
        let bagkey = "oteryn:item.tibia.i2854";
        let bagrevision = index.revision(bagkey).ok_or("bag revision")?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(
            crate::durability::item_fee_burn::test_support::item(170, bagkey, bagrevision, 1),
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(170).as_slice()).bind(id(42).as_slice()).bind(id(171).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let catalogue = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?;
        let model = catalogue.service_model_for_project().map_err(debug)?;
        owner
            .states
            .chat
            .install_npc_talk(Arc::new(NpcTalkBook::from_model(&model)));
        owner
            .states
            .chat
            .admit(session, "Cake Hero", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let npc = {
            let mut runtime = deps.runtime.lock().await;
            let at = runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position();
            runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: "oteryn:npc.brutus_the_baker".into(),
                            placements: vec![NpcPlacement {
                                x: u16::try_from(at.x + 1)?,
                                y: u16::try_from(at.y)?,
                                floor: -i8::try_from(at.floor)?,
                                direction: MovementFacing::West,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            let row = runtime.visible_entities().npcs[0];
            EntityRef {
                identity: row.actor.placement_identity(),
                generation: row.generation,
            }
        };
        let _clock = crate::gameplay_transport::seasonal_cake::test_clock::set(NOW + 60);
        for (command_id, text, changed) in [
            (1, "hi", false),
            (2, "help", false),
            (3, "backpack", true),
            (4, "backpack", false),
            (5, "blueberry", true),
            (6, "lemon", false),
        ] {
            if command_id == 3 {
                let started:bool=sqlx::query_scalar("SELECT started FROM game_seasonal_cake_characters WHERE character_id=encode($1,'hex')::uuid")
                    .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
                assert!(
                    started,
                    "native help must reach the durable errand owner before completion"
                );
                let mut seed = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica")
                    .execute(&mut *seed)
                    .await?;
                sqlx::query("UPDATE game_seasonal_cake_characters SET visited_cities=31 WHERE character_id=encode($1,'hex')::uuid")
                    .bind(id(41).as_slice()).execute(&mut *seed).await?;
                seed.commit().await?;
            }

            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("NPC clock")?;
            let reply = owner
                .npc_talk(
                    actor,
                    UseCommand {
                        game_session_id: session,
                        command_id,
                        item_fence: Some(item_fence),
                    },
                    NpcTalkIntent {
                        npc_actor: npc,
                        text: text.into(),
                    },
                )
                .await;
            assert_eq!(reply.disposition, NpcIntentDisposition::Ok, "{text}");
            assert!(!reply.unknown, "{text}");
            assert_eq!(
                reply.inventory_changed, changed,
                "{text}: {:?}",
                reply.conversation
            );
            assert!(reply.conversation.is_some(), "{text}");
        }
        let started:bool=sqlx::query_scalar("SELECT started FROM game_seasonal_cake_characters WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert!(started, "native help must reach durable errand owner");
        let mint_count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_reward_claims WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(41).as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(mint_count, 2);
        let stale = EntityRef {
            generation: npc.generation + 1,
            ..npc
        };
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let reply = owner
            .npc_talk(
                actor,
                UseCommand {
                    game_session_id: session,
                    command_id: 7,
                    item_fence: Some(item_fence),
                },
                NpcTalkIntent {
                    npc_actor: stale,
                    text: "cabinet".into(),
                },
            )
            .await;
        assert_eq!(reply.disposition, NpcIntentDisposition::Stale);
        assert!(!reply.inventory_changed);
        eprintln!("SRV7_BRUTUS_NATIVE_POSTGRES_CASE_COMPLETED");
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
