//! Real command6116 read projection plus unchanged paid writer/replay and fencing proof.
use super::{
    TestResult,
    fixture::{Harness, debug, id},
    item_forge::support::{self, Case, seed_items, send_command},
};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, ConnectionEnd, IDLE_LIVENESS, serve_admitted},
};
use oteryn_gameplay_items::forge::{EXALTED_CORE_KEY, FORGE_SLIVER_KEY};
use oteryn_protocol_oteryn::{CommandStatus, forge_preview as wire, item_forge::Operation};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[test]
fn forge_preview_postgres_coherent_quote_readonly_paid_replay_and_stale_fence() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("forge-preview", || {
        support::scenario(Case::Preview)
    })
}
async fn assets(h: &Harness) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('items',(SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i),'tiers',(SELECT jsonb_agg(to_jsonb(t) ORDER BY item_instance_id) FROM game_item_tiers t),'custody',(SELECT jsonb_agg(to_jsonb(e) ORDER BY item_instance_id) FROM game_item_container_entries e),'nested',(SELECT jsonb_agg(to_jsonb(e) ORDER BY item_instance_id) FROM game_item_nested_entries e),'dust',(SELECT jsonb_agg(to_jsonb(d) ORDER BY character_id) FROM game_character_forge_dust d),'bank',(SELECT jsonb_agg(to_jsonb(b) ORDER BY account_id,world_id) FROM game_account_bank_balances b),'dust_entries',(SELECT count(*) FROM game_character_forge_dust_entries),'bank_entries',(SELECT count(*) FROM game_account_bank_entries),'operations',(SELECT count(*) FROM game_forge_operations),'reservations',(SELECT count(*) FROM game_forge_reservations),'cooldowns',(SELECT jsonb_agg(to_jsonb(c) ORDER BY character_id) FROM game_forge_cooldowns c),'tier_audit',(SELECT count(*) FROM game_item_tier_audit),'audit',(SELECT count(*) FROM game_item_audit_outbox),'fees',(SELECT count(*) FROM game_item_fee_burns))")
        .fetch_one(&h.pool).await?)
}
async fn read(
    client: &mut tokio::io::DuplexStream,
    generation: u64,
    command: &mut u64,
    nonce: u64,
    selection: wire::Selection,
) -> TestResult<wire::Reply> {
    let payload = wire::Request { nonce, selection }
        .encode()
        .ok_or("preview request")?;
    let (status, reply) = send_command(
        client,
        generation,
        *command,
        wire::COMMAND_TYPE_FORGE_PREVIEW,
        &payload,
    )
    .await?;
    *command = command.checked_add(1).ok_or("command overflow")?;
    assert_eq!(status, CommandStatus::Accepted);
    let reply = wire::Reply::decode(&reply).ok_or("preview reply")?;
    assert_eq!(reply.nonce, nonce);
    Ok(reply)
}
async fn paid(
    client: &mut tokio::io::DuplexStream,
    generation: u64,
    command: &mut u64,
    payload: &[u8],
) -> TestResult<(CommandStatus, serde_json::Value)> {
    let (status, bytes) = send_command(
        client,
        generation,
        *command,
        oteryn_protocol_oteryn::item_forge::COMMAND_TYPE_ITEM_FORGE,
        payload,
    )
    .await?;
    *command = command.checked_add(1).ok_or("command overflow")?;
    Ok((status, serde_json::from_slice(&bytes)?))
}
pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    const SWORD: &str = "oteryn:item.tibia.i3264";
    seed_items(
        h,
        &[
            (101, SWORD, 1, 0),
            (102, SWORD, 1, 0),
            (104, "oteryn:item.tibia.i3043", 3, 0),
            (105, EXALTED_CORE_KEY, 2, 0),
            (106, FORGE_SLIVER_KEY, 17, 0),
        ],
    )
    .await?;
    let current = *admitted;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, current, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let exercise = async move {
        let mut command = current.continuity.next_command_id;
        let before = assets(h).await?;
        let wire::Payload::Page(page) = read(
            &mut client,
            current.continuity.connection_generation,
            &mut command,
            1,
            wire::Selection::Browse { after: 0 },
        )
        .await?
        .payload
        else {
            return Err("candidate page".into());
        };
        assert_eq!(page.resources.carried_gold, 30_000);
        assert_eq!(page.resources.bank_gold, 0);
        assert!(page.resources.bank_spendable);
        assert_eq!(
            (
                page.resources.dust,
                page.resources.dust_limit,
                page.resources.cores,
                page.resources.slivers
            ),
            (1000, 1000, 2, 17)
        );
        let swords = page
            .items
            .iter()
            .filter(|item| !item.container)
            .collect::<Vec<_>>();
        assert_eq!(swords.len(), 2);
        assert!(swords.iter().all(|item| !item.name.is_empty()
            && item.appearance == 3264
            && item.classification == 1
            && item.tier == 0
            && item.maximum_tier == 1));
        let selection = wire::Selection::Pair {
            operation: Operation::Fusion,
            first: swords[0].handle,
            second: swords[1].handle,
            boost: true,
            protect: true,
        };
        let wire::Payload::Quote(quote) = read(
            &mut client,
            current.continuity.connection_generation,
            &mut command,
            2,
            selection,
        )
        .await?
        .payload
        else {
            return Err("pair quote".into());
        };
        assert_eq!(quote.selection, selection);
        assert_eq!(quote.resources, page.resources);
        assert_eq!(
            quote.cost,
            wire::Cost {
                gold: 25_000,
                dust: 100,
                cores: 2
            }
        );
        assert_eq!(
            (
                quote.target_tier,
                quote.success_percent,
                quote.protection_percent
            ),
            (1, 65, 50)
        );
        super::forge_resource_confirmation::read_quotes(
            &mut client,
            current.continuity.connection_generation,
            &mut command,
        )
        .await?;
        super::exaltation::read_effects(
            &mut client,
            current.continuity.connection_generation,
            &mut command,
            swords[0].handle,
        )
        .await?;
        assert_eq!(
            assets(h).await?,
            before,
            "candidate/quote reads cannot change custody, balances, receipts, cooldown/reservations, audit or fees"
        );
        // A quote is advisory: after the live resources change, the original paid writer refuses.
        let mut setup = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *setup)
            .await?;
        sqlx::query(
        "UPDATE game_item_instances SET quantity=1 WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(104).as_slice())
    .execute(&mut *setup)
    .await?;
        setup.commit().await?;
        let request = selection.canonical(id(147)).ok_or("paid request")?;
        let paid_bytes = request.encode().ok_or("paid bytes")?;
        let (status, result) = paid(
            &mut client,
            current.continuity.connection_generation,
            &mut command,
            &paid_bytes,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(result["applied"], false);
        assert_eq!(result["reason"], "INSUFFICIENT_GOLD");
        let after = assets(h).await?;
        assert_eq!(
            paid(
                &mut client,
                current.continuity.connection_generation,
                &mut command,
                &paid_bytes
            )
            .await?,
            (CommandStatus::Accepted, result)
        );
        assert_eq!(
            assets(h).await?,
            after,
            "paid occurrence replay remains exactly once after a quote"
        );
        client.shutdown().await?;
        let mut remaining = Vec::new();
        client.read_to_end(&mut remaining).await?;
        Ok::<_, Box<dyn std::error::Error>>(after)
    };
    let (end, result) = tokio::time::timeout(
        std::time::Duration::from_secs(90),
        support::together(serving, exercise),
    )
    .await?;
    let summary = support::server_end_summary(&end);
    *admitted = match end {
        ConnectionEnd::AdmittedThenDisconnected(next) => next,
        _ => return Err(format!("Preview connection ended unexpectedly: {summary}").into()),
    };
    assert_eq!(
        admitted.continuity.next_command_id,
        current
            .continuity
            .next_command_id
            .checked_add(9)
            .ok_or("expected command overflow")?,
        "one connection executes Browse, Quote, three resource reads, two effect reads, paid writer and exact occurrence replay"
    );
    let after = result.map_err(|error| {
        format!("Preview one-connection flow failed: {error}; server: {summary}")
    })?;
    super::forge_resource_confirmation::read_without_handles(owner, admitted).await?;
    // The exact durability reader rejects a stale connection fence against real PG rows.
    let mut stale = admitted.item_fence.ok_or("current item fence")?;
    stale.connection_generation =
        crate::foundation::ConnectionGeneration::new(stale.connection_generation.get() + 1)
            .map_err(debug)?;
    let command = crate::foundation::CommandRef::new(
        admitted.game_session_id,
        crate::foundation::CommandId::new(admitted.continuity.next_command_id).map_err(debug)?,
    );
    let result = owner
        .root
        .read_forge_preview(
            owner.character,
            owner.holder,
            stale,
            command,
            owner
                .active_generation
                .and_then(|active| active.native_gameplay())
                .ok_or("native content")?,
            None,
        )
        .await;
    assert!(matches!(
        result,
        Err(crate::durability::character_forge_dust::ForgeDustLimitError::AuthorityRejected)
    ));
    assert_eq!(
        assets(h).await?,
        after,
        "stale preview adds no Forge or money effects"
    );
    eprintln!(
        "FORGE_PREVIEW POSTGRES_CASE_COMPLETED coherent_readonly_paid_replay_and_stale_fence"
    );
    Ok(())
}
