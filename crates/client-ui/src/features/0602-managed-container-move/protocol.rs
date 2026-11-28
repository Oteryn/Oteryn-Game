use super::{Batch, BatchKind, container_batch::Request};
use oteryn_client_runtime::raw_feature::FeatureCommand;

pub fn managed_move(batch: &mut Batch, request: Request) -> Option<FeatureCommand> {
    super::batch::request(batch, BatchKind::ManagedMove, request)
}
