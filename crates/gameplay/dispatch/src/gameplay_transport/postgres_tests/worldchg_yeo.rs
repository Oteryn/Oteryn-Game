//! Real native Yeo hi/gloo/yes -> existing fenced NPC Trade item barter and audit owner.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::durability::npc_trade::yeo::{GLOO, NPC, POSITION, SHRIMP};
use crate::durability::npc_trade::{NpcTradeOutcome, NpcTradeRequest};
use crate::foundation::{
    ChannelContentPin, CharacterLease, CommandId, CommandRef, ConnectionGeneration, GameSessionId,
    MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use crate::gameplay_transport::npc_trade_command::{derived, occurrence};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;
#[test]
fn worldchg_yeo_native_repeatable_ten_shrimp_one_gloo_atomic_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("worldchg-yeo", scenario)
}
async fn snapshot(h: &Harness) -> TestResult<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1),(SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$2),(SELECT count(*) FROM game_npc_trades),(SELECT count(*) FROM game_item_audit_outbox)")
        .bind(SHRIMP).bind(GLOO).fetch_one(&h.pool).await?)
}
fn rekey(
    request: &mut NpcTradeRequest,
    session: GameSessionId,
    npc: EntityRef,
    command: u64,
) -> TestResult {
    let id_for = |tag: &[u8]| derived(tag, session, command);
    request.command = CommandRef::new(session, CommandId::new(command).map_err(debug)?);
    request.cause.occurrence = occurrence(session, command, npc);
    request.transaction_id = id_for(b"oteryn:yeo-barter:transaction:v1");
    request.event_id = id_for(b"oteryn:yeo-barter:event:v1");
    request.output_item_instance_ids = [
        id_for(b"oteryn:yeo-barter:output1:v1"),
        id_for(b"oteryn:yeo-barter:output2:v1"),
        id_for(b"oteryn:yeo-barter:output3:v1"),
    ];
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        let started = std::time::Instant::now();
        reset();
        let h = Harness::create_with_admission_seed(admin, "worldchg_yeo", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("native generation")?;
        let (x, y, z) = POSITION;
        // Explicit native geometry, independently audited B3 walkability. No donor coordinate claim.
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (i32::from(x) - 1, i32::from(y), i16::from(z)),
        );
        let deps = Dependencies::with_content_and_capacity(
            channel_pin,
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
            3,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        deps.initialize(&owner).await;
        let model = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?
        .service_model_for_project()
        .map_err(debug)?;
        let book = NpcTalkBook::from_model(&model);
        let entry = book.get(NPC).ok_or("actual native Yeo greeting")?;
        assert!(
            entry.offers().is_empty(),
            "barter must not create a free generic shop offer"
        );
        owner.states.chat.install_npc_talk(Arc::new(book));
        let session = deps.admitted.game_session_id;
        owner
            .states
            .chat
            .admit(session, "Barter customer", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "current fence")?
            .ok_or("current fence")?
            .item_fence();
        let npc = {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    session,
                    CharacterLease::new(
                        crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                            .map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: NPC.into(),
                            placements: vec![NpcPlacement {
                                x,
                                y,
                                floor: -z,
                                direction: MovementFacing::South,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            runtime
                .visible_entities()
                .npcs
                .iter()
                .find(|n| runtime.npc_key(n.actor) == Some(NPC))
                .map(|n| EntityRef {
                    identity: n.actor.placement_identity(),
                    generation: n.generation,
                })
                .ok_or("actual Yeo actor")?
        };
        super::item_look::seed_main_backpack(&h, &[(SHRIMP, 25)]).await?;
        let command = |n| UseCommand {
            game_session_id: session,
            command_id: n,
            item_fence: Some(fence),
        };
        for (n, text) in [(1, "hi"), (2, "gloo")] {
            let result = owner
                .npc_talk(
                    deps.actor,
                    command(n),
                    NpcTalkIntent {
                        npc_actor: npc,
                        text: text.into(),
                    },
                )
                .await;
            assert_eq!(
                result.disposition,
                NpcIntentDisposition::Ok,
                "native {text}"
            );
        }
        assert_eq!(snapshot(&h).await?, (25, 0, 0, 0));
        let result = owner
            .npc_talk(
                deps.actor,
                command(3),
                NpcTalkIntent {
                    npc_actor: npc,
                    text: "yes".into(),
                },
            )
            .await;
        assert_eq!(result.disposition, NpcIntentDisposition::Ok);
        assert!(
            result.inventory_changed,
            "native yes must commit real barter"
        );
        assert_eq!(snapshot(&h).await?, (15, 1, 1, 1));
        let first = owner
            .states
            .npc_conversations
            .yeo_request_for_test(session)
            .ok_or("retained original source request")?;
        let line_units:(i64,i64)=sqlx::query_as("SELECT coalesce(sum(quantity_before-quantity_after) FILTER(WHERE line_kind=3),0)::bigint,coalesce(sum(quantity_after) FILTER(WHERE line_kind=4),0)::bigint FROM game_npc_trade_lines").fetch_one(&h.pool).await?;
        assert_eq!(line_units, (10, 1));
        assert!(matches!(
            h.root
                .commit_yeo_barter(&authority, &h.node, fence, first.clone(), POSITION)
                .await
                .map_err(debug)?,
            NpcTradeOutcome::AlreadyCommitted(_)
        ));
        assert_eq!(snapshot(&h).await?, (15, 1, 1, 1));
        let mut generic = first.clone();
        rekey(&mut generic, session, npc, 40)?;
        assert!(
            h.root
                .commit_npc_trade(&authority, &h.node, fence, generic)
                .await
                .is_err(),
            "generic zero-price Yeo cannot bypass barter"
        );
        let mut wrong = first.clone();
        rekey(&mut wrong, session, npc, 41)?;
        assert!(
            h.root
                .commit_yeo_barter(&authority, &h.node, fence, wrong, (x + 1, y, z))
                .await
                .is_err()
        );
        let mut stale_request = first;
        rekey(&mut stale_request, session, npc, 42)?;
        let stale = crate::durability::item_transfer::CurrentCharacterItemFence {
            connection_generation: ConnectionGeneration::new(fence.connection_generation.get() + 1)
                .map_err(debug)?,
            ..fence
        };
        assert!(
            h.root
                .commit_yeo_barter(&authority, &h.node, stale, stale_request, POSITION)
                .await
                .is_err()
        );
        assert_eq!(snapshot(&h).await?, (15, 1, 1, 1));
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        for (n, text) in [(4, "gloo"), (5, "yes")] {
            let result = owner
                .npc_talk(
                    deps.actor,
                    command(n),
                    NpcTalkIntent {
                        npc_actor: npc,
                        text: text.into(),
                    },
                )
                .await;
            assert_eq!(result.disposition, NpcIntentDisposition::Ok);
            if text == "yes" {
                assert!(
                    result.inventory_changed,
                    "barter is repeatable, not a one-time reward claim"
                );
            }
        }
        assert_eq!(snapshot(&h).await?, (5, 2, 2, 2));
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        for (n, text) in [(6, "gloo"), (7, "yes")] {
            let result = owner
                .npc_talk(
                    deps.actor,
                    command(n),
                    NpcTalkIntent {
                        npc_actor: npc,
                        text: text.into(),
                    },
                )
                .await;
            assert_eq!(result.disposition, NpcIntentDisposition::Ok);
            assert!(!result.inventory_changed);
        }
        assert_eq!(
            snapshot(&h).await?,
            (5, 2, 2, 2),
            "insufficient shrimp must not debit, mint or retain a trade receipt"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        assert!(started.elapsed().as_secs_f64() > 1.0);
        println!(
            "WORLDCHG-Q PG EXECUTED: native Yeo hi/gloo/yes, repeatable10Shrimp1Gloo, atomic ledger/audit, receipt replay, generic free-offer refusal, wrong source, stale Character, insufficient input rollback; elapsed={:?}",
            started.elapsed()
        );
        Ok(())
    })
}
