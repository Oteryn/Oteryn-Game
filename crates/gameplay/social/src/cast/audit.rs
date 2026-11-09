use super::*;
use oteryn_gameplay_durability::durability::{
    DurabilityRoot,
    cast_audit::{CastAuditReceipt, CastAuditRecord},
    runtime_scope_assignment::NodeIncarnationProof,
};
use std::collections::VecDeque;
use std::time::Duration;
use tokio::time::Instant;

pub(super) struct Journal {
    pub incarnation: [u8; 16],
    pub ordinal: u64,
    pub committed: u64,
    pub pending: VecDeque<CastAuditRecord>,
    reserved: usize,
    capacity: usize,
    pub failed: bool,
}
impl Journal {
    pub fn new(capacity: usize) -> Result<Self, Error> {
        let mut incarnation = [0; 16];
        aws_lc_rs::rand::fill(&mut incarnation).map_err(|_| Error::Unavailable)?;
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| Error::Unavailable)?
            .as_millis();
        incarnation[..6].copy_from_slice(&millis.to_be_bytes()[10..]);
        incarnation[6] = (incarnation[6] & 15) | 0x70;
        incarnation[8] = (incarnation[8] & 63) | 0x80;
        Ok(Self {
            incarnation,
            ordinal: 0,
            committed: 0,
            pending: VecDeque::new(),
            reserved: 0,
            capacity,
            failed: false,
        })
    }
    pub fn room(&self, extra: usize) -> Result<(), Error> {
        if self.failed
            || self.pending.len() + self.reserved + extra > self.capacity
            || self.ordinal > (i64::MAX as u64).saturating_sub((self.reserved + extra) as u64)
        {
            return Err(Error::Unavailable);
        }
        Ok(())
    }
    pub fn reserve(&mut self) -> Result<(), Error> {
        self.room(2)?;
        self.reserved += 1;
        Ok(())
    }
    pub fn append(
        &mut self,
        kind: u32,
        target: GameSessionId,
        state: State,
        subject: Option<[u8; 16]>,
    ) {
        self.ordinal += 1;
        self.pending.push_back(CastAuditRecord {
            ordinal: self.ordinal,
            kind,
            target_session: target.as_bytes().to_vec(),
            cast_id: state.cast_id.to_vec(),
            revision: state.revision,
            epoch: state.epoch,
            subject_account: subject.map(|v| v.to_vec()),
            ..Default::default()
        });
    }
    pub fn terminal(
        &mut self,
        kind: u32,
        target: GameSessionId,
        state: State,
        subject: Option<[u8; 16]>,
    ) {
        self.reserved -= 1;
        self.append(kind, target, state, subject);
    }
    fn ack(&mut self, receipt: CastAuditReceipt) -> Result<(), Error> {
        if receipt.incarnation() != self.incarnation
            || receipt.through() > self.ordinal
            || receipt.through() < self.committed
        {
            return Err(Error::Conflict);
        }
        self.committed = receipt.through();
        while self
            .pending
            .front()
            .is_some_and(|r| r.ordinal <= self.committed)
        {
            self.pending.pop_front();
        }
        Ok(())
    }
}
impl CastOwner {
    /// Off-tick only; never hold a world/target/Mutex guard across this await.
    /// Failed audit closes casts and purges buffers. Reserved terminals survive
    /// in the journal; only a committed receipt advances its prefix.
    pub async fn persist_audit(
        &mut self,
        root: &DurabilityRoot,
        node: &NodeIncarnationProof,
    ) -> Result<(), Error> {
        // A busy holder has not attempted storage. Give it the same one-second
        // window as cast commands, without retrying storage/receipt failures.
        let retry_deadline = Instant::now() + Duration::from_secs(1);
        loop {
            let batch: Vec<_> = self.audit.pending.iter().take(64).cloned().collect();
            let committed = match root
                .commit_cast_audit(node, self.audit.incarnation, &batch)
                .await
            {
                Ok(receipt) => self.audit.ack(receipt),
                Err(oteryn_gameplay_durability::durability::DurabilityError::RootUnavailable)
                    if Instant::now() < retry_deadline =>
                {
                    let now = Instant::now();
                    root.wait_holder_retry(
                        now + Duration::from_millis(2),
                        (now + Duration::from_millis(50)).min(retry_deadline),
                    )
                    .await;
                    continue;
                }
                Err(_) => Err(Error::Unavailable),
            };
            if committed.is_err() {
                self.audit.failed = true;
                let targets: Vec<_> = self.casts.keys().copied().collect();
                for target in targets {
                    self.end(target);
                }
                return Err(Error::Unavailable);
            }
            if self.audit.pending.is_empty() {
                self.audit.failed = false;
                return Ok(());
            }
        }
    }
    #[cfg(test)]
    pub(super) fn model_audit_commit(&mut self) {
        self.audit.committed = self.audit.ordinal;
        self.audit.pending.clear();
    }
}

#[cfg(feature = "test-support")]
pub mod test_support {
    use super::*;
    /// Test-only stand-in for already verified account material, never a password
    /// verifier and never available in a production dependency configuration.
    pub fn admission(
        state: State,
        account: [u8; 16],
        reservation: [u8; 16],
        access_revision: u64,
    ) -> VerifiedAdmission {
        VerifiedAdmission {
            cast_id: state.cast_id,
            epoch: state.epoch,
            account,
            reservation,
            access_revision,
            expires_ms: 30_000,
            chat: true,
        }
    }
}
