//! Selected Premium TALK briefings use the real catalogue and NPC transport.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

pub(super) const PREFIX: &str = "qengine-rewards-m01a-premium-";

#[test]
fn m01a_ivalisse_premium_production_talk_postgres() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-m01a-premium-ivalisse")
}

#[test]
fn m01a_eshaya_premium_production_talk_postgres() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-m01a-premium-eshaya")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let (quest, npc) = match name {
        "qengine-rewards-m01a-premium-ivalisse" => (
            "oteryn:quest.forgotten_knowledge_quest",
            "oteryn:npc.ivalisse",
        ),
        "qengine-rewards-m01a-premium-eshaya" => {
            ("oteryn:quest.kilmaresh_quest", "oteryn:npc.eshaya")
        }
        _ => return Err("unknown Premium TALK fixture".into()),
    };
    super::qengine_m01a_sven::exercise_route(owner, admitted, h, quest, npc).await
}
