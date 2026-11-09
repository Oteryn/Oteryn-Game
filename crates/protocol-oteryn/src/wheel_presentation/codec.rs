use super::*;
impl Reply {
    pub fn encode(&self) -> Option<Vec<u8>> {
        if self.nodes.len() != 36
            || self.revelations.len() != 4
            || self.catalogue.is_empty()
            || self.vocation.is_empty()
            || self.unlock_points > self.available
        {
            return None;
        }
        let request = self.request.encode()?;
        let mut b = (request.len() as u16).to_le_bytes().to_vec();
        b.extend(request);
        text(&mut b, &self.catalogue)?;
        text(&mut b, &self.vocation)?;
        b.extend(self.available.to_le_bytes());
        b.extend(self.unlock_points.to_le_bytes());
        b.extend([
            u8::from(self.removal_permitted),
            u8::from(self.eligible),
            u8::from(self.draft_valid),
        ]);
        text(&mut b, &self.reason)?;
        for n in &self.nodes {
            if n.domain > 3
                || n.capacity == 0
                || n.unlock_from.len() > 8
                || n.unlock_from.iter().any(|i| *i >= 36)
                || n.unlock_from.windows(2).any(|p| p[0] >= p[1])
            {
                return None;
            }
            b.push(n.domain);
            b.extend(n.capacity.to_le_bytes());
            b.extend(n.minimum_points.to_le_bytes());
            b.push(n.unlock_from.len() as u8);
            b.extend(&n.unlock_from);
            text(&mut b, &n.name)?;
            lines(&mut b, &n.per_point)?;
            lines(&mut b, &n.full_slot)?;
        }
        for (domain, r) in self.revelations.iter().enumerate() {
            if usize::from(r.domain) != domain
                || r.stages.len() != 3
                || r.stages
                    .windows(2)
                    .any(|p| p[0].minimum_points >= p[1].minimum_points)
            {
                return None;
            }
            b.push(r.domain);
            text(&mut b, &r.name)?;
            for stage in &r.stages {
                b.extend(stage.minimum_points.to_le_bytes());
                lines(&mut b, &stage.effects)?;
            }
        }
        lines(&mut b, &self.current)?;
        lines(&mut b, &self.draft)?;
        (b.len() <= MAX_REPLY_BYTES).then_some(b)
    }
    pub fn decode(b: &[u8]) -> Option<Self> {
        if b.len() > MAX_REPLY_BYTES {
            return None;
        }
        let mut r = Reader(b);
        let n = usize::from(r.u16()?);
        let request = Request::decode(r.bytes(n)?)?;
        let catalogue = r.text()?;
        let vocation = r.text()?;
        let available = r.u32()?;
        let unlock_points = r.u32()?;
        let removal_permitted = r.boolean()?;
        let eligible = r.boolean()?;
        let draft_valid = r.boolean()?;
        let reason = r.text()?;
        let mut nodes = Vec::with_capacity(36);
        for _ in 0..36 {
            let domain = r.byte()?;
            let capacity = r.u16()?;
            let minimum_points = r.u32()?;
            let n = usize::from(r.byte()?);
            if n > 8 {
                return None;
            }
            let unlock_from = r.bytes(n)?.to_vec();
            let name = r.text()?;
            let per_point = r.lines()?;
            let full_slot = r.lines()?;
            nodes.push(Node {
                domain,
                capacity,
                minimum_points,
                unlock_from,
                name,
                per_point,
                full_slot,
            });
        }
        let mut revelations = Vec::with_capacity(4);
        for _ in 0..4 {
            let domain = r.byte()?;
            let name = r.text()?;
            let mut stages = Vec::with_capacity(3);
            for _ in 0..3 {
                let minimum_points = r.u32()?;
                let effects = r.lines()?;
                stages.push(Stage {
                    minimum_points,
                    effects,
                });
            }
            revelations.push(Revelation {
                domain,
                name,
                stages,
            });
        }
        let current = r.lines()?;
        let draft = r.lines()?;
        if !r.0.is_empty() {
            return None;
        }
        let s = Self {
            request,
            catalogue,
            vocation,
            available,
            unlock_points,
            removal_permitted,
            eligible,
            draft_valid,
            reason,
            nodes,
            revelations,
            current,
            draft,
        };
        s.encode()?;
        Some(s)
    }
}
