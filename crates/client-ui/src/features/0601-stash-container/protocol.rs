use oteryn_client_runtime::{
    raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult},
    stash::container_batch::{self as wire, Reply, Request},
};
use std::num::NonZeroU64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchKind {
    Stash,
    ManagedMove,
}

impl BatchKind {
    pub fn capability(self) -> u32 {
        self.route().0
    }

    fn route(self) -> (u32, u32) {
        match self {
            Self::Stash => (
                wire::CAPABILITY_STASH_CONTAINER_V1,
                wire::COMMAND_TYPE_STASH_CONTAINER,
            ),
            Self::ManagedMove => (
                wire::CAPABILITY_MANAGED_CONTAINER_MOVE_V1,
                wire::COMMAND_TYPE_MANAGED_CONTAINER_MOVE,
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Batch {
    #[default]
    Idle,
    Waiting,
    Quote {
        id: NonZeroU64,
        quantity: u32,
    },
    Committed(u32),
    Unavailable,
    Stale,
}

impl Batch {
    pub fn description(&self, kind: BatchKind) -> Option<String> {
        Some(match self {
            Self::Idle => return None,
            Self::Waiting => "Waiting for the server…".into(),
            Self::Quote { quantity, .. } => match kind {
                BatchKind::Stash => {
                    format!("Stow {quantity} units from this container and its subcontainers?")
                }
                BatchKind::ManagedMove => {
                    let noun = if *quantity == 1 { "stack" } else { "stacks" };
                    format!("Move {quantity} {noun} to their configured managed containers?")
                }
            },
            Self::Committed(quantity) => match kind {
                BatchKind::Stash => format!("Stowed {quantity} units."),
                BatchKind::ManagedMove => {
                    let noun = if *quantity == 1 { "stack" } else { "stacks" };
                    format!("Moved {quantity} {noun}.")
                }
            },
            Self::Unavailable => "Sorry, not possible.".into(),
            Self::Stale => {
                "Sorry, not possible. The preview is no longer valid. Preview the container again."
                    .into()
            }
        })
    }

    pub fn apply(&mut self, request: Request, result: &GatedCommandResult) {
        if *self != Self::Waiting {
            return;
        }
        let reply = (result.status == CommandStatus::Accepted)
            .then(|| Reply::decode(&result.payload))
            .flatten();
        *self = match (request, reply) {
            (Request::Preview { .. }, Some(Reply::Preview(quantity))) => {
                NonZeroU64::new(result.command_id)
                    .map_or(Self::Unavailable, |id| Self::Quote { id, quantity })
            }
            (Request::Confirm { .. }, Some(Reply::Committed(quantity))) => {
                Self::Committed(quantity)
            }
            (Request::Preview { .. }, _) => Self::Unavailable,
            (Request::Confirm { .. }, _) => Self::Stale,
        };
    }
}

pub fn request(batch: &mut Batch, kind: BatchKind, request: Request) -> Option<FeatureCommand> {
    if matches!(batch, Batch::Waiting)
        || matches!(request, Request::Confirm { preview }
            if !matches!(batch, Batch::Quote { id, .. } if *id == preview))
    {
        return None;
    }
    *batch = Batch::Waiting;
    let (capability, kind) = kind.route();
    Some(FeatureCommand {
        capability,
        kind,
        payload: request.encode(),
    })
}
