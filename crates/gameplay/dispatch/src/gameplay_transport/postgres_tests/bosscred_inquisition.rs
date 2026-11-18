//! Crystal 201189f8 Inquisition forge exit. The wards share their death window.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_spatial::StepDirection;

pub(super) const NAME: &str = "bosscred-inquisition-forge";
const PORTAL: Position = Position {
    x: 33232,
    y: 31734,
    floor: -11,
};
const WARD: Position = Position {
    x: 33232,
    y: 31733,
    floor: -11,
};
const WEST: Position = Position { x: 33231, ..PORTAL };
const RETREAT: Position = Position {
    x: 33165,
    y: 31709,
    floor: -14,
};
const ARCANUM: Position = Position {
    x: 33038,
    y: 31753,
    floor: -15,
};
const PORTAL_PROGRESS: &str =
    "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/enter_teleport";

#[test]
fn bosscred_inquisition_forge_pair_opens_for_two_players_and_expires() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, PORTAL, false)
    })
}

pub(super) fn fixture_callback(mut adjoining_ward: Callback) -> Callback {
    // Layout only. The scenario uses the production registry for StepIn.
    adjoining_ward.selector.position = Some(PORTAL);
    adjoining_ward.selector.unique = None;
    adjoining_ward.selector.appearance = Some(1949);
    adjoining_ward.teleport = Some(ARCANUM);
    adjoining_ward
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    super::quest_world_callbacks::world_with_cells(callback, PORTAL, &[WARD, WEST, RETREAT])
}

async fn position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    player: &AdmittedSession,
) -> TestResult<crate::foundation::MovementLocalPosition> {
    Ok(owner
        .runtime
        .lock()
        .await
        .read_actor_position(player.runtime_actor.ok_or("player actor")?)
        .map_err(debug)?
        .position())
}

async fn exit(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    player: &mut AdmittedSession,
    expected: Position,
) -> TestResult {
    let actor = player.runtime_actor.ok_or("player actor")?;
    if position(owner, player).await? != super::quest_world_callbacks::local(WEST) {
        super::grizzly_boss::vacancy::teleport(
            owner,
            actor,
            player.game_session_id,
            super::quest_world_callbacks::local(WEST),
        )
        .await?;
    }
    super::questpc7_ferumbras::step(owner, player, StepDirection::East).await?;
    assert_eq!(
        position(owner, player).await?,
        super::quest_world_callbacks::local(expected)
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    player: &mut AdmittedSession,
    deps: &Dependencies,
    h: &Harness,
) -> TestResult {
    super::questpc7_native_deaths::prepare_native_hunter(owner, player).await?;
    let (_, mut witness) = super::grizzly_boss::vacancy::second_player(
        owner,
        deps,
        h,
        super::quest_world_callbacks::local(WARD),
    )
    .await?;
    exit(owner, player, RETREAT).await?;
    exit(owner, &mut witness, RETREAT).await?;

    for (ordinal, creature) in ["oteryn:creature.latrivan", "oteryn:creature.golgordan"]
        .into_iter()
        .enumerate()
    {
        let target = {
            let mut runtime = owner.runtime.lock().await;
            let hp = runtime
                .companion_policy(creature)
                .map_err(debug)?
                .maximum_health;
            let actor = runtime
                .admit_encounter_actor(creature, hp, super::quest_world_callbacks::local(WARD))
                .map_err(debug)?;
            runtime
                .install_creature_policy(actor, creature)
                .map_err(debug)?;
            actor
        };
        super::questpc7_native_deaths::lethal_creature(
            owner,
            player,
            target,
            31_500 + u64::try_from(ordinal)?,
            creature,
        )
        .await?;
        if ordinal == 0 {
            exit(owner, player, RETREAT).await?;
        }
    }

    exit(owner, player, ARCANUM).await?;
    exit(owner, &mut witness, ARCANUM).await?;
    for player in [&*player, &witness] {
        assert_eq!(
            super::questpc7_ferumbras::value(owner, player, PORTAL_PROGRESS).await?,
            4
        );
    }
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(600))
        .ok_or("clock origin")?;
    exit(owner, player, RETREAT).await?;
    exit(owner, &mut witness, RETREAT).await?;
    Ok(())
}
