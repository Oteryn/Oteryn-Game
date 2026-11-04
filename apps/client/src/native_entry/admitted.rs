use oteryn_session::Session;
use oteryn_session_tcp::TcpTlsStream;

/// An admitted game session on the routed World and Channel.
#[derive(Debug)]
pub struct AdmittedSession {
    pub(super) session: Session<TcpTlsStream>,
    pub(super) world_id: String,
    pub(super) channel_id: String,
}

impl AdmittedSession {
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
        if let Some(map) = self.session.world_map_snapshot() {
            view.set_live_map(&map)?;
        }
        view.with_entities(
            self.session
                .world_entities()
                .map(|entities| entities.others().copied().collect())
                .unwrap_or_default(),
        )
    }

    #[must_use]
    pub fn into_session(self) -> Session<TcpTlsStream> {
        self.session
    }
}
