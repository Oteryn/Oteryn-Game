//! Compatibility facade for the SAME lower Content callback witness/qualifier.
//! Scope, actor, map and occurrence authority remain with physical consumers.
use crate::content::{self, ProjectV2AuthoringProfile, ProjectV2DefinitionRef as Ref};
use crate::creature_auto_attack::AttackError;
#[cfg(test)]
use content::native_gameplay::CreatureProfilesDocument;
use content::native_gameplay::NativeGameplayState;
use content::native_monster_callbacks as lower;
pub use lower::NativeCallbackEntry;
use oteryn_gameplay_foundation::foundation::profile_schedule::ScheduleList;

#[derive(Debug, Clone)]
pub struct QualifiedNativeCallback(lower::QualifiedNativeCallback);
impl QualifiedNativeCallback {
    pub fn entry(&self) -> &NativeCallbackEntry {
        self.0.entry()
    }
    /// The same immutable Content source proof, without a second qualification.
    pub fn as_content_source(&self) -> &lower::QualifiedNativeCallback {
        &self.0
    }
    pub fn content(&self) -> [u8; 32] {
        self.0.content()
    }
    pub fn encounter_profile(&self) -> Result<&'static ProjectV2AuthoringProfile, AttackError> {
        self.0
            .encounter_profile()
            .map_err(|_| AttackError::InvalidSource)
    }
}
fn list(value: ScheduleList) -> lower::NativeCallbackList {
    match value {
        ScheduleList::Attack => lower::NativeCallbackList::Attack,
        ScheduleList::Defence => lower::NativeCallbackList::Defence,
    }
}
pub fn qualify(
    native: &NativeGameplayState,
    creature: &Ref,
    index: usize,
    phase: ScheduleList,
) -> Result<Option<QualifiedNativeCallback>, AttackError> {
    lower::qualify_native_callback(native, creature, index, list(phase))
        .map(|value| value.map(QualifiedNativeCallback))
        .map_err(|_| AttackError::InvalidSource)
}
#[cfg(test)]
pub(super) fn qualify_document(
    document: &CreatureProfilesDocument,
    creature: &Ref,
    index: usize,
    phase: ScheduleList,
) -> Result<Option<NativeCallbackEntry>, AttackError> {
    lower::qualify_native_callback_document(document, creature, index, list(phase))
        .map_err(|_| AttackError::InvalidSource)
}
pub(super) fn native_creature<'a>(
    native: &'a NativeGameplayState,
    target: &Ref,
) -> Result<&'a content::ProjectV2CreatureAuthoring, AttackError> {
    lower::native_callback_creature(native, target).map_err(|_| AttackError::InvalidSource)
}
