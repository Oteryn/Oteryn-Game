//! Pure pinned owners produce bounded data, never physical actors or HP setters.
use super::*;
#[derive(Debug)]
pub(super) enum Planned {
    StartTeleport,
    Spawn {
        key: &'static str,
        around: RoomPosition,
        offset: u8,
        close: bool,
    },
    Icon {
        id: u64,
        count: u8,
    },
    Damage {
        id: u64,
        amount: u32,
    },
    EssenceCount(u8),
    ResetEssence,
    Transform {
        at: RoomPosition,
        from: u16,
        to: u16,
    },
}
fn agony(effect: chag::AgonyEffect) -> Planned {
    match effect {
        chag::AgonyEffect::SetStacks { actor, count } => Planned::Icon { id: actor, count },
        chag::AgonyEffect::Damage(damage) => Planned::Damage {
            id: damage.actor,
            amount: damage.amount,
        },
    }
}
#[allow(
    clippy::too_many_arguments,
    reason = "source facts have independent current native owners"
)]
pub(super) fn plan(
    engine: &mut Engine,
    boss: RoomPosition,
    players: &[Player],
    bloodjaw_target: u8,
    nearby_bloodjaws: u8,
    nearby_pillars: u8,
    nearby_mushrooms: u8,
    now_ms: u64,
    paused: bool,
    active_essences: [bool; 4],
) -> Result<Vec<Planned>, ()> {
    let facts: Vec<_> = players
        .iter()
        .map(|p| chag::AgonyPlayer {
            actor: p.id,
            position: p.position,
            max_health: p.max_health,
        })
        .collect();
    let mut planned = Vec::new();
    match engine {
        Engine::Vemiath {
            think,
            agony: stacks,
        } => {
            for effect in think.think(&vem::VemiathThinkFacts {
                boss,
                mechanics_disabled: false,
                bloodjaw_spawn_paused: paused,
                bloodjaw_target,
                nearby_bloodjaws,
                nearby_pillars,
            }) {
                match effect {
                    vem::VemiathEffect::StartTeleportCycle => planned.push(Planned::StartTeleport),
                    vem::VemiathEffect::Spawn {
                        creature_key,
                        around,
                        offset_tiles,
                    } => planned.push(Planned::Spawn {
                        key: creature_key,
                        around,
                        offset: offset_tiles,
                        close: true,
                    }),
                    vem::VemiathEffect::RunAgonyCheck => {
                        planned.extend(stacks.check(boss, 16, &facts).into_iter().map(agony))
                    }
                }
            }
        }
        Engine::Chagorz(think) => {
            for effect in think.think(&chag::ChagorzThinkFacts {
                boss,
                now_ms,
                mechanics_disabled: false,
                bloodjaw_spawn_paused: paused,
                bloodjaw_target,
                nearby_bloodjaws,
                nearby_pillars,
                players: &facts,
            }) {
                match effect {
                    chag::ChagorzEffect::StartTeleportCycle => planned.push(Planned::StartTeleport),
                    chag::ChagorzEffect::Spawn {
                        creature_key,
                        around,
                        placement,
                    } => {
                        let chag::Placement::Close { offset_tiles } = placement;
                        planned.push(Planned::Spawn {
                            key: creature_key,
                            around,
                            offset: offset_tiles,
                            close: true,
                        });
                    }
                    chag::ChagorzEffect::Agony(effect) => planned.push(agony(effect)),
                }
            }
        }
        Engine::Bakragore {
            think,
            agony: stacks,
        } => {
            let echoes: Vec<_> = players
                .iter()
                .map(|p| bak::EchoPlayer {
                    actor: p.id,
                    position: p.position,
                    taint_level: p.taint,
                })
                .collect();
            // The accepted entry claims actual source flame receipts, including
            // an explicit zero header. Taints/child flags never create essences.
            for effect in think.think(&bak::BakragoreThinkFacts {
                boss,
                mechanics_disabled: false,
                bloodjaw_spawn_paused: paused,
                bloodjaw_target,
                nearby_bloodjaws,
                nearby_mushrooms,
                nearby_pillars,
                active_essences,
                players: &echoes,
            }) {
                match effect {
                    bak::BakragoreEffect::StartTeleportCycle => {
                        planned.push(Planned::StartTeleport)
                    }
                    bak::BakragoreEffect::Spawn {
                        creature_key,
                        around,
                        offset_tiles,
                    } => planned.push(Planned::Spawn {
                        key: creature_key,
                        around,
                        offset: offset_tiles,
                        close: true,
                    }),
                    bak::BakragoreEffect::RunAgonyCheck { room_players } => {
                        stacks.retain_present(&room_players);
                        let present: Vec<_> = facts
                            .iter()
                            .copied()
                            .filter(|p| room_players.contains(&p.actor))
                            .collect();
                        planned.extend(
                            stacks
                                .check(bak::AGONY_ROOM_CENTRE, 20, &present)
                                .into_iter()
                                .map(agony),
                        );
                    }
                    bak::BakragoreEffect::SetEssenceCount { count } => {
                        planned.push(Planned::EssenceCount(count))
                    }
                    bak::BakragoreEffect::SpawnEcho { creature_key, at } => {
                        planned.push(Planned::Spawn {
                            key: creature_key,
                            around: at,
                            offset: 0,
                            close: false,
                        })
                    }
                    bak::BakragoreEffect::ResetEssence { .. } => {
                        planned.push(Planned::ResetEssence)
                    }
                    bak::BakragoreEffect::TransformItem { at, from, to } => {
                        planned.push(Planned::Transform { at, from, to })
                    }
                }
            }
        }
    }
    Ok(planned)
}
