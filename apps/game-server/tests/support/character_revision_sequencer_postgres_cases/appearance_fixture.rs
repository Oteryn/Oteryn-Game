//! APP-3 setup reuses the sequencer harness with authenticated sex-bearing creation facts.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum CreationFixture {
    Historical,
    Own,
    Observer,
}

impl Harness {
    #[allow(dead_code, reason = "used by native APP-3 observer qualification")]
    pub(crate) async fn seed_appearance_observer(&self) -> TestResult {
        seed_character(
            &self.pool,
            &self.root,
            &self.node,
            true,
            false,
            CreationFixture::Observer,
        )
        .await
    }

    #[allow(dead_code, reason = "used by the native dispatch library harness")]
    pub(crate) async fn restart_appearance_root(&mut self) -> TestResult {
        self.root = DurabilityRoot::connect_test_runtime(&self.database.url)?;
        assert!(self.root.maintain_ready_once().await?);
        Ok(())
    }

    #[allow(dead_code, reason = "used by the native dispatch library harness")]
    pub(crate) async fn create_appearance(admin: String, tag: &str) -> TestResult<Self> {
        Self::create_seeded(admin, tag, true, false, true).await
    }
}
