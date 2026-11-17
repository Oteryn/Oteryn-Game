//! Chyllfroest activation follows the committed server-save date.
use super::*;
use crate::foundation::{ChannelId, WorldId};

#[path = "world_event/reset_fixture.rs"]
mod reset_fixture;

#[test]
fn cbchance_chyllfroest_opens_and_closes_at_server_save() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            run(async move |_| {
                let h = Harness::create_with_admission_seed(
                    admin,
                    "cbchance_chyllfroest",
                    true,
                    false,
                )
                .await?;
                sqlx::query(
                    "CREATE FUNCTION chyllfroest_save_clock() RETURNS trigger LANGUAGE plpgsql AS $$
                    BEGIN
                      NEW.activated_at = CASE NEW.reset_epoch
                          WHEN 1 THEN TIMESTAMPTZ '2026-03-31 06:00 Europe/Berlin'
                          WHEN 2 THEN TIMESTAMPTZ '2026-04-01 06:00 Europe/Berlin'
                          WHEN 3 THEN TIMESTAMPTZ '2026-05-01 06:00 Europe/Berlin'
                          ELSE TIMESTAMPTZ '2026-05-02 06:00 Europe/Berlin'
                      END;
                      NEW.reset_month = to_char(NEW.activated_at AT TIME ZONE 'Europe/Berlin', 'YYYY-MM');
                      RETURN NEW;
                    END $$",
                )
                .execute(&h.pool)
                .await?;
                sqlx::query(
                    "CREATE TRIGGER chyllfroest_save_clock BEFORE INSERT ON game_world_reset_epochs
                    FOR EACH ROW EXECUTE FUNCTION chyllfroest_save_clock()",
                )
                .execute(&h.pool)
                .await?;
                let world = WorldId::decode(&id(42)).map_err(debug)?;
                let channel = ChannelId::decode(&id(43)).map_err(debug)?;
                let (_controller, pin) =
                    crate::content::native_gameplay::test_support::activated_with_loot_tables(
                        world, None,
                    );
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                reset_fixture::reset_epochs(&h, &authority, pin.qualified_room(), world, channel, 4)
                    .await?;
                let days: Vec<(String, bool)> = sqlx::query_as(
                    "SELECT reset_epoch::text,active FROM game_world_change_states
                     WHERE world_id=encode($1,'hex')::uuid AND change_key='chyllfroest'
                     ORDER BY reset_epoch",
                )
                .bind(world.as_bytes().as_slice())
                .fetch_all(&h.pool)
                .await?;
                assert_eq!(
                    days,
                    vec![
                        ("1".into(), false),
                        ("2".into(), true),
                        ("3".into(), true),
                        ("4".into(), false),
                    ]
                );
                assert!(!h.root.world_change_active(world, "chyllfroest").await?);
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Chyllfroest save test thread panicked")??;
    Ok(())
}
