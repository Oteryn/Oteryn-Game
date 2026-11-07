use super::*;
use std::sync::Arc;

/// Complete replacement snapshot, never an opaque player packet. No account,
/// character IDs, labels, UI, private text, inventory or combat telemetry.
pub use oteryn_protocol_oteryn::cast::Scene;
pub(super) struct Queued {
    pub due_ms: u64,
    pub content: Content,
    pub consents: Arc<[(CharacterId, i64)]>,
}
impl Queued {
    pub fn plain(due_ms: u64, content: Content) -> Self {
        Self {
            due_ms,
            content,
            consents: Arc::from([]),
        }
    }
}
impl CastOwner {
    /// Trusted Game settings owner only. A wire request cannot supply consent
    /// facts. Equal revision/value refreshes freshness; rollback/substitution
    /// fails closed. Cross-node settings invalidation must feed this barrier.
    pub fn consent(&mut self, character: CharacterId, value: CastConsent) -> Result<(), Error> {
        if self
            .consent_floor
            .get(&character)
            .is_some_and(|floor| value.revision < *floor)
        {
            return Err(Error::Conflict);
        }
        if value.revision < 0 || (value.allowed && value.revision == 0) {
            return Err(Error::Invalid);
        }
        if let Some((old, _)) = self.consents.get(&character) {
            if old.revision > value.revision || (old.revision == value.revision && *old != value) {
                return Err(Error::Conflict);
            }
        } else if self.consents.len() >= self.node_capacity * 128 {
            return Err(Error::Full);
        }
        self.consents.insert(character, (value, self.clock_ms));
        if !value.allowed {
            self.purge_character(character);
        }
        Ok(())
    }
    /// Invalidation barrier used BEFORE a durable consent withdrawal. It never
    /// creates a fake preference revision and purges already queued/shown scenes.
    pub fn invalidate(&mut self, character: CharacterId, through: i64) -> Result<(), Error> {
        if self.consent_floor.len() >= self.node_capacity * 128
            && !self.consent_floor.contains_key(&character)
        {
            for target in self.casts.keys().copied().collect::<Vec<_>>() {
                self.end(target);
            }
            self.audit.failed = true;
            return Err(Error::Full);
        }
        let minimum = through.saturating_add(1);
        self.consent_floor
            .entry(character)
            .and_modify(|v| *v = (*v).max(minimum))
            .or_insert(minimum);
        self.consents.remove(&character);
        self.purge_character(character);
        Ok(())
    }
    fn purge_character(&mut self, character: CharacterId) {
        for viewer in self.viewers.values_mut() {
            if viewer.shown.iter().any(|(id, _)| *id == character)
                || viewer
                    .queue
                    .iter()
                    .any(|q| q.consents.iter().any(|(id, _)| *id == character))
            {
                viewer.queue.clear();
                viewer.shown = Arc::from([]);
                viewer
                    .queue
                    .push_back(Queued::plain(self.clock_ms, Content::Clear));
                let delay = self
                    .casts
                    .get(&viewer.target)
                    .map_or(0, |c| u64::from(c.settings.delay_seconds) * 1000);
                viewer
                    .queue
                    .push_back(Queued::plain(self.clock_ms + delay, Content::Slate));
            }
        }
    }
    pub fn slate(&mut self, facts: TargetFacts, now_ms: u64) -> Result<(), Error> {
        self.tick(now_ms)?;
        self.refresh(facts);
        let target = facts.authority.current_game_session_id();
        let cast = self.casts.get(&target).ok_or(Error::Unavailable)?;
        let due = now_ms + u64::from(cast.settings.delay_seconds) * 1000;
        for viewer in self.viewers.values_mut().filter(|v| v.target == target) {
            viewer.queue.clear();
            viewer.shown = Arc::from([]);
            viewer
                .queue
                .push_back(Queued::plain(now_ms, Content::Clear));
            viewer.queue.push_back(Queued::plain(due, Content::Slate));
        }
        Ok(())
    }
    /// Caller supplies the committed target-visible player set from the Game
    /// interest owner, not a caster/viewer assertion. No hidden-player input.
    pub fn publish(
        &mut self,
        facts: TargetFacts,
        visible: &[CharacterId],
        scene: Scene,
        now_ms: u64,
    ) -> Result<(), Error> {
        self.tick(now_ms)?;
        self.refresh(facts);
        let target = facts.authority.current_game_session_id();
        if visible.len() > 128
            || scene.tiles.len() > 512
            || scene.actors.len() > 128
            || scene.actors.iter().any(|a| a.4 > 100)
        {
            self.end(target);
            return Err(Error::Invalid);
        }
        let cast = self.casts.get(&target).ok_or(Error::Unavailable)?;
        let caster = cast.facts.authority.commit().character_id();
        let mut bindings = Vec::new();
        let mut permitted = true;
        for character in visible
            .iter()
            .filter(|id| id.as_bytes() != caster.as_bytes())
        {
            if let Some((consent, observed)) = self
                .consents
                .get(character)
                .filter(|(c, observed)| c.allowed && now_ms.saturating_sub(*observed) <= 5000)
            {
                let _ = observed;
                bindings.push((*character, consent.revision));
            } else {
                permitted = false;
            }
        }
        let content = if permitted {
            Content::Scene(Arc::new(scene))
        } else {
            bindings.clear();
            Content::Slate
        };
        let bindings: Arc<[(CharacterId, i64)]> = bindings.into();
        let due_ms = now_ms
            .checked_add(u64::from(cast.settings.delay_seconds) * 1000)
            .ok_or(Error::Invalid)?;
        let mut slow = Vec::new();
        for (id, viewer) in self.viewers.iter_mut().filter(|(_, v)| v.target == target) {
            if viewer.queue.len() >= VIEWER_QUEUE_ITEMS {
                slow.push(*id);
            } else {
                viewer.queue.push_back(Queued {
                    due_ms,
                    content: content.clone(),
                    consents: Arc::clone(&bindings),
                });
            }
        }
        for id in slow {
            self.leave(id)?;
        }
        Ok(())
    }
}

impl CastOwner {
    /// Same consent binding and delay as scenes, including final-write purge.
    /// Map chunks and public events never bypass a neutral privacy slate.
    pub fn publish_media(
        &mut self,
        facts: TargetFacts,
        visible: &[CharacterId],
        media: Vec<oteryn_protocol_oteryn::cast::PublicMedia>,
        now_ms: u64,
    ) -> Result<(), Error> {
        self.tick(now_ms)?;
        self.refresh(facts);
        if media.len() > 64 || media.iter().any(|m| m.encode().is_err()) {
            return Err(Error::Invalid);
        }
        let target = facts.authority.current_game_session_id();
        let cast = self.casts.get(&target).ok_or(Error::Unavailable)?;
        let mut bindings = Vec::new();
        for character in visible
            .iter()
            .filter(|id| id.as_bytes() != facts.authority.commit().character_id().as_bytes())
        {
            let Some((consent, _)) = self
                .consents
                .get(character)
                .filter(|(c, observed)| c.allowed && now_ms.saturating_sub(*observed) <= 5000)
            else {
                return self.slate(facts, now_ms);
            };
            bindings.push((*character, consent.revision));
        }
        let due_ms = now_ms + u64::from(cast.settings.delay_seconds) * 1000;
        let bindings: Arc<[(CharacterId, i64)]> = bindings.into();
        let media: Vec<_> = media
            .into_iter()
            .map(|m| Content::Media(Arc::new(m)))
            .collect();
        let mut slow = Vec::new();
        for (id, viewer) in self.viewers.iter_mut().filter(|(_, v)| v.target == target) {
            if viewer.queue.len() + media.len() > VIEWER_QUEUE_ITEMS {
                slow.push(*id);
                continue;
            }
            for content in &media {
                viewer.queue.push_back(Queued {
                    due_ms,
                    content: content.clone(),
                    consents: Arc::clone(&bindings),
                });
            }
        }
        for id in slow {
            self.leave(id)?;
        }
        Ok(())
    }
}
