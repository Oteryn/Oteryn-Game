//! Carried and ground container transport. Depot browsing is not covered.
pub use oteryn_session::container_tree::{
    ContainerView, ContainerViewIntent, ContainerViewOutcome, ContainerViews,
    decode_container_views_with_house_capacity, encode_container_views_with_house_capacity,
};
pub use oteryn_session::{ItemEntry, ItemHandle};

pub fn initial<S: oteryn_session::SessionStream>(
    session: &oteryn_session::Session<S>,
) -> Option<ContainerViews> {
    session.container_views().cloned()
}

pub async fn send<S: oteryn_session::SessionStream>(
    session: &mut oteryn_session::Session<S>,
    intent: ContainerViewIntent,
) -> Result<Option<ContainerViewOutcome>, oteryn_session::SessionError> {
    session.container_view(intent).await
}

pub fn feedback(outcome: Option<ContainerViewOutcome>) -> &'static str {
    match outcome {
        Some(ContainerViewOutcome::Opened) => "Opened",
        Some(ContainerViewOutcome::Closed) => "Closed",
        Some(ContainerViewOutcome::Stale) => "Container is no longer visible",
        Some(ContainerViewOutcome::TooFar) => "Container is too far away",
        Some(ContainerViewOutcome::NotAContainer) => "Not a container",
        Some(ContainerViewOutcome::TooManyViews) => "Too many open containers",
        None => "Container request refused",
    }
}
