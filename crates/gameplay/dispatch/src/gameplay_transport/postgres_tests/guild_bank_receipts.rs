use super::fixture::{Harness, debug, id};
use super::guild_bank_fixtures::{bank_seed, guild, transport::exchange};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::guild_bank::{self as wire, GuildBankAction, GuildBankIntent};
use oteryn_protocol_oteryn::{CommandStatus, guild as guild_wire};

#[test]
fn guild_bank_receipts_deliver_success_and_failure_once_to_the_acting_inbox() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |_| {
        let h =
            Harness::create_with_admission_seed(admin, "guild_bank_receipts", true, false).await?;
        let result = exercise(&h).await;
        h.cleanup().await?;
        result
    })
}

async fn exercise(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
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
    owner
        .runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let (current, _) =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(deps.admitted.game_session_id)
            .await?;
    let mut player = deps.admitted;
    player.first_entry = FirstEntryOutcome::Positioned;
    player.item_fence = crate::gameplay_transport::item_fence_of(current);
    player.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[40, 55])
            .ok_or("guild bank capability")?;
    let guild = guild(
        &owner,
        &mut player,
        guild_wire::GuildIntent::Found {
            name: "Receipt Guild".into(),
        },
    )
    .await?
    .guild;
    for (action, code) in [
        (GuildBankAction::Deposit(35), 1),
        (GuildBankAction::Withdraw(36), 4),
    ] {
        let id = player.continuity.next_command_id;
        let payload =
            wire::encode_guild_bank_intent(&GuildBankIntent { guild, action }).map_err(debug)?;
        let (status, bytes) = exchange(
            &owner,
            &mut player,
            id,
            wire::COMMAND_TYPE_GUILD_BANK_INTENT,
            &payload,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            wire::decode_guild_bank_reply(&bytes).map_err(debug)?.code,
            code
        );
        let (status, replay) = exchange(
            &owner,
            &mut player,
            id,
            wire::COMMAND_TYPE_GUILD_BANK_INTENT,
            &payload,
        )
        .await?;
        assert_eq!(status, CommandStatus::DuplicateReplay);
        assert_eq!(replay, bytes);
    }
    let receipts: Vec<(String, String)> = sqlx::query_as(
        "SELECT i.definition_production_key,t.text
         FROM game_item_character_inbox_locations l
         JOIN game_item_instances i USING(item_instance_id,world_id)
         JOIN game_item_texts t USING(item_instance_id)
         WHERE l.character_id=encode($1,'hex')::uuid AND i.definition_production_key IN
            ('oteryn:item.tibia.i21932','oteryn:item.tibia.i21933')
         ORDER BY l.ordinal",
    )
    .bind(id(41).as_slice())
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(
        receipts.len(),
        2,
        "the acting Character must receive one paper per attempted transaction"
    );
    assert_eq!(receipts[0].0, "oteryn:item.tibia.i21932");
    assert!(receipts[0].1.contains("35") && receipts[0].1.contains("Receipt Guild"));
    assert_eq!(receipts[1].0, "oteryn:item.tibia.i21933");
    assert!(receipts[1].1.contains("36") && receipts[1].1.contains("Insufficient"));
    let snapshot = bank_seed::financial_snapshot(h).await?;
    sqlx::raw_sql(
        "CREATE FUNCTION reject_receipt_test() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN RAISE EXCEPTION 'injected receipt text failure'; END $$;
         CREATE TRIGGER reject_receipt_test BEFORE INSERT ON game_item_texts
          FOR EACH ROW EXECUTE FUNCTION reject_receipt_test();",
    )
    .execute(&h.pool)
    .await?;
    let id = player.continuity.next_command_id;
    let payload = wire::encode_guild_bank_intent(&GuildBankIntent {
        guild,
        action: GuildBankAction::Withdraw(10),
    })
    .map_err(debug)?;
    assert!(matches!(
        owner
            .guild_command(player, id, wire::COMMAND_TYPE_GUILD_BANK_INTENT, &payload)
            .await,
        crate::gameplay_transport::guild_intent::GuildReply::Unavailable
    ));
    assert_eq!(
        bank_seed::financial_snapshot(h).await?,
        snapshot,
        "receipt failure rolls back gold"
    );
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_inbox_deliveries WHERE cause_kind='GUILD_BANK_RECEIPT'",
    ).fetch_one(&h.pool).await?;
    assert_eq!(count, 2, "receipt failure rolls back all Inbox custody");
    Ok(())
}
