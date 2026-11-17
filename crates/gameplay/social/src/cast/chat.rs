use super::*;
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatLine {
    pub handle: [u8; 16],
    pub text: Arc<str>,
    pub name: Arc<str>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content {
    Chat(ChatLine),
    Media(Arc<oteryn_protocol_oteryn::cast::PublicMedia>),
    Metadata { viewers: u8, delay_seconds: u16 },
    Clear,
    Slate,
    Scene(Arc<Scene>),
}
#[derive(Default)]
pub(super) struct Room {
    pub(super) last_roster: Vec<[u8; 16]>,
    pub(super) roster_revision: u64,
    pub(super) roster_at: u64,
    muted: HashSet<[u8; 16]>,
    pub(super) presence: VecDeque<ChatLine>,
    caster: VecDeque<ChatLine>,
    buckets: HashMap<[u8; 16], (u64, u8)>,
    second: u64,
    count: u8,
}

impl CastOwner {
    pub fn keepalive(&mut self, id: ObserverId, now_ms: u64) -> Result<(), Error> {
        self.tick(now_ms)?;
        let viewer = self.viewers.get_mut(&id).ok_or(Error::Unavailable)?;
        viewer.expires_ms = now_ms
            .checked_add(30_000)
            .ok_or(Error::Invalid)?
            .min(viewer.reservation_expires_ms);
        Ok(())
    }
    pub fn mute(
        &mut self,
        facts: TargetFacts,
        revision: u64,
        subject: ObserverId,
        muted: bool,
    ) -> Result<(), Error> {
        self.refresh(facts);
        let target = facts.authority.current_game_session_id();
        let viewer = self
            .viewers
            .get(&subject)
            .filter(|v| v.target == target)
            .ok_or(Error::Unavailable)?;
        let cast = self.casts.get_mut(&target).ok_or(Error::Unavailable)?;
        if cast.state.revision != revision {
            return Err(Error::Conflict);
        }
        self.audit.room(1)?;
        if muted {
            if cast.chat.muted.len() >= self.node_capacity * 4 {
                return Err(Error::Full);
            }
            cast.chat.muted.insert(viewer.account);
        } else {
            cast.chat.muted.remove(&viewer.account);
        }
        self.audit.append(
            if muted { 5 } else { 6 },
            target,
            cast.state,
            Some(viewer.account),
        );
        cast.audit_ordinal = self.audit.ordinal;
        Ok(())
    }
    pub fn kick(
        &mut self,
        facts: TargetFacts,
        revision: u64,
        subject: ObserverId,
    ) -> Result<(), Error> {
        self.refresh(facts);
        let target = facts.authority.current_game_session_id();
        let cast = self.casts.get(&target).ok_or(Error::Unavailable)?;
        if cast.state.revision != revision {
            return Err(Error::Conflict);
        }
        if self
            .viewers
            .get(&subject)
            .is_none_or(|v| v.target != target)
        {
            return Err(Error::Unavailable);
        }
        self.audit.room(1)?;
        let account = self
            .viewers
            .get(&subject)
            .ok_or(Error::Unavailable)?
            .account;
        self.audit.append(7, target, cast.state, Some(account));
        self.leave(subject)
    }
    /// This data channel never calls ChatIntent or gameplay dispatch. The client
    /// must render text escaped, without links, embeds or executable actions.
    pub fn chat(&mut self, id: ObserverId, text: &str, now_ms: u64) -> Result<(), Error> {
        self.tick(now_ms)?;
        let text = validate_text(text)?;
        let viewer = self.viewers.get(&id).ok_or(Error::Unavailable)?;
        if !viewer.chat_selected || viewer.audit_ordinal > self.audit.committed {
            return Err(Error::Unavailable);
        }
        let target = viewer.target;
        let cast = self.casts.get_mut(&target).ok_or(Error::Unavailable)?;
        if !cast.settings.chat_enabled || cast.chat.muted.contains(&viewer.account) {
            return Err(Error::Muted);
        }
        if cast.chat.caster.len() >= 64
            || (!cast.chat.buckets.contains_key(&viewer.account)
                && cast.chat.buckets.len() >= self.node_capacity * 4)
        {
            return Err(Error::Full);
        }
        let bucket = cast
            .chat
            .buckets
            .entry(viewer.account)
            .or_insert((now_ms, 3));
        let replenished = (now_ms - bucket.0) / 3000;
        if replenished > 0 {
            bucket.1 = bucket.1.saturating_add(replenished.min(3) as u8).min(3);
            bucket.0 = now_ms;
        }
        if cast.chat.second != now_ms / 1000 {
            cast.chat.second = now_ms / 1000;
            cast.chat.count = 0;
        }
        if bucket.1 == 0 || cast.chat.count >= 20 {
            return Err(Error::Full);
        }
        let due = now_ms
            .checked_add(u64::from(cast.settings.delay_seconds) * 1000)
            .ok_or(Error::Invalid)?;
        bucket.1 -= 1;
        cast.chat.count += 1;
        let line = ChatLine {
            handle: id.0,
            name: format!("Viewer-{}", super::access::hex(id.0)).into(),
            text: text.into(),
        };
        cast.chat.caster.push_back(line.clone()); // Live, regardless of viewer D.
        self.queue(target, due, Content::Chat(line));
        Ok(())
    }
    pub fn reply(&mut self, facts: TargetFacts, text: &str, now_ms: u64) -> Result<(), Error> {
        self.tick(now_ms)?;
        self.refresh(facts);
        let text = validate_text(text)?;
        let target = facts.authority.current_game_session_id();
        let cast = self.casts.get_mut(&target).ok_or(Error::Unavailable)?;
        if !cast.settings.chat_enabled || cast.chat.caster.len() >= 64 {
            return Err(Error::Full);
        }
        if cast.chat.second != now_ms / 1000 {
            cast.chat.second = now_ms / 1000;
            cast.chat.count = 0;
        }
        let bucket = cast.chat.buckets.entry([0; 16]).or_insert((now_ms, 3));
        let replenished = (now_ms - bucket.0) / 3000;
        if replenished > 0 {
            bucket.1 = bucket.1.saturating_add(replenished.min(3) as u8).min(3);
            bucket.0 = now_ms;
        }
        if bucket.1 == 0 || cast.chat.count >= 20 {
            return Err(Error::Full);
        }
        bucket.1 -= 1;
        cast.chat.count += 1;
        let line = ChatLine {
            handle: [0; 16],
            name: "Caster".into(),
            text: text.into(),
        };
        cast.chat.caster.push_back(line.clone());
        let due = now_ms + u64::from(cast.settings.delay_seconds) * 1000;
        self.queue(target, due, Content::Chat(line));
        Ok(())
    }
    /// Resolve moderation only against the caster's current authority and room.
    pub(super) fn ban_subject(
        &mut self,
        facts: TargetFacts,
        revision: u64,
        id: ObserverId,
    ) -> Result<[u8; 16], Error> {
        self.refresh(facts);
        let target = facts.authority.current_game_session_id();
        let cast = self.casts.get(&target).ok_or(Error::Unavailable)?;
        if cast.state.revision != revision {
            return Err(Error::Conflict);
        }
        self.viewers
            .get(&id)
            .filter(|v| v.target == target)
            .map(|v| v.account)
            .ok_or(Error::Unavailable)
    }
    pub(super) fn remove_banned(
        &mut self,
        target: GameSessionId,
        account: [u8; 16],
    ) -> Result<(), Error> {
        let ids: Vec<_> = self
            .viewers
            .iter()
            .filter(|(_, v)| v.target == target && v.account == account)
            .map(|(id, _)| *id)
            .collect();
        for id in ids {
            self.leave(id)?;
        }
        Ok(())
    }
    pub fn take_caster_presence(&mut self, target: GameSessionId) -> Vec<ChatLine> {
        if self
            .casts
            .get(&target)
            .is_none_or(|c| c.audit_ordinal > self.audit.committed)
        {
            return Vec::new();
        }
        self.casts
            .get_mut(&target)
            .map_or_else(Vec::new, |c| c.chat.presence.drain(..).collect())
    }
    pub(super) fn presence(&mut self, target: GameSessionId, id: ObserverId, text: &str) {
        if let Some(cast) = self.casts.get_mut(&target) {
            if cast.chat.presence.len() >= 64 {
                cast.chat.presence.pop_front();
            }
            cast.chat.presence.push_back(ChatLine {
                handle: id.0,
                name: format!("Viewer-{}", super::access::hex(id.0)).into(),
                text: text.into(),
            });
        }
    }
    pub fn take_caster_chat(&mut self, target: GameSessionId) -> Vec<ChatLine> {
        if self
            .casts
            .get(&target)
            .is_none_or(|c| c.audit_ordinal > self.audit.committed)
        {
            return Vec::new();
        }
        self.casts
            .get_mut(&target)
            .map_or_else(Vec::new, |c| c.chat.caster.drain(..).collect())
    }
    pub(super) fn queue(&mut self, target: GameSessionId, due: u64, content: Content) {
        let mut slow = Vec::new();
        for (id, viewer) in self.viewers.iter_mut().filter(|(_, v)| v.target == target) {
            if matches!(content, Content::Chat(_)) && !viewer.chat_selected {
                continue;
            }
            // Chat overflow throttles only chat; it never waits on a viewer.
            if viewer.queue.len() < VIEWER_QUEUE_ITEMS {
                viewer
                    .queue
                    .push_back(super::projection::Queued::plain(due, content.clone()));
            } else if !matches!(content, Content::Chat(_)) {
                slow.push(*id);
            }
        }
        for id in slow {
            let _ = self.remove_viewer(id);
        }
    }
    pub(super) fn queue_metadata(&mut self, target: GameSessionId) {
        if let Some(c) = self.casts.get(&target) {
            self.queue(
                target,
                self.clock_ms + u64::from(c.settings.delay_seconds) * 1000,
                Content::Metadata {
                    viewers: c.state.viewers,
                    delay_seconds: c.settings.delay_seconds,
                },
            );
        }
    }
    /// Worker-only drain. Production egress must serialize this with the final
    /// nonblocking write and revocation barrier, never with a world/tick lock.
    pub(super) fn drain(&mut self, id: ObserverId, now_ms: u64) -> Result<Vec<Content>, Error> {
        self.tick(now_ms)?;
        let viewer = self.viewers.get_mut(&id).ok_or(Error::Unavailable)?;
        let blocked = viewer.audit_ordinal > self.audit.committed
            || self
                .casts
                .get(&viewer.target)
                .is_none_or(|c| c.audit_ordinal > self.audit.committed);
        let mut ready = Vec::new();
        while ready.len() < EGRESS_BATCH
            && viewer.queue.front().is_some_and(|item| {
                item.due_ms <= now_ms && (!blocked || matches!(item.content, Content::Clear))
            })
        {
            if let Some(item) = viewer.queue.pop_front() {
                if item.consents.iter().any(|(character, revision)| {
                    self.consents.get(character).is_none_or(|(c, observed)| {
                        !c.allowed
                            || c.revision != *revision
                            || now_ms.saturating_sub(*observed) > 5000
                    })
                }) {
                    viewer.queue.clear();
                    viewer.shown = Arc::from([]);
                    ready.push(Content::Clear);
                    ready.push(Content::Slate);
                    break;
                }
                if matches!(item.content, Content::Scene(_) | Content::Media(_)) {
                    viewer.shown = item.consents;
                }
                if matches!(item.content, Content::Clear | Content::Slate) {
                    viewer.shown = Arc::from([]);
                }
                ready.push(item.content);
            }
        }
        Ok(ready)
    }
    /// Exclusive owner access serializes revocation with each synchronous,
    /// nonblocking write. No frames are handed to an asynchronous writer queue.
    pub fn flush(
        &mut self,
        id: ObserverId,
        now_ms: u64,
        write: &mut dyn FnMut(&Content) -> bool,
    ) -> Result<(), Error> {
        for item in self.drain(id, now_ms)? {
            if !write(&item) {
                self.leave(id)?;
                return Err(Error::Unavailable);
            }
        }
        Ok(())
    }
}

fn validate_text(text: &str) -> Result<String, Error> {
    if text.is_empty() || text.len() > 256 || text.chars().any(|c| c.is_control() || matches!(c, '\u{061c}' | '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')) {
        return Err(Error::Invalid);
    }
    let text: String = text.trim().nfc().collect();
    let lower = text.to_ascii_lowercase();
    if text.is_empty() || text.len() > 256 || lower.contains("://") || lower.contains("www.") {
        return Err(Error::Invalid);
    }
    Ok(text)
}
