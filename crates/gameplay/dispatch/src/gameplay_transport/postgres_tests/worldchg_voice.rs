use super::*;
use crate::durability::world_event::changes::counters::{
    CounterKind, CounterState, Credit, CreditEvent, CreditOutcome,
};
use crate::foundation::{ChannelId, CharacterId, RuntimeScopeRefV1, ScopeOwnershipGeneration, WorldId};

#[path = "world_event/reset_fixture.rs"]
mod event_fixture;

#[test]
fn worldchg_voice_mage_qualification_death_and_empty_restoration() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    super::kill_reward_e2e::scenario_thread::run("worldchg-voice", scenario)
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "worldchg_voice", false, false).await?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let channel = ChannelId::decode(&id(43)).map_err(debug)?;
        let character = CharacterId::decode(&id(41)).map_err(debug)?;
        let scope = RuntimeScopeRefV1::channel(world, channel);
        let (_, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let epoch = (1..=64)
            .find(|epoch| crate::durability::world_event::changes::roll(world, *epoch, 0)
                .is_ok_and(|day| day.their_masters_voice))
            .ok_or("active voice epoch")?;
        event_fixture::reset_epochs(&h, &authority, pin.qualified_room(), world, channel, epoch).await?;
        event_fixture::admit(&h, epoch + 1).await?;
        let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&h.pool).await?;
        let deadline = now + 60_000;
        let kind = CounterKind::TheirMastersVoice;
        let mut state = CounterState::new(kind, epoch, false);
        state.stage = 3;
        state.counts = [100, 25, (1 << 25) - 1];
        state.deadline_ms = Some(u64::try_from(deadline)?);
        sqlx::query("UPDATE game_world_change_counters SET state=$1 WHERE change_key='their_masters_voice'")
            .bind(serde_json::to_value(state)?).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_world_change_personal_counters(world_id,change_key,character_id,counter,period_deadline_ms) VALUES(encode($1,'hex')::uuid,'their_masters_voice',encode($2,'hex')::uuid,24,$3)")
            .bind(world.as_bytes().as_slice()).bind(character.as_bytes().as_slice()).bind(deadline).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_terrain_tool_transforms(world_id,channel_id,placement,reset_epoch,map_revision,content_generation,state_item_id,object_revision,decay_states,decay_delays,decay_at_unix_ms) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,'map-1',$5,12065,1,ARRAY[12059]::bigint[],ARRAY[60000]::bigint[],$6)")
            .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
            .bind(placement()?).bind(i64::try_from(epoch)?).bind([7u8; 32].as_slice())
            .bind(deadline).execute(&h.pool).await?;
        let day = h.root.world_changes(world, epoch).await?.ok_or("active change")?;
        assert!(day.their_masters_voice);
        let mut occurrence = world.as_bytes().to_vec();
        occurrence.extend_from_slice(channel.as_bytes());
        occurrence.extend_from_slice(&(epoch + 1).to_be_bytes());
        occurrence.extend_from_slice(&[1; 12]);
        let event = CreditEvent {
            source_kind: 1,
            occurrence,
            credits: vec![(kind, Credit::VoiceMage)],
            counter_generations: day.counter_generations,
            occurred_at_ms: u64::try_from(now)?,
            character: Some(character),
        };
        let generation = ScopeOwnershipGeneration::new(epoch + 1).map_err(debug)?;
        assert_eq!(
            h.root.credit_world_change_event(&authority, &h.node, scope, generation, epoch, event.clone()).await?,
            CreditOutcome::Refused,
        );
        sqlx::query("UPDATE game_world_change_personal_counters SET counter=25 WHERE change_key='their_masters_voice'")
            .execute(&h.pool).await?;
        assert_eq!(
            h.root.credit_world_change_event(&authority, &h.node, scope, generation, epoch, event).await?,
            CreditOutcome::Applied { phase_changed: true },
        );
        let terrain: (i64, i64, Option<i64>) = sqlx::query_as(
            "SELECT state_item_id,object_revision,decay_at_unix_ms FROM game_terrain_tool_transforms",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(terrain, (12059, 2, None));
        sqlx::query("UPDATE game_terrain_tool_transforms SET state_item_id=12065,decay_step=0,decay_at_unix_ms=$1")
            .bind(deadline).execute(&h.pool).await?;
        for (empty, expected) in [(false, 12065), (true, 12059)] {
            let mut empty = empty;
            assert!(h.root.project_world_change_counters(
                &authority, &h.node, scope, generation, epoch, &mut empty,
                |empty, _| Ok(*empty),
            ).await?);
            let appearance: i64 = sqlx::query_scalar(
                "SELECT state_item_id FROM game_terrain_tool_transforms",
            ).fetch_one(&h.pool).await?;
            assert_eq!(appearance, expected);
        }
        Ok(())
    })
}


fn placement() -> TestResult<Vec<u8>> {
    let mut placement = vec![7; 32];
    let key = oteryn_world_bundle::bundle::placement_key(-9, 33312, 31872, 0)
        .ok_or("fungus placement")?;
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(placement)
}
