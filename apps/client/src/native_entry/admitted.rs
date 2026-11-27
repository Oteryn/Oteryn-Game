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
    /// The World's Channels as the directory listed them (chsw, capabilities 512 and 513).
    pub(super) channels: Vec<crate::character_selection::ChannelChoice>,
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
    pub fn channels(&self) -> &[crate::character_selection::ChannelChoice] {
        &self.channels
    }

    #[must_use]
    pub fn channel_id(&self) -> &str {
        &self.channel_id
    }

    /// The server-owned join map, actors and overlays.
    pub fn play_view(&self) -> Result<crate::play::PlayView, oteryn_renderer::BatchError> {
        admitted_play_view(&self.session, self.preference_character_id, self.world_id())
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

/// Install the admitted cache scope before the join map can load or update it.
pub(crate) fn admitted_play_view<S: oteryn_session::SessionStream>(
    session: &Session<S>,
    character: oteryn_session::CharacterId,
    world: &str,
) -> Result<crate::play::PlayView, oteryn_renderer::BatchError> {
    let mut view = crate::play::PlayView::from_join(session.join_snapshot())?;
    view.bind_admitted_minimap(world, *character.as_bytes());
    view.with_join_scene(session)
}
