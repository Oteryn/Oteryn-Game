//! Daily boost and reward over the real node; reference: `client_e2e/daily_boost.rs`.
use super::*;
pub(super) async fn daily_boost(
    client: &mut oteryn_dev_client::DevClientSession,
) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime, client_ui};
    use oteryn_protocol_oteryn::daily_boost;
    if !client
        .selected_capabilities()
        .contains(&daily_boost::CAPABILITY_DAILY_BOOST_V1)
    {
        // daily_boost_available requires a bundle and eligible creature AND boss pools.
        evidence("mechanic=daily_boost skipped=entry_room_has_no_bundle_boost_pools");
        return Ok(());
    }
    let mut panel = client_ui::daily_boost::DailyBoostPanel {
        selected: true,
        ..Default::default()
    };
    let command = client_ui::daily_boost::request(
        &mut panel,
        client_ui::daily_boost::Intent {
            command: daily_boost::COMMAND_TYPE_DAILY_BOOST_QUERY,
            payload: vec![],
        },
    )
    .ok_or("daily boost request")?;
    let response = client_runtime::raw_feature::send(client.session_mut(), &command).await?;
    client_ui::daily_boost::apply_result(&mut panel, &response);
    if panel.boost.is_none() {
        return Err("daily boost view was not decoded".into());
    }
    let boost = panel.boost.clone().ok_or("daily boost not decoded")?;
    let repeated = client_runtime::raw_feature::send(client.session_mut(), &command).await?;
    if !repeated_boost_is_stable(&mut panel, &repeated) {
        return Err("daily choice changed within the same server-save epoch".into());
    }
    evidence(&format!(
        "mechanic=daily_boost state=stable epoch={} creature={} boss={}",
        boost.reset_epoch, boost.creature_key, boost.boss_key
    ));
    use client_runtime::daily_reward::{DailyRewardCommand, DailyRewardReply};
    if client
        .selected_capabilities()
        .contains(&client_runtime::daily_reward::daily_reward::CAPABILITY_DAILY_REWARD_V2)
    {
        let query =
            client_runtime::daily_reward::send(client.session_mut(), &DailyRewardCommand::Query)
                .await?;
        let wall = match (query.status, query.reply) {
            (CommandStatus::Accepted, Some(DailyRewardReply::Wall(wall))) => wall,
            _ => return Err("daily reward wall was not decoded".into()),
        };
        if wall.claim_enabled {
            let claim = client_runtime::daily_reward::send(
                client.session_mut(),
                &DailyRewardCommand::Claim,
            )
            .await?;
            if claim.status != CommandStatus::Accepted
                || !matches!(claim.reply, Some(DailyRewardReply::Result(result))
                    if result.disposition == 1 && result.transaction_id.is_some())
            {
                return Err(format!("enabled daily reward claim did not commit: {claim:?}").into());
            }
            let refreshed = client_runtime::daily_reward::send(
                client.session_mut(),
                &DailyRewardCommand::Query,
            )
            .await?;
            if refreshed.status != CommandStatus::Accepted
                || !matches!(refreshed.reply, Some(DailyRewardReply::Wall(wall))
                    if wall.claimed && !wall.claim_enabled)
            {
                return Err(
                    format!("daily reward wall did not reflect the claim: {refreshed:?}").into(),
                );
            }
        }
        evidence("mechanic=daily_boost reward=query_and_conditional_claim transport=tls");
    } else {
        evidence("mechanic=daily_boost reward=unavailable capability=182_not_selected");
    }
    Ok(())
}

fn repeated_boost_is_stable(
    panel: &mut oteryn_dev_client::client_ui::daily_boost::DailyBoostPanel,
    reply: &oteryn_dev_client::client_runtime::raw_feature::GatedCommandResult,
) -> bool {
    let before = panel.boost.clone();
    oteryn_dev_client::client_ui::daily_boost::apply_result(panel, reply);
    reply.status == CommandStatus::Accepted && panel.boost.is_some() && panel.boost == before
}

#[test]
fn refused_repeat_daily_boost_query_cannot_use_cached_view() {
    use oteryn_dev_client::client_runtime::{
        cui3::daily_boost::DailyBoostView, raw_feature::GatedCommandResult,
    };
    let mut panel = oteryn_dev_client::client_ui::daily_boost::DailyBoostPanel {
        boost: Some(DailyBoostView {
            reset_epoch: 1,
            creature_key: "rat".into(),
            boss_key: "boss".into(),
        }),
        ..Default::default()
    };
    let reply = GatedCommandResult {
        command_id: 2,
        status: CommandStatus::Rejected,
        server_sequence: 2,
        payload: Vec::new(),
    };
    assert!(!repeated_boost_is_stable(&mut panel, &reply));
}
