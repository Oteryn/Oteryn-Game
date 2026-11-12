use crate::play::feature_0115_mail::protocol::{accepted, uuid};
use oteryn_client_runtime::raw_feature::{FeatureCommand, GatedCommandResult};
use oteryn_client_ui::guild_bank::{Intent, Panel};
use oteryn_session::property_services::guild_bank as wire;

pub(in crate::play) fn command(panel: &Panel, intent: Intent) -> Result<FeatureCommand, String> {
    let guild = || uuid(&panel.fields[0]);
    let (kind, payload) = match intent {
        Intent::Refresh | Intent::Older | Intent::Claims | Intent::MoreClaims => {
            let claims = matches!(intent, Intent::Claims | Intent::MoreClaims);
            if intent == Intent::Older && panel.loaded_guild != panel.fields[0] {
                return Err(tr!("Refresh this guild before paging").into());
            }
            let query = wire::GuildBankQuery {
                guild: if claims { None } else { Some(guild()?) },
                before: if intent == Intent::Older {
                    Some(panel.next.ok_or(tr!("No older ledger entries"))?)
                } else {
                    None
                },
                after_claim: if intent == Intent::MoreClaims {
                    Some(panel.next_claim.ok_or(tr!("No more open claims"))?)
                } else {
                    None
                },
            };
            (
                wire::COMMAND_TYPE_GUILD_BANK_QUERY,
                wire::encode_guild_bank_query(&query),
            )
        }
        Intent::Deposit | Intent::Withdraw | Intent::Claim => {
            let action = if intent == Intent::Claim {
                wire::GuildBankAction::Claim
            } else {
                let amount = panel.fields[1]
                    .trim()
                    .parse()
                    .map_err(|_| tr!("Enter a positive amount"))?;
                if intent == Intent::Deposit {
                    wire::GuildBankAction::Deposit(amount)
                } else {
                    wire::GuildBankAction::Withdraw(amount)
                }
            };
            (
                wire::COMMAND_TYPE_GUILD_BANK_INTENT,
                wire::encode_guild_bank_intent(&wire::GuildBankIntent {
                    guild: guild()?,
                    action,
                }),
            )
        }
    };
    Ok(FeatureCommand {
        capability: 55,
        kind,
        payload: payload.map_err(|_| tr!("Invalid guild UUID or amount"))?,
    })
}
pub(in crate::play) fn apply(
    panel: &mut Panel,
    command: &FeatureCommand,
    result: &GatedCommandResult,
) {
    if !panel.pending || panel.pending_kind != command.kind {
        return;
    }
    panel.pending = false;
    // Never retain another guild's balances or cursors after a refusal.
    panel.summary.clear();
    panel.next = None;
    panel.next_claim = None;
    if !accepted(result) {
        panel.feedback = tr!("The server refused the guild bank request").into();
        return;
    }
    let Ok(reply) = wire::decode_guild_bank_reply(&result.payload) else {
        panel.feedback = tr!("Invalid guild bank reply").into();
        return;
    };
    if reply.code != 1 {
        panel.feedback = match reply.code {
            2 => tr!("Guild bank access denied"),
            3 => tr!("Guild is disbanding"),
            4 => tr!("Insufficient balance"),
            5 => tr!("Balance limit reached"),
            6 => tr!("Junior characters cannot transact"),
            7 => tr!("No open claim"),
            _ => tr!("Guild bank request refused"),
        }
        .into();
        return;
    }
    panel.next = reply.next;
    panel.next_claim = reply.next_claim;
    panel.loaded_guild = panel.fields[0].clone();
    panel.summary = lfmt!(
        "Personal bank: {}\r\nGuild bank: {}\r\nClaim: {}",
        reply.bank_balance,
        reply.guild_balance,
        reply.claim
    );
    for entry in reply.entries {
        panel.summary.push_str(&format!(
            "\r\n#{}: {} {} ({} → {})",
            entry.ordinal,
            match entry.kind {
                1 => tr!("Deposit"),
                2 => tr!("Withdrawal"),
                3 => tr!("Disband payout"),
                4 => tr!("Hall reserve"),
                5 => tr!("Hall refund"),
                _ => tr!("Hall payment"),
            },
            entry.amount,
            entry.before,
            entry.after
        ));
    }
    for claim in reply.claims {
        let id: String = claim
            .guild
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        panel
            .summary
            .push_str(&lfmt!("\r\nOpen claim: {id} — {}", claim.amount, id = id));
    }
    panel.feedback = tr!("Guild bank confirmed by the server").into();
}
