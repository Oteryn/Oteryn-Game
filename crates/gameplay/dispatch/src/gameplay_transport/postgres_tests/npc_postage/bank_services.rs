//! Caller-specific coverage of the existing fenced Bank owner.
use super::*;
pub(super) async fn run(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    npc: EntityRef,
    command: impl Fn(u64) -> UseCommand,
    greeting: &str,
) -> TestResult {
    let before = exact_shops::gold(h).await?;
    let mut replies = std::collections::BTreeMap::new();
    let rows = [
        (1, "hi", greeting),
        (2, "balance", "Your account balance is 0 gold."),
        (3, "deposit 10 gold", "Do you want to deposit 10 gold?"),
        (4, "yes", ""),
        (4, "yes", ""),
        (5, "balance", "Your account balance is 10 gold."),
        (6, "withdraw 5 gold", "Do you want to withdraw 5 gold?"),
        (7, "yes", ""),
        (7, "yes", ""),
        (8, "balance", "Your account balance is 5 gold."),
    ];
    for (ordinal, text, expected) in rows {
        owner.clock_origin -= std::time::Duration::from_secs(1);
        let outcome = owner.npc_talk(actor, command(ordinal), NpcTalkIntent {
            npc_actor: npc, text: text.into(),
        }).await;
        assert_eq!(outcome.disposition, NpcIntentDisposition::Ok, "Bank {text}");
        let lines = outcome.conversation.ok_or("native Bank conversation")?.lines;
        if let Some(original) = replies.insert(ordinal, lines.clone()) { assert_eq!(lines, original, "exact Bank replay"); }
        if !expected.is_empty() {
            assert!(lines.iter().any(|line| line == expected), "Bank {text}: {lines:?}");
        }
    }
    assert_eq!(exact_shops::gold(h).await?, before, "Bank moves conserve money");
    let operations: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_bank_operations WHERE acting_character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(operations, 2, "deposit/withdraw confirmation replay adds no receipt");
    let audits: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_bank_audit_outbox")
        .fetch_one(&h.pool).await?;
    assert_eq!(audits, 2, "replay appends no second Bank audit");
    Ok(())
}
