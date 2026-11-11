//! WU26-05: Winter Update 2026 retired the daily item picker. Its commands left capability
//! 182 and are refused as unregistered without touching the daily claim.
use super::*;
#[test]
fn daily_picker_commands_are_refused_after_rework() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_feature(false, true).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "picker refusal scenario panicked")?
        .map_err(Into::into)
}
async fn counts(h: &Harness) -> TestResult<(i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_daily_reward_claims),(SELECT count(*) FROM game_daily_reward_picks)")
        .fetch_one(&h.pool).await?)
}
pub(super) async fn case(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    shrine: &[u8],
) -> TestResult {
    use crate::foundation::CommandStatus;
    use wire::picker::{CLAIM, Claim, Pick, QUERY};
    let selection = Claim {
        epoch: 1,
        day: 1,
        picks: vec![Pick {
            item: 266,
            count: 5,
        }],
    }
    .encode()?;
    let (_, output) = serve(
        owner,
        admitted,
        &[(2, shrine), (QUERY, &[]), (CLAIM, &selection)],
    )
    .await?;
    let values = results(&output)?;
    assert_eq!(
        values[1..]
            .iter()
            .map(|(_, status, payload)| (*status, payload.len()))
            .collect::<Vec<_>>(),
        vec![(CommandStatus::Rejected, 0), (CommandStatus::Rejected, 0)],
        "the picker query and pick claim are refused with an empty payload"
    );
    assert_eq!(counts(h).await?, (0, 0), "a refused pick claim writes nothing");
    // The one-click claim still owns the day; a later pick claim stays refused.
    let (_, output) = serve(
        owner,
        admitted,
        &[
            (wire::COMMAND_TYPE_DAILY_REWARD_CLAIM, &[]),
            (CLAIM, &selection),
        ],
    )
    .await?;
    let values = results(&output)?;
    assert_eq!(values[0].2[1], 1, "one-click claim");
    assert_eq!((values[1].1, values[1].2.len()), (CommandStatus::Rejected, 0));
    assert_eq!(counts(h).await?, (1, 0), "no picked item is ever minted");
    eprintln!("POSTGRES_EXECUTED WU26-05 daily_picker_commands_are_refused_after_rework");
    Ok(())
}
