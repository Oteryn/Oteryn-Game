use super::*;

impl CastOwner {
    /// Consume a verified, purpose/cast/epoch/access-bound, globally reserved
    /// single-use admission. No CharacterId, actor, lease or command sequence.
    /// The opaque input cannot be constructed by the wire decoder.
    pub fn join(
        &mut self,
        admission: &VerifiedAdmission,
        now_ms: u64,
    ) -> Result<ObserverId, Error> {
        self.tick(now_ms)?;
        let (&target, cast) = self
            .casts
            .iter()
            .find(|(_, c)| c.state.cast_id == admission.cast_id)
            .ok_or(Error::Unavailable)?;
        if admission.epoch != cast.state.epoch
            || admission.access_revision != cast.settings.access_revision
            || admission.expires_ms <= now_ms
            || admission.expires_ms > now_ms.saturating_add(30_000)
            || admission.account == [0; 16]
            || admission.reservation == [0; 16]
            || self.used_tickets.contains_key(&admission.reservation)
        {
            return Err(Error::Unavailable);
        }
        // The second ceiling is node-local defense in depth. Platform must
        // reserve the global two/account ceiling before minting this input.
        if cast.state.viewers >= cast.settings.viewer_cap
            || self.viewers.len() >= self.node_capacity
            || self
                .viewers
                .values()
                .filter(|v| v.account == admission.account)
                .count()
                >= 2
            || self.used_tickets.len() >= self.node_capacity * 4
            || self.releases.len() >= self.node_capacity
        {
            return Err(Error::Full);
        }
        let mut id = [0; 16];
        aws_lc_rs::rand::fill(&mut id).map_err(|_| Error::Unavailable)?;
        let id = ObserverId(id);
        if self.viewers.contains_key(&id) {
            return Err(Error::Unavailable);
        }
        self.audit.reserve()?;
        self.audit
            .append(3, target, cast.state, Some(admission.account));
        self.used_tickets
            .insert(admission.reservation, admission.expires_ms);
        self.viewers.insert(
            id,
            Viewer {
                target,
                account: admission.account,
                reservation: admission.reservation,
                expires_ms: admission.expires_ms,
                reservation_expires_ms: admission.expires_ms,
                chat_selected: admission.chat,
                queue: std::collections::VecDeque::new(),
                shown: std::sync::Arc::from([]),
                audit_ordinal: self.audit.ordinal,
            },
        );
        self.casts
            .get_mut(&target)
            .ok_or(Error::Unavailable)?
            .state
            .viewers += 1;
        self.casts
            .get_mut(&target)
            .ok_or(Error::Unavailable)?
            .audit_ordinal = self.audit.ordinal;
        self.queue_metadata(target);
        self.presence(target, id, "joined transmission");
        Ok(id)
    }
    /// Renew only after another successful online Platform lease check. Epoch,
    /// account and reservation substitution cannot extend an existing observer.
    pub fn renew(
        &mut self,
        id: ObserverId,
        admission: &VerifiedAdmission,
        now_ms: u64,
    ) -> Result<(), Error> {
        self.tick(now_ms)?;
        let viewer = self.viewers.get_mut(&id).ok_or(Error::Unavailable)?;
        let cast = self.casts.get(&viewer.target).ok_or(Error::Unavailable)?;
        if admission.chat != viewer.chat_selected
            || admission.reservation != viewer.reservation
            || admission.account != viewer.account
            || admission.cast_id != cast.state.cast_id
            || admission.epoch != cast.state.epoch
            || admission.access_revision != cast.settings.access_revision
            || admission.expires_ms <= now_ms
            || admission.expires_ms > now_ms.saturating_add(5000)
        {
            return Err(Error::Unavailable);
        }
        viewer.reservation_expires_ms = admission.expires_ms;
        viewer.expires_ms = admission.expires_ms;
        Ok(())
    }
    pub fn leave(&mut self, id: ObserverId) -> Result<(), Error> {
        let target = self.viewers.get(&id).ok_or(Error::Unavailable)?.target;
        self.remove_viewer(id)?;
        self.queue_metadata(target);
        Ok(())
    }
    pub(super) fn remove_viewer(&mut self, id: ObserverId) -> Result<(), Error> {
        let viewer = self.viewers.remove(&id).ok_or(Error::Unavailable)?;
        if let Some(cast) = self.casts.get_mut(&viewer.target) {
            cast.state.viewers = cast.state.viewers.saturating_sub(1);
            self.audit
                .terminal(4, viewer.target, cast.state, Some(viewer.account));
            cast.audit_ordinal = self.audit.ordinal;
        }
        self.presence(viewer.target, id, "left transmission");
        self.releases.push(viewer.reservation);
        Ok(())
    }
    /// The off-tick Gateway adapter must durably release these global slots.
    /// Admission backpressures rather than allowing an unbounded release queue.
    pub fn take_releases(&mut self) -> Vec<[u8; 16]> {
        std::mem::take(&mut self.releases)
    }
}
