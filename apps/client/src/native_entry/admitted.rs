use crate::traffic::{Counting, Traffic};
use oteryn_session::Session;
use oteryn_session_tcp::TcpTlsStream;
use std::sync::Arc;

/// An admitted game session on the routed World and Channel.
#[derive(Debug)]
pub struct AdmittedSession {
    pub(super) character_id: String,
    pub(super) preference_character_id: oteryn_session::CharacterId,
    pub(super) character_name: String,
    pub(super) training_requested: bool,
    pub(super) session: Session<Counting<TcpTlsStream>>,
    pub(super) traffic: Arc<Traffic>,
    pub(super) world_id: String,
    pub(super) channel_id: String,
}

impl AdmittedSession {
    pub(crate) fn bulletin(&self) -> Option<&str> {
        self.session.bulletin()
    }
    #[must_use]
    pub fn training_requested(&self) -> bool {
        self.training_requested
    }

    pub fn character_id(&self) -> &str {
        &self.character_id
    }
    #[must_use]
    pub fn character_name(&self) -> &str {
        &self.character_name
    }

    #[must_use]
    pub fn world_id(&self) -> &str {
        &self.world_id
    }

    #[must_use]
    pub fn channel_id(&self) -> &str {
        &self.channel_id
    }

    /// The server-owned join map, actors and overlays.
    pub fn play_view(&self) -> Result<crate::play::PlayView, oteryn_renderer::BatchError> {
        let mut view = crate::play::PlayView::from_join(self.session.join_snapshot())?;
        view.bind_minimap_world(self.world_id());
        view.set_preference_character(*self.preference_character_id.as_bytes());
        view.with_join_scene(&self.session)
    }

    /// The session's byte counters, for the diagnostics overlay.
    #[must_use]
    pub fn traffic(&self) -> Arc<Traffic> {
        Arc::clone(&self.traffic)
    }

    #[must_use]
    pub fn into_session(self) -> Session<Counting<TcpTlsStream>> {
        self.session
    }
}
