//! P12a: command 113 observes the existing Charm and Bosstiary owners, then persists titles
//! under the current session fence. Fixture seeds retain complete Bestiary receipt chains.
use super::character_title_sources_fixtures::{seed_completed, source_limits};

use super::cyclopedia_fixtures::support::exchange;
use super::fixture::{Harness, id, run};
use super::{Dependencies, TestResult, debug, prepare_current_session, reset};
use crate::domain::bestiary::BestiaryRace;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::character_info::CharacterTitle;

#[test]
fn character_title_native_sources_real_dispatch_and_reload() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "title native sources scenario panicked")??;
    eprintln!("character_title_native_sources_real_dispatch_and_reload POSTGRES_CASE_COMPLETED");
    Ok(())
}

fn catalogue() -> TestResult<Vec<(BestiaryRace, u32)>> {
    let mut rows = crate::content::canonical_bestiary_rows(source_limits()).map_err(debug)?;
    // Minimize fixture receipts using the admitted rewards; no product threshold is invented.
    rows.retain(|(_, points)| *points > 0);
    rows.sort_by_key(|(race, points)| {
        (race.final_kill_threshold() / points, race.key().to_owned())
    });
    Ok(rows)
}

fn completion_prefix(rows: &[(BestiaryRace, u32)], required: u32) -> TestResult<usize> {
    let mut points = 0;
    rows.iter()
        .position(|(_, reward)| {
            points += reward;
            points >= required
        })
        .map(|index| index + 1)
        .ok_or_else(|| "insufficient canonical Charm rewards".into())
}

async fn title_count(h: &Harness, title: CharacterTitle) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_character_titles WHERE character_id=encode($1,'hex')::uuid AND title=$2")
        .bind(id(41).as_slice()).bind(i16::from(title.id())).fetch_one(&h.pool).await?)
}

async fn title_command(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let command = admitted.continuity.next_command_id;
    exchange(owner, admitted, command, 113, payload).await
}

async fn charm_command(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    action: oteryn_protocol_oteryn::charm_progression::CharmProgressionAction,
) -> TestResult<oteryn_protocol_oteryn::charm_progression::CharmProgressionReply> {
    use oteryn_protocol_oteryn::charm_progression as wire;
    let payload = wire::encode_charm_progression_request(&action).map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let (status, bytes) = exchange(owner, admitted, command, 84, &payload).await?;
    assert_eq!(status, CommandStatus::Accepted);
    wire::decode_charm_progression_reply(&bytes).map_err(debug)
}

fn scenario() -> TestResult {
    use crate::gameplay_transport::capabilities::{
        PRODUCTION_OFFERED_CAPABILITIES, SelectedCapabilities,
    };
    use crate::gameplay_transport::connection::FirstEntryOutcome;
    use oteryn_protocol_oteryn::charm_progression::CharmProgressionAction;
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "character_title_sources", true, false)
            .await?;
        prepare_current_session(&h).await?;
        let rows = catalogue()?;
        let split = completion_prefix(&rows, 2_000)?;
        let initial_points: u32 = rows[..split].iter().map(|(_, points)| points).sum();
        assert!((2_000..2_240).contains(&initial_points));
        seed_completed(&h, &rows[..split]).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (generation, pin) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(world, "content-1")?;
        let room = pin.qualified_room().clone();
        let (channel_pin, _, _, _) = pin.into_channel_parts();
        let deps = Dependencies::with_content(
            channel_pin,
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = generation.active();
        deps.initialize(&owner).await;
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let current = crate::gameplay_transport::character_gameplay_fence::current(
            &owner,
            deps.admitted.game_session_id,
            false,
        )
        .await
        .ok_or("current fence")?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(current.item_fence());
        assert_eq!(
            title_command(&owner, &mut admitted, &[2, CharacterTitle::Beastly.id()])
                .await?
                .0,
            CommandStatus::Rejected
        );
        assert_eq!(
            title_count(&h, CharacterTitle::Beastly).await?,
            0,
            "unnegotiated request writes nothing"
        );
        let offers = crate::gameplay_transport::charm_progression::offered(
            PRODUCTION_OFFERED_CAPABILITIES,
            true,
        );
        let offers = crate::gameplay_transport::character_info::offered(&offers, true);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &[1, 67, 97]).ok_or("capabilities")?;
        let query = charm_command(&owner, &mut admitted, CharmProgressionAction::Query).await?;
        assert_eq!(query.charms.charm_points_available, initial_points);
        let charm = query
            .charms
            .charms
            .iter()
            .find(|entry| entry.next_stage_cost == 240)
            .ok_or("canonical 240-point charm")?
            .charm;
        let spent = charm_command(
            &owner,
            &mut admitted,
            CharmProgressionAction::Unlock {
                charm,
                expected_stage: 0,
            },
        )
        .await?;
        assert_eq!(spent.charms.charm_points_available, initial_points - 240);
        assert_eq!(
            title_command(&owner, &mut admitted, &[2, CharacterTitle::Beastly.id()])
                .await?
                .0,
            CommandStatus::Rejected,
            "lifetime-earned points do not satisfy the donor's available balance predicate"
        );
        assert_eq!(title_count(&h, CharacterTitle::Beastly).await?, 0);

        // A different recorded boss never qualifies the Ahau-specific rule.
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("session")?;
        let boss = |key: &str, category: &str| crate::domain::bosstiary::BosstiaryBoss {
            key: key.to_owned(),
            category: category.to_owned(),
        };
        h.root
            .credit_bosstiary_kill(
                &authority,
                &h.node,
                fence,
                crate::durability::bestiary_progress::BestiaryKillOccurrence::from_bytes(id(101))
                    .map_err(debug)?,
                &boss("oteryn:creature.gaz_haragoth", "nemesis"),
            )
            .await
            .map_err(debug)?;
        assert_eq!(
            title_command(
                &owner,
                &mut admitted,
                &[2, CharacterTitle::ChallengerOfTheIks.id()]
            )
            .await?
            .0,
            CommandStatus::Rejected
        );
        let occurrence =
            crate::durability::bestiary_progress::BestiaryKillOccurrence::from_bytes(id(102))
                .map_err(debug)?;
        let ahau = boss("oteryn:creature.ahau", "archfoe");
        assert!(
            h.root
                .credit_bosstiary_kill(&authority, &h.node, fence, occurrence, &ahau)
                .await
                .map_err(debug)?
        );
        assert!(
            !h.root
                .credit_bosstiary_kill(&authority, &h.node, fence, occurrence, &ahau)
                .await
                .map_err(debug)?,
            "boss occurrence replay"
        );
        let mut stale = admitted.item_fence.ok_or("item fence")?;
        stale.connection_generation =
            crate::foundation::ConnectionGeneration::new(999).map_err(debug)?;
        assert!(
            h.root
                .qualify_and_unlock_character_title(
                    &authority,
                    &h.node,
                    stale,
                    CharacterTitle::ChallengerOfTheIks,
                    1,
                    None,
                    None
                )
                .await
                .is_err()
        );
        assert_eq!(
            title_count(&h, CharacterTitle::ChallengerOfTheIks).await?,
            0,
            "stale generation writes nothing"
        );
        for _ in 0..2 {
            assert_eq!(
                title_command(
                    &owner,
                    &mut admitted,
                    &[2, CharacterTitle::ChallengerOfTheIks.id()]
                )
                .await?,
                (
                    CommandStatus::Accepted,
                    vec![2, CharacterTitle::ChallengerOfTheIks.id()]
                )
            );
        }
        assert_eq!(
            title_count(&h, CharacterTitle::ChallengerOfTheIks).await?,
            1
        );

        // More canonical completions now qualify Beastly. Self-query exercises automatic grant.
        let extra = completion_prefix(&rows[split..], 240)?;
        seed_completed(&h, &rows[split..split + extra]).await?;
        assert_eq!(
            title_command(&owner, &mut admitted, &[0, 0, 0, 1]).await?.0,
            CommandStatus::Accepted
        );
        assert_eq!(title_count(&h, CharacterTitle::Beastly).await?, 1);
        assert_eq!(
            title_command(&owner, &mut admitted, &[2, CharacterTitle::Beastly.id()])
                .await?
                .1,
            [2, CharacterTitle::Beastly.id()]
        );
        assert_eq!(
            title_command(&owner, &mut admitted, &[3, CharacterTitle::Beastly.id()])
                .await?
                .0,
            CommandStatus::Accepted
        );
        let upgraded = charm_command(
            &owner,
            &mut admitted,
            CharmProgressionAction::Unlock {
                charm,
                expected_stage: 1,
            },
        )
        .await?;
        assert!(upgraded.charms.charm_points_available < 2_000);
        let (status, payload) = title_command(&owner, &mut admitted, &[0, 0, 0, 1]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            payload.last().copied(),
            Some(CharacterTitle::Beastly.id()),
            "permanent selected title survives spending and connection reload"
        );
        assert_eq!(title_count(&h, CharacterTitle::Beastly).await?, 1);
        let selected: Option<i16> = sqlx::query_scalar("SELECT title FROM game_character_title_selection WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(selected, Some(i16::from(CharacterTitle::Beastly.id())));
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
