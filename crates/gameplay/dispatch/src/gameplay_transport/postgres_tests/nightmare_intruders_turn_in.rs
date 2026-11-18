//! Crystal 201189f data-global/npc/maelyrra.lua:93-105.
use super::classic_quests::{install, progress, talk};
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
    threatened_dreams, world_callbacks::Position,
};
use std::sync::Arc;

pub(super) const NAME: &str = "nightmare-intruders-turn-in";
const QUEST: &str = "oteryn:quest.threatened_dreams_quest";
const MAELYRRA: &str = "oteryn:npc.maelyrra";
const NIGHTMARE: &str = "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/nightmare_intruders";
const ENFEEBLED: &str =
    "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/enfeebled_count";
const FRAZZLEMAWS: &str = "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/frazzlemaws_count";
const KROAZUR: &str =
    "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/kroazur_kill";

pub(super) fn catalogue(base: &QuestStateCatalogue) -> TestResult<Arc<QuestStateCatalogue>> {
    use QuestComparison as From;
    use QuestEffectKind as Effect;
    let effects = [
        (NIGHTMARE, From::Eq(1), Effect::Set(2)),
        (
            ENFEEBLED,
            From::Any,
            Effect::RequireSumAtLeast {
                minimum: 200,
                other_effect: 2,
            },
        ),
        (FRAZZLEMAWS, From::Any, Effect::Add(0)),
        (KROAZUR, From::Ge(1), Effect::Add(0)),
    ]
    .into_iter()
    .map(|(track, from, effect)| QuestEffect {
        track: track.into(),
        from,
        effect,
    })
    .collect();
    Ok(Arc::new(
        base.with_fixture_transition(QuestTransition {
            key: threatened_dreams::NIGHTMARE_REPORT.into(),
            quest: QUEST.into(),
            effects,
            completes: false,
            experience: None,
            world_change: None,
        })
        .map_err(debug)?,
    ))
}

#[test]
fn maelyrra_checks_both_kill_counters_and_boss_credit_in_the_same_turn_in() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(
            NAME,
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let [npc] = install(owner, admitted, [MAELYRRA]).await?;
    talk(owner, admitted, npc, 2200, "hi").await?;
    talk(owner, admitted, npc, 2201, "mission").await?;
    talk(owner, admitted, npc, 2202, "yes").await?;
    assert_eq!(progress(owner, admitted, authority, NIGHTMARE).await?, 1);

    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    // Fixture supplies results from the qcredit lane under test. The installed
    // session copy stays at its previous counts, so success requires locked reads.
    for (index, (frazzlemaws, kroazur, reply, stage)) in
        [(80, 0, 105, 1), (79, 1, 105, 1), (80, 1, 98, 2)]
            .into_iter()
            .enumerate()
    {
        for (track, value) in [
            (ENFEEBLED, 120i64),
            (FRAZZLEMAWS, frazzlemaws),
            (KROAZUR, kroazur),
        ] {
            sqlx::query(
                "INSERT INTO game_character_quest_tracks (character_id, track_key, quest_key, value, committed_character_revision)
                 SELECT character_id, $2, quest_key, $3, committed_character_revision
                   FROM game_character_quest_states
                  WHERE character_id=encode($1,'hex')::uuid AND quest_key=$4
                 ON CONFLICT (character_id, track_key) DO UPDATE SET value=EXCLUDED.value",
            )
            .bind(fence.character_id.as_bytes().as_slice())
            .bind(track)
            .bind(value)
            .bind(QUEST)
            .execute(&h.pool).await?;
        }
        let command = 2210 + u64::try_from(index)? * 2;
        talk(owner, admitted, npc, command, "mission").await?;
        let lines = talk(owner, admitted, npc, command + 1, "yes").await?;
        assert_eq!(
            lines.as_slice(),
            threatened_dreams::reply(MAELYRRA, reply).ok_or("source reply")?
        );
        assert_eq!(
            progress(owner, admitted, authority, NIGHTMARE).await?,
            stage
        );
    }
    talk(owner, admitted, npc, 2220, "yes").await?;
    assert_eq!(progress(owner, admitted, authority, NIGHTMARE).await?, 3);
    Ok(())
}
