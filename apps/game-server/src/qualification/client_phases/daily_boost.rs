//! Daily boost and reward over the real node; reference: `client_e2e/daily_boost.rs`.
use super::*;
pub(super) async fn reward_selection(
    client: &mut oteryn_dev_client::DevClientSession,
) -> TestResult<oteryn_protocol_oteryn::daily_boost::DailyBoostView> {
    use oteryn_dev_client::client_ui::daily_boost::{self, DailyBoostPanel};
    use oteryn_protocol_oteryn::daily_boost as wire;
    if !client
        .selected_capabilities()
        .contains(&wire::CAPABILITY_DAILY_BOOST_V1)
    {
        return Err("World reward qualification requires the daily boost capability".into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    let response = tokio::time::timeout_at(deadline, async {
        loop {
            let response = client
                .session_mut()
                .gated_command(
                    wire::CAPABILITY_DAILY_BOOST_V1,
                    wire::COMMAND_TYPE_DAILY_BOOST_QUERY,
                    &[],
                )
                .await?;
            if response.status != CommandStatus::Rejected
                || response.payload != oteryn_protocol_oteryn::COMMAND_BUSY_REFUSAL
            {
                break TestResult::Ok(response);
            }
            client.service_liveness(Duration::from_millis(50)).await?;
        }
    })
    .await
    .map_err(|_| "daily reward selection exchange deadline")??;
    let mut panel = DailyBoostPanel {
        selected: true,
        ..Default::default()
    };
    daily_boost::apply_result(&mut panel, &response);
    if response.status != CommandStatus::Accepted {
        return Err(format!("daily reward selection refused: {:?}", response.status).into());
    }
    panel
        .boost
        .ok_or_else(|| "daily reward selection did not decode".into())
}

pub(super) fn boosted_xp(
    base: u64,
    species: &str,
    selection: &oteryn_protocol_oteryn::daily_boost::DailyBoostView,
) -> TestResult<u64> {
    if selection.creature_key == species {
        base.checked_mul(2)
            .ok_or_else(|| "daily creature XP overflow".into())
    } else {
        Ok(base)
    }
}

#[test]
fn qualification_daily_boost_doubles_only_the_selected_creature_pool() -> TestResult {
    use oteryn_protocol_oteryn::daily_boost::DailyBoostView;
    let selection = DailyBoostView {
        reset_epoch: 1,
        creature_key: "oteryn:creature.bonelord".into(),
        boss_key: "oteryn:creature.abyssador".into(),
    };
    assert_eq!(boosted_xp(170, &selection.creature_key, &selection)?, 340);
    assert_eq!(boosted_xp(170, &selection.boss_key, &selection)?, 170);
    assert_eq!(boosted_xp(10, "oteryn:creature.cave_rat", &selection)?, 10);
    Ok(())
}

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
    client_ui::daily_boost::apply_result(&mut panel, &repeated);
    if panel.boost.as_ref() != Some(&boost) {
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
        let wall = match query.reply {
            Some(DailyRewardReply::Wall(wall)) => wall,
            _ => return Err("daily reward wall was not decoded".into()),
        };
        if wall.claim_enabled {
            let claim = client_runtime::daily_reward::send(
                client.session_mut(),
                &DailyRewardCommand::Claim,
            )
            .await?;
            if !matches!(claim.reply, Some(DailyRewardReply::Result(_))) {
                return Err("daily reward claim result was not decoded".into());
            }
        }
        evidence("mechanic=daily_boost reward=query_and_conditional_claim transport=tls");
    } else {
        evidence("mechanic=daily_boost reward=unavailable capability=182_not_selected");
    }
    Ok(())
}
