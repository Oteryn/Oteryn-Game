//! FND-04B §5: planned reset is separate from unexpected loss and shutdown.
use super::WorldId;

/// Inert classification from a sealed owning source. A reset binding is
/// independently revalidated against the durable RETIRING record at commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlLossCauseV1 {
    AuthoritativeUnexpectedLoss,
    PlannedWorldReset {
        world_id: WorldId,
        reset_epoch: u64,
        retirement_binding: [u8; 32],
        target_digest: [u8; 32],
    },
    HealthyController,
    SocketClosedOnly,
    ProcessRestartOnly,
    GracefulLogout,
    HealthyMigration,
    Suspected,
}

impl ControlLossCauseV1 {
    pub(super) fn admits(self, session_world: WorldId) -> bool {
        match self {
            Self::AuthoritativeUnexpectedLoss => true,
            Self::PlannedWorldReset {
                world_id,
                reset_epoch,
                retirement_binding,
                target_digest,
            } => {
                world_id == session_world
                    && reset_epoch < u64::MAX
                    && retirement_binding != [0; 32]
                    && target_digest != [0; 32]
            }
            Self::HealthyController
            | Self::SocketClosedOnly
            | Self::ProcessRestartOnly
            | Self::GracefulLogout
            | Self::HealthyMigration
            | Self::Suspected => false,
        }
    }
}
