//! Runtime adapters for qualified Content, kept above the immutable Content layer.

use crate::ChannelRuntimeV1;
use oteryn_gameplay_content::content::{ContentError, native_gameplay::NativeGameplayState};

/// Installs the artifact-qualified policies into the matching physical Channel owner.
pub trait NativeGameplayRuntimeExt {
    fn install_companion_policies(
        &self,
        runtime: &mut ChannelRuntimeV1,
    ) -> Result<(), ContentError>;
}

impl NativeGameplayRuntimeExt for NativeGameplayState {
    fn install_companion_policies(
        &self,
        runtime: &mut ChannelRuntimeV1,
    ) -> Result<(), ContentError> {
        runtime
            .install_companion_policies(self.policies().clone())
            .map_err(|_| {
                ContentError::InvalidArtifact("native gameplay active creature policy pin")
            })
    }
}
