//! Attached Gaz source events; immutable evidence, never an actor/map grant.
use super::*;

#[derive(Debug, Clone)]
pub struct QualifiedGazEvents {
    parent: Ref,
    minion: Ref,
    content: [u8; 32],
}
impl QualifiedGazEvents {
    pub fn parent(&self) -> &Ref {
        &self.parent
    }
    pub fn minion(&self) -> &Ref {
        &self.minion
    }
    pub fn content(&self) -> [u8; 32] {
        self.content
    }
}
impl NativeGameplayState {
    pub fn gaz_events_source(&self) -> Result<QualifiedGazEvents, NativeCallbackError> {
        let source = witness()?;
        let required = [
            (
                "GazHaragothHeal",
                "data-otservbr-global/scripts/creaturescripts/monster/gaz_haragoth_heal.lua",
                "ab715b9d27d98511d47707103860383766b579d71de567a6455aa0deabb3a12c",
            ),
            (
                "MinionGazDeath",
                "data-otservbr-global/scripts/creaturescripts/monster/minion_gaz_haragoth_vortex.lua",
                "140e7db53054ac626d49e0b411395fe7b5e0a3d4bdb951de42be0853c77d6159",
            ),
            (
                "GazNightmareVortexStep",
                "data-otservbr-global/scripts/movements/roshamuul/strange_vortex_tp.lua",
                "1e80473ce94e535c4aa56a5f784df35221fa354128b13406dd18d51e88e2c25e",
            ),
        ];
        for (name, path, sha) in required {
            let mut found = source
                .source_events
                .iter()
                .filter(|event| event.name == name);
            let event = found.next().ok_or(NativeCallbackError::InvalidSource)?;
            if found.next().is_some()
                || event.path != path
                || event.sha256 != sha
                || event.repository != "opentibiabr/canary"
                || event.revision != "47dfd51f45280a59a1d3e50ba7edd573d7234446"
            {
                return Err(NativeCallbackError::InvalidSource);
            }
        }
        let mut references = Vec::new();
        for (name, external) in [
            ("gaz_haragoth", "quests/roshamuul/gaz'haragoth"),
            (
                "minion_of_gaz_haragoth",
                "quests/roshamuul/minion_of_gaz'haragoth",
            ),
        ] {
            let key = format!("oteryn:creature.{name}");
            let mut records = self
                .creature_profiles()
                .records
                .iter()
                .filter(|record| record.profile.target.key == key);
            let record = records.next().ok_or(NativeCallbackError::InvalidSource)?;
            if records.next().is_some() {
                return Err(NativeCallbackError::InvalidSource);
            }
            native_callback_creature(self, &record.profile.target)?;
            if name == "gaz_haragoth" {
                let behavior = record
                    .behavior
                    .as_ref()
                    .ok_or(NativeCallbackError::InvalidSource)?;
                if source
                    .profiles
                    .iter()
                    .filter(|row| row.target == behavior.target)
                    .count()
                    != 1
                    || !source.profiles.iter().any(|row| row == behavior)
                {
                    return Err(NativeCallbackError::InvalidSource);
                }
            }
            let bindings = source
                .source_identity_bindings
                .iter()
                .filter(|binding| binding.target == record.profile.target)
                .collect::<Vec<_>>();
            if bindings.len() != 1
                || bindings[0].source_key != "oteryn:source.canary"
                || bindings[0].source_revision != "47dfd51f45280a59a1d3e50ba7edd573d7234446"
                || bindings[0].identity_namespace != "canary/monster-file"
                || bindings[0].external_id != external
                || bindings[0].disposition
                    != super::super::ProjectV2SourceIdentityDisposition::Exact
            {
                return Err(NativeCallbackError::InvalidSource);
            }
            references.push(record.profile.target.clone());
        }
        Ok(QualifiedGazEvents {
            parent: references.remove(0),
            minion: references.remove(0),
            content: self.source_digest(),
        })
    }
}
