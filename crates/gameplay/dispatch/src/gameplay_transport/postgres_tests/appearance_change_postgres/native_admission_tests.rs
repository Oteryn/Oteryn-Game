//! Native content/admission negative matrix; each case mutates exactly one input.
#[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
use super::*;

#[cfg(any(not(feature = "pg-tests-g1"), feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
pub(super) async fn qualify(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    change: &appearance::Change,
) -> TestResult {
    for case in 0..8 {
        let mut invalid = change.clone();
        match case {
            0 => invalid.context.content_generation[0] ^= 1,
            1 => invalid.context.cosmetics_digest[0] ^= 1,
            2 => invalid.expected_root_revision += 1,
            3 => invalid.content_revision = "wrong-content".into(),
            4 => invalid.selection.outfit_key = "oteryn:outfit.unknown".into(),
            5 => invalid.selection.addons = 3,
            6 => invalid.selection.colours[0] = u16::MAX,
            _ => invalid.selection.mount_key = Some("oteryn:mount.unknown".into()),
        }
        assert_eq!(
            crate::gameplay_transport::connection::appearance_change::dispatch(
                owner,
                actor,
                UseCommand {
                    command_id: 100 + case,
                    ..command
                },
                &invalid.encode().map_err(debug)?,
            )
            .await,
            Some(appearance::ChangeResult::Rejected),
            "native admission case {case}",
        );
    }
    Ok(())
}
