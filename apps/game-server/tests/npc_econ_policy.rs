#![allow(clippy::expect_used)]
//! Exact adapter source exercised against the genuine native server dependency graph.
pub use oteryn_game_server::{domain, durability};

#[path = "../src/interaction/bank_npc.rs"]
mod bank_npc;

use self::domain::character_name::CharacterName;
use self::domain::{CharacterId, WorldId};
use self::durability::bank::BankResult;
use self::durability::bank::{
    BankCoinFacts, BankDepositRequest, BankOperationIdentity, BankOperationOccurrence,
    BankTransferRequest, BankWithdrawRequest,
};
use self::durability::item_mint::TypedDefinitionRef;
use self::durability::item_transfer::{ItemDefinitionFacts, ItemStackClass};
use bank_npc::*;
use oteryn_protocol_oteryn::GameSessionId;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

fn bytes(tag: u8) -> [u8; 16] {
    let mut value = [0; 16];
    value[6] = 0x70;
    value[8] = 0x80;
    value[15] = tag;
    value
}

fn context() -> BankDialogueContext {
    BankDialogueContext {
        session: GameSessionId::decode(&bytes(1)).expect("fixture UUIDv7"),
        character: CharacterId::from_bytes(bytes(2)).expect("fixture UUIDv7"),
        world: WorldId::from_bytes(bytes(3)).expect("fixture UUIDv7"),
        npc: EntityRef {
            identity: bytes(4),
            generation: 1,
        },
        conversation_revision: 1,
    }
}

#[test]
fn accepted_commands_and_each_native_cap_have_true_boundaries() {
    assert_eq!(parse("BaLaNcE"), Ok(BankNpcIntent::Balance));
    assert_eq!(parse("deposit all"), Ok(BankNpcIntent::DepositAll));
    for (verb, maximum) in [("deposit", 20_000_000_u64), ("withdraw", 1_009_999)] {
        for amount in [1, maximum] {
            assert!(parse(&format!("{verb} {amount} gold")).is_ok());
        }
        for amount in [0, maximum + 1, u64::MAX] {
            assert_eq!(
                parse(&format!("{verb} {amount} gold")),
                Err(BankNpcError::AmountOutOfRange)
            );
        }
    }
    assert!(parse("transfer 999999999999 gold to Al Dric").is_ok());
    assert_eq!(
        parse("transfer 1000000000000 gold to Al Dric"),
        Err(BankNpcError::AmountOutOfRange)
    );
}

#[test]
fn only_deposit_and_withdraw_resolve_backpack_coin_custody() {
    // Balance and transfer previews never read backpack facts, so an unresolvable backpack or
    // stale coin revision cannot refuse them (they pass no `BankCoinFacts`).
    for (text, custody) in [
        ("balance", false),
        ("transfer 5 gold to Al Dric", false),
        ("deposit 5 gold", true),
        ("deposit all", true),
        ("withdraw 5 gold", true),
    ] {
        assert_eq!(
            parse(text).expect("accepted command").uses_coin_custody(),
            custody,
            "{text}"
        );
    }
}

#[test]
fn text_is_bounded_and_neither_substrings_nor_invalid_names_are_accepted() {
    let at_max = format!("{}balance", " ".repeat(248));
    assert_eq!(at_max.len(), 255);
    assert!(parse(&at_max).is_ok());
    assert_eq!(
        parse(&(" ".to_owned() + &at_max)),
        Err(BankNpcError::InvalidInput)
    );
    for input in [
        "deposit +1 gold",
        "deposit -1 gold",
        "withdraw 1 gold extra",
        "rebalance",
        "balance\n",
        "exchange 1 gold",
        "promotion",
        "transfer 1 gold to Élan",
        "transfer 1 gold to A",
    ] {
        assert!(parse(input).is_err(), "{input:?}");
    }
    assert_eq!(
        parse("transfer 1 gold to Al  Dric"),
        Err(BankNpcError::UnknownRecipient)
    );
    assert_eq!(
        parse("transfer 1 gold to Al\u{200b}Dric"),
        Err(BankNpcError::UnknownRecipient)
    );
}

#[test]
fn confirmation_preserves_the_resolved_deposit_all_amount() {
    let current = context();
    let pending =
        BankConfirmation::new(BankNpcOperation::Deposit(123), current, 100).expect("prompt");
    assert_eq!(
        pending.respond(Some(&current), "YeS", 101),
        Some(BankNpcOperation::Deposit(123))
    );
}

#[test]
fn timeout_boundary_clock_reversal_and_every_non_yes_response_cancel() {
    let current = context();
    for (now, accepted) in [
        (99, false),
        (100, true),
        (30_099, true),
        (30_100, false),
        (30_101, false),
    ] {
        let pending =
            BankConfirmation::new(BankNpcOperation::Withdraw(1), current, 100).expect("prompt");
        assert_eq!(
            pending.respond(Some(&current), "yes", now).is_some(),
            accepted
        );
    }
    for text in ["no", "yes please", "yesterday", "yes\n", "", "ＹＥＳ"] {
        let pending =
            BankConfirmation::new(BankNpcOperation::Withdraw(1), current, 100).expect("prompt");
        assert_eq!(pending.respond(Some(&current), text, 101), None);
    }
    for (padding, accepted) in [(252, true), (253, false)] {
        let pending =
            BankConfirmation::new(BankNpcOperation::Withdraw(1), current, 100).expect("prompt");
        assert_eq!(
            pending
                .respond(Some(&current), &format!("{}yes", " ".repeat(padding)), 101)
                .is_some(),
            accepted
        );
    }
}

#[test]
fn missing_or_individually_changed_independent_context_cancels() {
    let original = context();
    let mut changed = [original; 6];
    changed[0].session = GameSessionId::decode(&bytes(9)).expect("fixture UUIDv7");
    changed[1].character = CharacterId::from_bytes(bytes(9)).expect("fixture UUIDv7");
    changed[2].world = WorldId::from_bytes(bytes(9)).expect("fixture UUIDv7");
    changed[3].npc.identity = bytes(9);
    changed[4].npc.generation += 1;
    changed[5].conversation_revision += 1;
    for current in std::iter::once(None).chain(changed.iter().map(Some)) {
        let pending =
            BankConfirmation::new(BankNpcOperation::Deposit(1), original, 100).expect("prompt");
        assert_eq!(pending.respond(current, "yes", 101), None);
    }
}

#[test]
fn invalid_amount_context_and_deadline_overflow_never_open_a_prompt() {
    let original = context();
    assert_eq!(
        BankConfirmation::new(BankNpcOperation::Deposit(0), original, 0).err(),
        Some(BankNpcError::AmountOutOfRange)
    );
    assert_eq!(
        BankConfirmation::new(BankNpcOperation::Withdraw(1_010_000), original, 0).err(),
        Some(BankNpcError::AmountOutOfRange)
    );
    let changed = BankDialogueContext {
        conversation_revision: 0,
        ..original
    };
    assert_eq!(
        BankConfirmation::new(BankNpcOperation::Deposit(1), changed, 0).err(),
        Some(BankNpcError::InvalidContext)
    );
    assert_eq!(
        BankConfirmation::new(BankNpcOperation::Deposit(1), original, u64::MAX).err(),
        Some(BankNpcError::TimeOverflow)
    );
}

#[test]
fn transfer_replies_preserve_the_accepted_anti_enumeration_contract() {
    let operation = BankNpcOperation::Transfer {
        amount: 17,
        recipient: context().character,
        name: CharacterName::parse("Al Dric").expect("valid name"),
    };
    for (result, reply) in [
        (
            BankResult::Ok,
            "Very well. You have transferred 17 gold to Al Dric.",
        ),
        (BankResult::UnknownRecipient, "This player does not exist."),
        (
            BankResult::RecipientCannotReceiveTransfers,
            "You cannot transfer money to this account.",
        ),
        (
            BankResult::BalanceLimit,
            "You cannot transfer money to this account.",
        ),
        (
            BankResult::SameAccount,
            "This character already shares your bank balance.",
        ),
        (
            BankResult::InsufficientBalance,
            "There is not enough gold on your account.",
        ),
        (
            BankResult::JuniorAccount,
            "You can use the bank once you have left the island.",
        ),
    ] {
        assert_eq!(operation.reply(result), reply);
    }
    for result in [
        BankResult::NoRoom,
        BankResult::InsufficientCoins,
        BankResult::BalanceLimit,
    ] {
        let reply = BankNpcOperation::Deposit(1).reply(result);
        assert!(!reply.is_empty());
        assert!(!reply.contains("Al Dric"));
    }
}

#[test]
fn native_requests_must_match_the_confirmed_kind_amount_and_recipient_binding() {
    let definition = |key: &str| TypedDefinitionRef {
        family: "item".into(),
        production_key: key.into(),
        revision_ref: "fixture-revision".into(),
    };
    let coins = BankCoinFacts {
        gold: definition("gold"),
        platinum: definition("platinum"),
        crystal: definition("crystal"),
        backpack: ItemDefinitionFacts {
            weight: None,
            definition: definition("backpack"),
            stack: ItemStackClass::NonStackable,
            container_capacity: Some(20),
            container_slot_equip_pattern: true,
        },
    };
    let identity = BankOperationIdentity {
        occurrence: BankOperationOccurrence::from_bytes(bytes(10))
            .expect("native UUIDv7 constructor"),
        transaction_id: bytes(11),
        event_id: bytes(12),
        occurred_at_unix_ms: 100,
        server_build_id: "fixture".into(),
    };
    let deposit = BankDepositRequest {
        identity: identity.clone(),
        amount: 17,
        coins: coins.clone(),
        change_item_instance_ids: [bytes(20), bytes(21)],
    };
    assert!(BankNpcOperation::Deposit(17).matches_deposit(&deposit));
    assert!(!BankNpcOperation::Withdraw(17).matches_deposit(&deposit));
    assert!(!BankNpcOperation::Deposit(18).matches_deposit(&deposit));
    let withdraw = BankWithdrawRequest {
        identity: identity.clone(),
        amount: 17,
        coins,
        output_item_instance_ids: [bytes(20), bytes(21), bytes(22)],
    };
    assert!(BankNpcOperation::Withdraw(17).matches_withdraw(&withdraw));
    assert!(!BankNpcOperation::Deposit(17).matches_withdraw(&withdraw));
    assert!(!BankNpcOperation::Withdraw(18).matches_withdraw(&withdraw));
    let operation = BankNpcOperation::Transfer {
        amount: 17,
        recipient: context().character,
        name: CharacterName::parse("Al Dric").expect("valid name"),
    };
    let transfer = BankTransferRequest {
        identity,
        amount: 17,
        recipient_character_id: context().character,
        recipient_name_key: "aldric".into(),
    };
    assert!(operation.matches_transfer(&transfer));
    assert!(!operation.matches_transfer(&BankTransferRequest {
        amount: 18,
        ..transfer.clone()
    }));
    assert!(!operation.matches_transfer(&BankTransferRequest {
        recipient_character_id: CharacterId::from_bytes(bytes(9)).expect("UUIDv7"),
        ..transfer.clone()
    }));
    assert!(!operation.matches_transfer(&BankTransferRequest {
        recipient_name_key: "someoneelse".into(),
        ..transfer
    }));
}

#[test]
fn prompts_bind_exact_resolved_amount_and_privacy_safe_recipient() {
    let transfer = BankNpcOperation::Transfer {
        amount: 123,
        recipient: context().character,
        name: CharacterName::parse("Al Dric").expect("native name"),
    };
    assert_eq!(
        transfer.prompt(),
        "Do you want to transfer 123 gold to Al Dric?"
    );
    assert_eq!(
        BankNpcOperation::Deposit(15).prompt(),
        "Do you want to deposit 15 gold?"
    );
    assert_eq!(
        BankNpcOperation::Withdraw(16).prompt(),
        "Do you want to withdraw 16 gold?"
    );
    assert_eq!(
        refusal_reply(BankResult::UnknownRecipient),
        "This player does not exist."
    );
    assert_eq!(
        refusal_reply(BankResult::SameAccount),
        "This character already shares your bank balance."
    );
    assert_eq!(
        refusal_reply(BankResult::JuniorAccount),
        "You can use the bank once you have left the island."
    );
    assert_eq!(
        refusal_reply(BankResult::RecipientCannotReceiveTransfers),
        "You cannot transfer money to this account."
    );
}
