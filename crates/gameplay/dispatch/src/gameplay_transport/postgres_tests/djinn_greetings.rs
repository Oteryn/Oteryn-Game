use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_gameplay_domain::domain::condition::{ConditionValues, DotElement};
use oteryn_protocol_oteryn::npc_service::NpcTalkIntent;

#[test]
fn baa_leal_soulfire_postgres_npc_talk() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            run(async |admin| {
                let h = Harness::create_with_admission_seed(admin, "baa_leal", true, false).await?;
                prepare_current_session(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let deps = Dependencies::new()?;
                let owner = deps.owner(&h, &authority);
                let actor = deps.actor;
                let session = deps.admitted.game_session_id;
                let position = {
                    let mut runtime = deps.runtime.lock().await;
                    runtime
                        .initialize_first_entry_position(actor)
                        .map_err(debug)?;
                    let mut spells = owner.states.spell_states.lock().await;
                    spells
                        .initialize(
                            &runtime,
                            actor,
                            session,
                            crate::gameplay_transport::actor_spell::tests::FACTS,
                            (0, 0),
                            owner.owner_now(),
                        )
                        .ok_or("player state")?;
                    runtime
                        .read_actor_position(actor)
                        .map_err(debug)?
                        .position()
                };
                let [npc] = super::questl2_fixture::install_at(
                    &owner,
                    &deps.admitted,
                    ["oteryn:npc.baa_leal"],
                    [oteryn_gameplay_quests::quest::world_callbacks::Position {
                        x: u16::try_from(position.x + 1)?,
                        y: u16::try_from(position.y)?,
                        floor: -i8::try_from(position.floor)?,
                    }],
                )
                .await?;
                let fence = owner
                    .current_quest_fence(session)
                    .await
                    .map_err(|()| "quest fence")?
                    .ok_or("quest fence")?
                    .item_fence();
                for (command_id, text) in [(1, "DJANNI'HAH"), (2, "hi")] {
                    owner
                        .npc_talk(
                            actor,
                            UseCommand {
                                command_id,
                                game_session_id: session,
                                item_fence: Some(fence),
                            },
                            NpcTalkIntent {
                                npc_actor: npc,
                                text: text.into(),
                            },
                        )
                        .await;
                    let runtime = deps.runtime.lock().await;
                    let spells = owner.states.spell_states.lock().await;
                    let store = spells
                        .owned_source_condition_fixture_snapshot(&runtime, actor, session)
                        .ok_or("player condition store")?;
                    let fire = store
                        .instances()
                        .iter()
                        .find(|instance| instance.definition().key() == "npc.baa_leal.soulfire");
                    if command_id == 1 {
                        assert!(fire.is_none());
                    } else {
                        assert_eq!(
                            fire.ok_or("wrong greeting did not apply Soulfire")?
                                .definition()
                                .values(),
                            ConditionValues::DamageOverTime {
                                element: DotElement::Fire,
                                total_min: 1_500,
                                total_max: 1_500,
                                per_tick: 10,
                                interval_ms: 5_000,
                                delayed: true,
                            },
                        );
                    }
                }
                drop(owner);
                drop(authority);
                drop(seal);
                h.cleanup().await
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Baa'leal PostgreSQL test thread panicked")??;
    Ok(())
}
