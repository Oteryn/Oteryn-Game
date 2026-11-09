//! Immutable copied CreatureEvent registrations for the canonical deferred death phase.
//! Constructors remain inside Content; a descriptor grants no HP, Map or Item authority.
use super::native_gameplay::{CreatureProfilesDocument, NativeGameplayState};
use super::{
    ProjectV2AuthoringProfile, ProjectV2AuthoringProfileData, ProjectV2DefinitionRef,
    ProjectV2Family,
};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedNativeDeathEvent {
    registration_index: u16,
    name: String,
    source_path: String,
    source_sha256: [u8; 32],
}
impl QualifiedNativeDeathEvent {
    /// Called only after an independent source qualifier has proved the exact
    /// parent registration and loaded callback body. This is not an external API.
    pub(crate) fn from_qualified_source(
        registration_index: u16,
        name: String,
        source_path: String,
        source_sha256: [u8; 32],
    ) -> Self {
        Self {
            registration_index,
            name,
            source_path,
            source_sha256,
        }
    }
    pub fn registration_index(&self) -> u16 {
        self.registration_index
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn source_path(&self) -> &str {
        &self.source_path
    }
    pub fn source_sha256(&self) -> [u8; 32] {
        self.source_sha256
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedNativeDeathEvents {
    profile: Arc<ProjectV2AuthoringProfile>,
    content_digest: [u8; 32],
    events: Arc<[QualifiedNativeDeathEvent]>,
}
impl QualifiedNativeDeathEvents {
    /// W01 and Soul War use independent bounded source witnesses. Neither
    /// qualifier's success depends on the unrelated witness being valid.
    pub(crate) fn from_qualified_profile(
        profile: ProjectV2AuthoringProfile,
        content_digest: [u8; 32],
        events: Vec<QualifiedNativeDeathEvent>,
    ) -> Option<Self> {
        if profile.target.family != ProjectV2Family::Creature
            || !matches!(&profile.data, ProjectV2AuthoringProfileData::Creature(_))
            || events.len() > 32
            || events
                .windows(2)
                .any(|pair| pair[0].registration_index >= pair[1].registration_index)
            || events.iter().any(|event| {
                event.name.is_empty()
                    || event.name.len() > 128
                    || event.source_path.is_empty()
                    || event.source_path.len() > 512
                    || event.source_sha256 == [0; 32]
            })
        {
            return None;
        }
        Some(Self {
            profile: Arc::new(profile),
            content_digest,
            events: events.into(),
        })
    }
    pub fn creature(&self) -> &ProjectV2DefinitionRef {
        &self.profile.target
    }
    pub fn content_digest(&self) -> [u8; 32] {
        self.content_digest
    }
    pub fn matches(&self, digest: [u8; 32], key: &str, revision: &str) -> bool {
        self.content_digest == digest
            && self.profile.target.key == key
            && self.profile.target.revision == revision
    }
    pub(crate) fn bind_outer(&mut self, digest: [u8; 32]) {
        self.content_digest = digest;
    }
    /// Original successful registration order. Indices may have gaps because
    /// this copied vector contains onDeath callbacks, not other event kinds.
    pub fn events(&self) -> &[QualifiedNativeDeathEvent] {
        &self.events
    }
    pub fn matches_current(&self, native: &NativeGameplayState) -> bool {
        self.matches_document(native.creature_profiles(), native.source_digest())
    }
    pub(crate) fn matches_document(
        &self,
        document: &CreatureProfilesDocument,
        digest: [u8; 32],
    ) -> bool {
        if digest != self.content_digest {
            return false;
        }
        let mut found = document
            .records
            .iter()
            .filter(|record| record.profile.target == self.profile.target);
        found
            .next()
            .is_some_and(|record| record.profile == *self.profile)
            && found.next().is_none()
    }
}

/// Compile each independently source-qualified registry once at activation.
pub(crate) fn death_event_policies(
    document: &CreatureProfilesDocument,
    digest: [u8; 32],
) -> Vec<Arc<QualifiedNativeDeathEvents>> {
    let mut candidates = super::native_w01_death_events::death_event_policies(document, digest);
    candidates.extend(super::native_soul_war_death_events::death_event_policies(
        document, digest,
    ));
    candidates.sort_by(|a, b| {
        (&a.creature().key, &a.creature().revision)
            .cmp(&(&b.creature().key, &b.creature().revision))
    });
    let mut result = Vec::new();
    let mut index = 0;
    while index < candidates.len() {
        let first = &candidates[index];
        let mut end = index + 1;
        while end < candidates.len() && candidates[end].creature() == first.creature() {
            end += 1;
        }
        if candidates[index..end]
            .iter()
            .all(|other| other.as_ref() == first.as_ref())
        {
            result.push(Arc::clone(first));
        }
        index = end;
    }
    result
}
/// A broken source witness never silently turns a reserved type ordinary.
pub fn requires_native_death_events(key: &str) -> bool {
    super::native_w01_death_events::requires_native_death_events(key)
        || super::native_soul_war_death_events::requires_native_death_events(key)
}
impl NativeGameplayState {
    pub fn native_death_events(
        &self,
        target: &ProjectV2DefinitionRef,
    ) -> Option<Arc<QualifiedNativeDeathEvents>> {
        if target.family != ProjectV2Family::Creature {
            return None;
        }
        self.policies()
            .native_death_events(&target.key, &target.revision)
    }
}
