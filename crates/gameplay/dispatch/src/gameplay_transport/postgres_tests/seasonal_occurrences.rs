//! Seasonal decisions through real reset publication and finalization.
use super::*;
use crate::foundation::{ChannelId, WorldId};

#[test]
fn seasonal_occurrences_keep_identity_across_saves_and_close_tinder_and_spring() -> TestResult {
    scenario(
        "DATE '2026-04-15' + CASE NEW.reset_epoch WHEN 1 THEN 1 WHEN 2 THEN 2 WHEN 3 THEN 8 WHEN 4 THEN 16 ELSE 17 END",
        5,
        &[("spring_into_life", &[1, 2]), ("tinder_box", &[1, 2, 3, 4])],
    )
}

#[test]
fn seasonal_occurrences_open_last_creep_in_eligible_months_and_close_at_save() -> TestResult {
    scenario(
        "CASE NEW.reset_epoch WHEN 1 THEN DATE '2026-03-20' WHEN 2 THEN DATE '2026-04-20' WHEN 3 THEN DATE '2026-04-24' WHEN 4 THEN DATE '2026-04-25' ELSE DATE '2026-05-20' END",
        5,
        &[("last_creep_standing", &[2, 3, 5])],
    )
}

#[test]
fn seasonal_occurrences_kingsday_roll_is_frozen_for_one_save() -> TestResult {
    // For World id(42) the v1 draw selects save 21 and closes at save 22.
    scenario(
        "DATE '2026-06-01' + NEW.reset_epoch::integer",
        22,
        &[("kingsday", &[21])],
    )
}

fn scenario(
    date_expression: &'static str,
    epochs: u64,
    events: &'static [(&'static str, &'static [i32])],
) -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run("seasonal-occurrences", move || {
        run(async move |_| {
            reset();
            let h = Harness::create_with_admission_seed(admin, "seasonal_occurrences", true, false)
                .await?;
            let world = WorldId::decode(&id(42)).map_err(debug)?;
            let channel = ChannelId::decode(&id(43)).map_err(debug)?;
            let (_, pin) =
                crate::content::native_gameplay::test_support::activated_with_loot_tables(
                    world, None,
                );
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            sqlx::query(&format!(
                "CREATE FUNCTION seasonal_occurrences_test_clock() RETURNS trigger LANGUAGE plpgsql AS $$
                BEGIN
                    NEW.activated_at = (({date_expression}) + TIME '06:00') AT TIME ZONE 'Europe/Berlin';
                    NEW.reset_month = to_char(NEW.activated_at AT TIME ZONE 'Europe/Berlin', 'YYYY-MM');
                    RETURN NEW;
                END $$"
            ))
            .execute(&h.pool)
            .await?;
            sqlx::query("CREATE TRIGGER seasonal_occurrences_test_clock BEFORE INSERT ON game_world_reset_epochs FOR EACH ROW EXECUTE FUNCTION seasonal_occurrences_test_clock()")
                .execute(&h.pool)
                .await?;
            super::world_event_fixtures::event_fixture::reset_epochs(
                &h,
                &authority,
                pin.qualified_room(),
                world,
                channel,
                epochs,
            )
            .await?;

            for &(key, expected) in events {
                let rows: Vec<(i32, Vec<u8>)> = sqlx::query_as(
                    "SELECT reset_epoch::integer, occurrence_id FROM game_world_seasonal_occurrences WHERE event_key=$1 ORDER BY reset_epoch",
                ).bind(key).fetch_all(&h.pool).await?;
                assert_eq!(
                    rows.iter().map(|(epoch, _)| *epoch).collect::<Vec<_>>(),
                    expected
                );
                if key == "last_creep_standing" {
                    assert_eq!(rows[0].1, rows[1].1);
                    assert_ne!(rows[1].1, rows[2].1);
                }
                if key == "tinder_box" || key == "spring_into_life" {
                    assert!(rows.windows(2).all(|pair| pair[0].1 == pair[1].1));
                }
                assert_eq!(
                    h.root.world_change_active(world, key).await?,
                    expected.contains(&i32::try_from(epochs)?),
                );
            }
            drop(authority);
            h.cleanup().await
        })
    })
}
