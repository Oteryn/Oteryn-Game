use oteryn_client_runtime::charm_progression as wire;
pub(super) fn reply() -> std::result::Result<wire::CharmProgressionReply, Box<dyn std::error::Error>>
{
    let id = |n| std::num::NonZeroU32::new(n).ok_or("id");
    Ok(wire::CharmProgressionReply {
        outcome: wire::CharmProgressionOutcome::Query,
        fee_gold: 0,
        bestiary: vec![wire::bestiary::BestiaryRaceProgress {
            race: id(7)?,
            kill_count: 321,
            kill_thresholds: [1, 20, 321],
        }],
        charms: wire::charm::CharmView {
            charm_points_available: 900,
            charms: vec![wire::charm::CharmState {
                charm: id(1)?,
                kind: wire::charm::CharmKind::Major,
                unlocked_stage: 0,
                assigned_race: None,
                next_stage_cost: 200,
                effect_active: false,
            }],
            ..Default::default()
        },
    })
}
