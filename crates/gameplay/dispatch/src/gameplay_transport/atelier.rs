//! Native dispatch boundary for Gem Atelier/Fragment Workshop commands.

pub use oteryn_gameplay_progression::atelier::{
    AtelierAction, AtelierCommit, AtelierRefusal, AtelierState,
};

/// The single native action path. Durable adapters load a state under the character fence,
/// invoke this owner, then persist the successful state and receipt atomically.
pub fn dispatch_atelier(
    state: &mut AtelierState,
    expected_revision: u64,
    action: AtelierAction,
) -> Result<AtelierCommit, AtelierRefusal> {
    state.commit(expected_revision, action)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test fixtures")]
mod tests {
    use super::*;
    use oteryn_gameplay_progression::{
        atelier::{FragmentKind, GemQuality},
        wheel::WheelDomain,
    };

    #[test]
    fn real_dispatch_reveals_sockets_dismantles_and_grades_without_partial_refusals() {
        let mut state = AtelierState {
            gold: 100_000_000,
            ..AtelierState::default()
        };
        dispatch_atelier(
            &mut state,
            0,
            AtelierAction::Drop {
                quality: GemQuality::Greater,
            },
        )
        .unwrap();
        dispatch_atelier(
            &mut state,
            1,
            AtelierAction::Reveal {
                id: 1,
                quality: GemQuality::Greater,
                domain: WheelDomain::Green,
                mods: vec![0, 1, 0],
            },
        )
        .unwrap();
        dispatch_atelier(&mut state, 2, AtelierAction::Socket { id: 1 }).unwrap();
        let before = state.clone();
        assert_eq!(
            dispatch_atelier(
                &mut state,
                3,
                AtelierAction::Dismantle {
                    id: 1,
                    yield_amount: 3
                }
            ),
            Err(AtelierRefusal::InVessel)
        );
        assert_eq!(state, before);
        dispatch_atelier(&mut state, 3, AtelierAction::Remove { id: 1 }).unwrap();
        dispatch_atelier(
            &mut state,
            4,
            AtelierAction::Drop {
                quality: GemQuality::Lesser,
            },
        )
        .unwrap();
        dispatch_atelier(
            &mut state,
            5,
            AtelierAction::Reveal {
                id: 2,
                quality: GemQuality::Lesser,
                domain: WheelDomain::Green,
                mods: vec![0],
            },
        )
        .unwrap();
        dispatch_atelier(
            &mut state,
            6,
            AtelierAction::Dismantle {
                id: 1,
                yield_amount: 3,
            },
        )
        .unwrap();
        state.fragments.insert(FragmentKind::Basic, 50);
        for expected in 7..10 {
            dispatch_atelier(
                &mut state,
                expected,
                AtelierAction::GradeUp {
                    kind: FragmentKind::Basic,
                    mod_id: 0,
                },
            )
            .unwrap();
        }
        assert_eq!(state.grades[&(FragmentKind::Basic, 0)], 3);
    }
}
