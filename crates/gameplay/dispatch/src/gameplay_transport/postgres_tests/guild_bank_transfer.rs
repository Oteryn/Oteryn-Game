use super::fixture::{Harness, debug, id};
use super::guild_bank_fixtures::{bank_seed, guild, query, transport::exchange};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::domain::CharacterId;
use crate::durability::guild_chat_fixture as seed;
use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_protocol_oteryn::guild_bank::{self as wire, GuildBankAction, GuildBankIntent};
use oteryn_protocol_oteryn::npc_service::NpcTalkIntent;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

#[test]
fn guild_bank_transfer_uses_confirmed_banker_dialogue_and_conserves_both_ledgers() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |_| {
        let h =
            Harness::create_with_admission_seed(admin, "guild_bank_transfer", true, false).await?;
        let result = exercise(&h).await;
        h.cleanup().await?;
        result
    })
}

async fn exercise(h: &Harness) -> TestResult {
    let banker = "oteryn:npc.naji";
    prepare_current_session(h).await?;
    let target_leader = CharacterId::from_bytes(seed::sid(90, 1)).map_err(debug)?;
    let target_account = seed::sid(90, 2);
    seed::historical_character(
        &h.pool,
        90,
        target_leader,
        target_account,
        id(42),
        "Target Leader",
    )
    .await?;
    let guest =
        seed::historical_session(&h.pool, 90, super::fixture::fence(1)?.item_fence()).await?;
    bank_seed::balance(h, id(40), id(41), 100, 160).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let deps = Dependencies::new()?;
    let coordinator =
        crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
            Default::default(),
            None,
            None,
            Some(h.root.clone()),
        );
    let mut owner = deps.owner(h, &authority);
    owner.premium_coordinator = Some(&coordinator);
    deps.initialize(&owner).await;
    let mut player = deps.admitted;
    player.first_entry = FirstEntryOutcome::Positioned;
    let (current, _) =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(player.game_session_id)
            .await?;
    player.item_fence = crate::gameplay_transport::item_fence_of(current);
    player.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[40, 55])
            .ok_or("guild bank capability")?;
    let (npc, target_actor) = {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let spot = runtime
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position();
        let placed = runtime
            .place_npcs(
                &NpcPlacementTable {
                    npcs: vec![NpcPlacements {
                        key: banker.into(),
                        placements: vec![NpcPlacement {
                            x: u16::try_from(spot.x + 1)?,
                            y: u16::try_from(spot.y)?,
                            floor: -i8::try_from(spot.floor)?,
                            direction: MovementFacing::South,
                        }],
                    }],
                },
                |_| true,
                true,
            )
            .map_err(debug)?;
        let npc = placed.actors[0];
        let reservation = runtime
            .reserve_fresh_session(guest.game_session_id)
            .map_err(debug)?;
        let target_actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_first_entry_position(target_actor)
            .map_err(debug)?;
        (
            EntityRef {
                identity: npc.placement_identity(),
                generation: npc.actor_local_generation(),
            },
            target_actor,
        )
    };
    deps.initialize_player(&owner, target_actor, guest.game_session_id)
        .await;
    let mut target_player = AdmittedSession {
        game_session_id: guest.game_session_id,
        runtime_actor: Some(target_actor),
        item_fence: Some(guest),
        controller: None,
        ..player
    };
    target_player.continuity.next_command_id = 1;
    let source = guild(
        &owner,
        &mut player,
        oteryn_protocol_oteryn::guild::GuildIntent::Found {
            name: "Source Guild".into(),
        },
    )
    .await?
    .guild;
    let target = guild(
        &owner,
        &mut target_player,
        oteryn_protocol_oteryn::guild::GuildIntent::Found {
            name: "Target Guild".into(),
        },
    )
    .await?
    .guild;
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
    let command_id = player.continuity.next_command_id;
    let (status, bytes) = exchange(
        &owner,
        &mut player,
        command_id,
        wire::COMMAND_TYPE_GUILD_BANK_INTENT,
        &wire::encode_guild_bank_intent(&GuildBankIntent {
            guild: source,
            action: GuildBankAction::Deposit(100),
        })
        .map_err(debug)?,
    )
    .await?;
    assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
    assert_eq!(
        wire::decode_guild_bank_reply(&bytes)
            .map_err(debug)?
            .guild_balance,
        100
    );
    owner
        .states
        .chat
        .admit(player.game_session_id, "Fixture Hero", owner.owner_now());
    owner.states.chat.set_npc_service(player.game_session_id);
    let talk = |command_id, text: &str| {
        owner.npc_talk(
            deps.actor,
            UseCommand {
                game_session_id: player.game_session_id,
                command_id,
                item_fence: player.item_fence,
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            },
        )
    };
    talk(10, "hi").await;
    let prompt = talk(11, "guild transfer 30 to Target Guild").await;
    assert!(
        prompt.conversation.is_some_and(|conversation| {
            conversation
                .lines
                .iter()
                .any(|line| line.contains("30") && line.contains("Target Guild"))
        }),
        "the banker should ask to confirm the named guild and amount"
    );
    let completed = talk(12, "yes").await;
    assert!(!completed.unknown);
    assert_eq!(
        completed.disposition,
        oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok
    );
    let before_replay = bank_seed::financial_snapshot(h).await?;
    let replay = crate::gameplay_transport::npc_command::resume_bank_talk(
        &owner,
        deps.actor,
        UseCommand {
            game_session_id: player.game_session_id,
            command_id: 12,
            item_fence: player.item_fence,
        },
    )
    .await
    .ok_or("retained NPC transfer outcome")?;
    assert_eq!(replay.disposition, completed.disposition);
    assert_eq!(bank_seed::financial_snapshot(h).await?, before_replay);
    let refused_prompt = talk(13, "guild transfer 71").await;
    assert!(
        refused_prompt.conversation.is_some_and(|conversation| {
            conversation.lines.iter().any(|line| line.contains("71"))
        })
    );
    talk(14, "target guild").await;
    let refusal = talk(15, "yes").await;
    assert!(refusal.conversation.is_some_and(|conversation| {
        conversation
            .lines
            .iter()
            .any(|line| line.contains("insufficient gold"))
    }));
    assert_eq!(bank_seed::financial_snapshot(h).await?, before_replay);
    assert_eq!(query(&owner, &mut player, source).await?.guild_balance, 70);
    assert_eq!(
        query(&owner, &mut target_player, target)
            .await?
            .guild_balance,
        30
    );
    let paired: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_guild_bank_entries e JOIN game_guild_bank_operations o USING(operation_id)
        WHERE o.kind='GUILD_TRANSFER' AND e.amount=30",
    ).fetch_one(&h.pool).await?;
    assert_eq!(
        paired, 2,
        "one debit and one credit must share the operation"
    );
    let receipts: Vec<(i16, String, String)> = sqlx::query_as(
        "SELECT o.result,i.definition_production_key,t.text
         FROM game_guild_bank_operations o
         JOIN game_character_inbox_deliveries d
           ON d.cause_kind='GUILD_BANK_RECEIPT' AND d.cause_ref=o.operation_id::text
          AND (d.character_id,d.world_id)=(o.actor,o.world_id)
         JOIN game_item_instances i USING(item_instance_id)
         JOIN game_item_texts t USING(item_instance_id)
         WHERE o.kind='GUILD_TRANSFER' ORDER BY o.command_id",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(receipts.len(), 2);
    assert_eq!(
        (receipts[0].0, receipts[0].1.as_str()),
        (1, "oteryn:item.tibia.i21932")
    );
    assert_eq!(
        (receipts[1].0, receipts[1].1.as_str()),
        (4, "oteryn:item.tibia.i21933")
    );
    assert!(
        receipts
            .iter()
            .all(|receipt| receipt.2.contains("Recipient: Target Guild"))
    );
    Ok(())
}
