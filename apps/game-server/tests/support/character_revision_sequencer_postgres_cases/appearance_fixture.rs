//! APP-3 setup reuses the sequencer harness with authenticated sex-bearing creation facts.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum CreationFixture {
    Historical,
    Own,
    OwnFemale,
    Observer,
}

impl CreationFixture {
    pub(super) fn sex(self) -> Option<i16> {
        match self {
            Self::Historical => None,
            Self::OwnFemale => Some(1),
            Self::Own | Self::Observer => Some(2),
        }
    }
}

impl Harness {
    #[allow(dead_code, reason = "used by the auto-mounted PVP-RT-1 dispatch tests")]
    pub(crate) async fn seed_pvp_target(&self) -> TestResult {
        seed_character(
            &self.pool,
            &self.root,
            &self.node,
            false,
            false,
            CreationFixture::Observer,
            "starter-1",
        )
        .await
    }

    #[allow(dead_code, reason = "used by native APP-3 observer qualification")]
    pub(crate) async fn seed_appearance_observer(&self) -> TestResult {
        seed_character(
            &self.pool,
            &self.root,
            &self.node,
            true,
            false,
            CreationFixture::Observer,
            "starter-1",
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
        Self::create_seeded(admin, tag, true, false, CreationFixture::Own).await
    }

    #[allow(
        dead_code,
        reason = "used by the QENGINE reward dispatch library cases"
    )]
    pub(crate) async fn create_quest_reward(
        admin: String,
        tag: &str,
        female: bool,
    ) -> TestResult<Self> {
        Self::create_seeded(
            admin,
            tag,
            false,
            false,
            if female {
                CreationFixture::OwnFemale
            } else {
                CreationFixture::Own
            },
        )
        .await
    }
}
