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

    /// The placeholder view of the join snapshot.
    pub fn play_view(&self) -> Result<crate::play::PlayView, oteryn_renderer::BatchError> {
        crate::play::PlayView::from_join(self.session.join_snapshot())?.with_entities(
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
