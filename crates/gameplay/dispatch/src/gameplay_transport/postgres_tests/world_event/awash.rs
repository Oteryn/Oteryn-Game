use super::*;

#[test]
fn awash_coal_threshold_is_applied_at_the_next_server_save() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            run(async |admin| {
                reset();
                let h = Harness::create_with_admission_seed(admin, "wcphase_awash", true, false).await?;
                let world = WorldId::decode(&id(42)).map_err(debug)?;
                let channel = ChannelId::decode(&id(43)).map_err(debug)?;
                let (_controller, pin) =
                    crate::content::native_gameplay::test_support::activated_with_loot_tables(
                        world,
                        Some(include_bytes!(
                            "../../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
                        )),
                    );
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                h.root.t94_initialize(&authority, "oteryn:game:primary", &mut T94Bootstrap).await?;
                h.root.t94_start_world(&authority, world, &Default::default(), "oteryn:game:primary").await?;

                // Seed the completed daily pump tallies after the owning day exists.
                // The actual reset job must select the following day's state.
                sqlx::raw_sql(
                    "CREATE FUNCTION seed_awash_coal() RETURNS trigger LANGUAGE plpgsql AS $$
                     BEGIN
                       IF NEW.reset_epoch BETWEEN 1 AND 7 THEN
                         INSERT INTO game_world_change_tallies VALUES
                           (NEW.world_id, NEW.reset_epoch, 'coal', CASE WHEN NEW.reset_epoch=1 THEN 201 ELSE 202 END);
                       END IF;
                       RETURN NEW;
                     END $$;
                     CREATE TRIGGER seed_awash_coal AFTER INSERT ON game_world_change_days
                       FOR EACH ROW EXECUTE FUNCTION seed_awash_coal();
                     CREATE FUNCTION seed_awash_scout_credit() RETURNS trigger LANGUAGE plpgsql AS $$
                     BEGIN
                       IF NEW.change_key='awash' AND NEW.active AND NEW.reset_epoch IN (3,5,6) THEN
                         UPDATE game_world_change_states SET counter=CASE NEW.reset_epoch
                             WHEN 3 THEN 20 WHEN 5 THEN 30 ELSE 40 END
                         WHERE (world_id, reset_epoch, change_key)=
                             (NEW.world_id, NEW.reset_epoch, NEW.change_key);
                       END IF;
                       RETURN NEW;
                     END $$;
                     CREATE TRIGGER seed_awash_scout_credit AFTER INSERT ON game_world_change_states
                       FOR EACH ROW EXECUTE FUNCTION seed_awash_scout_credit();",
                )
                .execute(&h.pool)
                .await?;
                event_fixture::reset_epochs(&h, &authority, pin.qualified_room(), world, channel, 8).await?;
                let states: Vec<(bool, i64)> = sqlx::query_as(
                    "SELECT active,counter FROM game_world_change_states
                     WHERE world_id=encode($1,'hex')::uuid AND change_key='awash'
                     ORDER BY reset_epoch",
                )
                .bind(world.as_bytes().as_slice())
                .fetch_all(&h.pool)
                .await?;
                assert_eq!(states, [(false, 0), (false, 0), (true, 20), (true, 20), (true, 30), (true, 40), (true, 40), (false, 0)]);
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Awash server save thread panicked")??;
    Ok(())
}
