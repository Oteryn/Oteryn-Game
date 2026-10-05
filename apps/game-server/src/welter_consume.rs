//! Dedicated donor defenses[4] sourcecase; never aliases ordinary native defense-2.
//! Called in an owning AI think work item. Production timer/transport composition is root-owned.
use crate::content::{
    ProjectReferenceRecord, ProjectV2AuthoringProfile, ProjectV2AuthoringProfileData as Data,
    ProjectV2DefinitionRef as Ref, ProjectV2Family, ProjectV2SourceIdentityBinding,
    ProjectV2SourceIdentityDisposition,
};
use crate::foundation::owner_timer::SemanticTimeMicros;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, WelterConsumeError, WelterConsumeLedger, WelterConsumeResult,
    WelterSourceRegistration,
};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
const SOURCE_REVISION: &str = "47dfd51f45280a59a1d3e50ba7edd573d7234446";
const INTERVAL_MICROS: u64 = 2_000_000;
const CHANCE_PPM: u64 = 80_000;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WelterSourceError {
    InvalidSource,
    ContentChanged,
    StaleCaster,
    InvalidSequence,
    OccurrenceConflict,
    OccurrenceSuperseded,
    ClockOverflow,
    ClockRegression,
    Native(WelterConsumeError),
}
impl From<WelterConsumeError> for WelterSourceError {
    fn from(e: WelterConsumeError) -> Self {
        Self::Native(e)
    }
}
/// Sealed registration obtained from three canonical typed profiles + exact pinned bindings.
/// Like MeleeSource, arrays must come from the current native artifact's owning loader.
/// Numeric/key/name agreement without exact donor source bindings is rejected.
#[derive(Clone)]
pub(crate) struct WelterSource {
    registration: WelterSourceRegistration,
    content_digest: [u8; 32],
    fingerprint: [u8; 32],
}
impl WelterSource {
    pub(crate) fn from_native(
        runtime: &ChannelRuntimeV1,
        records: &[ProjectReferenceRecord],
        profiles: &[ProjectV2AuthoringProfile],
        bindings: &[ProjectV2SourceIdentityBinding],
        content_digest: [u8; 32],
    ) -> Result<Self, WelterSourceError> {
        if runtime.content_pin().server_artifact_digest() != content_digest {
            return Err(WelterSourceError::ContentChanged);
        }
        let mut map = BTreeMap::new();
        for p in profiles {
            if map.insert(p.target.clone(), &p.data).is_some() {
                return Err(WelterSourceError::InvalidSource);
            }
        }
        let mut chosen = Vec::new();
        let mut source_bindings = Vec::new();
        for (slug, name, file) in [
            ("the_welter", "The Welter", "raids/the_welter"),
            ("egg", "Egg", "raids/egg_the_welter"),
            (
                "spawn_of_the_welter",
                "Spawn of the Welter",
                "raids/spawn_of_the_welter",
            ),
        ] {
            let target = Ref {
                family: ProjectV2Family::Creature,
                key: format!("oteryn:creature.{slug}"),
                revision: "definition-r1".to_owned(),
            };
            let record=records.iter().filter(|r|matches!(r,ProjectReferenceRecord::Creature{identity,..} if identity.family=="Creature" && identity.key==target.key && identity.revision==target.revision)).collect::<Vec<_>>();
            if record.len() != 1 {
                return Err(WelterSourceError::InvalidSource);
            }
            let Some(Data::Creature(c)) = map.get(&target).copied() else {
                return Err(WelterSourceError::InvalidSource);
            };
            if c.details.as_ref().map(|d| d.display_name.as_str()) != Some(name)
                || c.health.is_none_or(|h| h == 0)
            {
                return Err(WelterSourceError::InvalidSource);
            }
            if slug == "the_welter" && c.health != Some(25000) {
                return Err(WelterSourceError::InvalidSource);
            }
            let bound = bindings
                .iter()
                .filter(|b| {
                    b.target == target
                        && b.source_key == "oteryn:source.canary"
                        && b.identity_namespace == "canary/monster-file"
                })
                .collect::<Vec<_>>();
            if bound.len() != 1
                || bound[0].source_revision != SOURCE_REVISION
                || bound[0].external_id != file
                || bound[0].disposition != ProjectV2SourceIdentityDisposition::Exact
            {
                return Err(WelterSourceError::InvalidSource);
            }
            source_bindings.push(bound[0]);
            chosen.push((target, c, record[0]));
        }
        let boss_health = chosen[0].1.health.ok_or(WelterSourceError::InvalidSource)?;
        let registration = runtime.bind_welter_pinned_definition(
            &chosen[0].0.key,
            "The Welter",
            boss_health,
            &chosen[1].0.key,
            "Egg",
            &chosen[2].0.key,
            "Spawn of the Welter",
        )?;
        let fingerprint = Sha256::digest(
            serde_json::to_vec(&(
                chosen,
                source_bindings,
                content_digest,
                "the_welter.defenses[4]",
                INTERVAL_MICROS,
                CHANCE_PPM,
            ))
            .map_err(|_| WelterSourceError::InvalidSource)?,
        )
        .into();
        Ok(Self {
            registration,
            content_digest,
            fingerprint,
        })
    }
    pub(crate) fn registration(&self) -> &WelterSourceRegistration {
        &self.registration
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WelterThinkResult {
    pub(crate) newly_processed: bool,
    pub(crate) due: bool,
    pub(crate) chance_passed: bool,
    pub(crate) commit: Option<WelterConsumeResult>,
    pub(crate) sequence: u64,
}
struct Last {
    sequence: u64,
    now: SemanticTimeMicros,
    root: GameplayDecisionRoot,
    result: WelterThinkResult,
}
/// One driver beside the caster's existing ephemeral ProfileScheduleState, same owner lock.
/// Per-caster state; native D115 SKIP_TO_LATEST timing, no backlog of missed spell casts.
pub(crate) struct WelterDefenceDriver {
    caster: ExactActorRef,
    source: [u8; 32],
    next_due: SemanticTimeMicros,
    last: Option<Last>,
}
impl WelterDefenceDriver {
    pub(crate) fn attach(
        runtime: &ChannelRuntimeV1,
        source: &WelterSource,
        caster: ExactActorRef,
        now: SemanticTimeMicros,
    ) -> Result<Self, WelterSourceError> {
        validate(runtime, source, caster)?;
        let ready = now
            .get()
            .checked_add(INTERVAL_MICROS)
            .ok_or(WelterSourceError::ClockOverflow)?;
        Ok(Self {
            caster,
            source: source.fingerprint,
            next_due: SemanticTimeMicros::from_micros(ready),
            last: None,
        })
    }
    pub(crate) fn on_think(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        ledger: &mut WelterConsumeLedger,
        source: &WelterSource,
        sequence: u64,
        now: SemanticTimeMicros,
        root: &GameplayDecisionRoot,
    ) -> Result<WelterThinkResult, WelterSourceError> {
        validate(runtime, source, self.caster)?;
        if source.fingerprint != self.source {
            return Err(WelterSourceError::ContentChanged);
        }
        if sequence == 0 {
            return Err(WelterSourceError::InvalidSequence);
        }
        if let Some(last) = &self.last {
            if now < last.now {
                return Err(WelterSourceError::ClockRegression);
            }
            if sequence < last.sequence {
                return Err(WelterSourceError::OccurrenceSuperseded);
            }
            if sequence == last.sequence {
                if now != last.now || *root != last.root {
                    return Err(WelterSourceError::OccurrenceConflict);
                }
                return Ok(WelterThinkResult {
                    newly_processed: false,
                    commit: last.result.commit.map(|r| WelterConsumeResult {
                        newly_committed: false,
                        ..r
                    }),
                    ..last.result
                });
            }
        }
        let due = now >= self.next_due;
        let next = if due {
            SemanticTimeMicros::from_micros(
                now.get()
                    .checked_add(INTERVAL_MICROS)
                    .ok_or(WelterSourceError::ClockOverflow)?,
            )
        } else {
            self.next_due
        };
        let passed = due && chance(self.caster, sequence, root)?;
        let commit = if passed {
            Some(runtime.commit_welter_consume(
                ledger,
                &source.registration,
                self.caster,
                sequence,
            )?)
        } else {
            None
        };
        let result = WelterThinkResult {
            newly_processed: true,
            due,
            chance_passed: passed,
            commit,
            sequence,
        };
        self.next_due = next;
        self.last = Some(Last {
            sequence,
            now,
            root: root.clone(),
            result,
        });
        Ok(result)
    }
}
fn validate(
    runtime: &ChannelRuntimeV1,
    source: &WelterSource,
    caster: ExactActorRef,
) -> Result<(), WelterSourceError> {
    if runtime.content_pin().server_artifact_digest() != source.content_digest {
        return Err(WelterSourceError::ContentChanged);
    }
    if !runtime.matches_live_creature_identity(caster, b"oteryn:creature.the_welter") {
        return Err(WelterSourceError::StaleCaster);
    }
    Ok(())
}
fn chance(
    caster: ExactActorRef,
    sequence: u64,
    root: &GameplayDecisionRoot,
) -> Result<bool, WelterSourceError> {
    let d = Sha256::new()
        .chain_update(b"oteryn:welter-consume:defenses-4:v1")
        .chain_update(caster.placement_identity())
        .chain_update(sequence.to_be_bytes())
        .finalize();
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&d[..16]);
    let draw = deterministic_decision_u64(
        root,
        DecisionOccurrenceId::from_bytes(bytes),
        "AI_DEFENCE",
        3,
    )
    .map_err(|_| WelterSourceError::InvalidSource)?;
    Ok(draw % 1_000_000 < CHANCE_PPM)
}
#[cfg(test)]
mod native_source_tests {
    use super::*;
    // Qualification only: compile within the actual carrier module to admit fixture actors.
    #[cfg(test)]
    #[test]
    fn welter_source_producer_and_chat_native_owner_proof() {
        use crate::content::{
            ProjectReferenceRecord, ProjectV2AuthoringProfile, ProjectV2SourceIdentityBinding,
        };
        use crate::foundation::owner_timer::SemanticTimeMicros;
        use crate::foundation::{
            ChannelContentPin, ChannelId, GameSessionId, MovementLocalPosition, NodeId, WorldId,
        };
        use crate::welter_consume::{WelterDefenceDriver, WelterSource, WelterSourceError};
        use oteryn_simulation_determinism::GameplayDecisionRoot;
        fn time(v: u64) -> SemanticTimeMicros {
            SemanticTimeMicros::from_micros(v)
        }
        fn id(tag: u8) -> [u8; 16] {
            let mut b = [0; 16];
            b[6] = 0x70;
            b[8] = 0x80;
            b[15] = tag;
            b
        }
        let world = WorldId::decode(&id(1)).unwrap();
        let mut r = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).unwrap(),
            NodeId::decode(&id(3)).unwrap(),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            16,
            ChannelContentPin::test(world),
        )
        .unwrap();
        fn actor(r: &mut ChannelRuntimeV1, name: &str, hp: i64, x: i32, y: i32) -> ExactActorRef {
            r.admit_source_pinned_lab_creature(MovementLocalPosition { x, y, floor: 7 }, name, hp)
                .unwrap()
        }
        let caster = actor(&mut r, "oteryn:creature.the_welter", 25000, 100, 100);
        r.commit_monster_lab_damage(caster, b"damage1", 20000)
            .unwrap();
        let egg = actor(&mut r, "oteryn:creature.egg", 800, 101, 100);
        let profiles:Vec<ProjectV2AuthoringProfile>=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../docs/agents/evidence/monster-full-mechanics-20261004/lanes/encounters/welter-native-profiles.json"))).unwrap();
        let v:serde_json::Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../docs/agents/evidence/monster-full-mechanics-20261004/lanes/encounters/welter-native-records-bindings.json"))).unwrap();
        let records: Vec<ProjectReferenceRecord> =
            serde_json::from_value(v["records"].clone()).unwrap();
        let bindings: Vec<ProjectV2SourceIdentityBinding> =
            serde_json::from_value(v["bindings"].clone()).unwrap();
        let digest = r.content_pin().server_artifact_digest();
        let source = WelterSource::from_native(&r, &records, &profiles, &bindings, digest).unwrap();
        let mut fake = bindings.clone();
        fake[0].source_revision = "fake".to_owned();
        assert!(matches!(
            WelterSource::from_native(&r, &records, &profiles, &fake, digest),
            Err(WelterSourceError::InvalidSource)
        ));
        fake = bindings.clone();
        fake[0].external_id = "raids/other".to_owned();
        assert!(matches!(
            WelterSource::from_native(&r, &records, &profiles, &fake, digest),
            Err(WelterSourceError::InvalidSource)
        ));
        assert!(matches!(
            WelterSource::from_native(&r, &records, &profiles, &[], digest),
            Err(WelterSourceError::InvalidSource)
        ));
        assert!(matches!(
            WelterSource::from_native(&r, &records, &profiles, &bindings, [9; 32]),
            Err(WelterSourceError::ContentChanged)
        ));
        let mut dup = profiles.clone();
        dup.push(profiles[0].clone());
        assert!(matches!(
            WelterSource::from_native(&r, &records, &dup, &bindings, digest),
            Err(WelterSourceError::InvalidSource)
        ));
        let mut fakeprofile = profiles.clone();
        if let crate::content::ProjectV2AuthoringProfileData::Creature(c) = &mut fakeprofile[0].data
        {
            c.health = Some(500)
        }
        assert!(matches!(
            WelterSource::from_native(&r, &records, &fakeprofile, &bindings, digest),
            Err(WelterSourceError::InvalidSource)
        ));
        let mut driver = WelterDefenceDriver::attach(&r, &source, caster, time(0)).unwrap();
        let mut ledger = WelterConsumeLedger::default();
        let root = GameplayDecisionRoot::from_bytes([1; 32]);
        let early = driver
            .on_think(&mut r, &mut ledger, &source, 1, time(1_000_000), &root)
            .unwrap();
        assert!(!early.due);
        assert!(r.contains_live_creature(egg));
        // Find a deterministic 8%-passing root; production still uses the real owner root.
        let (passing, mut next, mut successful_ledger) = (0u64..10000)
            .find_map(|v| {
                let mut b = [0; 32];
                b[..8].copy_from_slice(&v.to_be_bytes());
                let root = GameplayDecisionRoot::from_bytes(b);
                let mut d = WelterDefenceDriver::attach(&r, &source, caster, time(0)).unwrap();
                let mut l = WelterConsumeLedger::default();
                let x = d
                    .on_think(&mut r, &mut l, &source, 2, time(2_000_000), &root)
                    .unwrap();
                if x.chance_passed {
                    Some((root, d, l))
                } else {
                    None
                }
            })
            .unwrap();
        // The successful real owner trial consumed the egg; prove source receipt and cap.
        assert!(!r.contains_live_creature(egg));
        assert_eq!(
            r.welter_committed_consume(&successful_ledger, source.registration(), caster, 2)
                .unwrap()
                .unwrap()
                .1
                .health_after,
            25000
        );
        // A fresh clone's retry suppresses the already-resolved successful occurrence.
        let repeated = next
            .on_think(
                &mut r,
                &mut successful_ledger,
                &source,
                2,
                time(2_000_000),
                &passing,
            )
            .unwrap();
        assert!(!repeated.newly_processed);
        assert!(!repeated.commit.unwrap().newly_committed);
        let conflict = next.on_think(
            &mut r,
            &mut successful_ledger,
            &source,
            2,
            time(2_000_001),
            &passing,
        );
        assert_eq!(conflict, Err(WelterSourceError::OccurrenceConflict));
        let late = next
            .on_think(
                &mut r,
                &mut successful_ledger,
                &source,
                3,
                time(3_000_000),
                &passing,
            )
            .unwrap();
        assert!(!late.due);
        assert!(
            next.on_think(
                &mut r,
                &mut successful_ledger,
                &source,
                1,
                time(3_000_000),
                &passing
            )
            .is_err()
        );
        let session = GameSessionId::decode(&id(8)).unwrap();
        let reservation = r.reserve_fresh_session(session).unwrap();
        let player = r.commit_fresh_session(reservation).unwrap();
        r.initialize_movement_test_position(
            player,
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
        )
        .unwrap();
        let mut mailbox = crate::weak_spot_speech::WeakSpotSpeechMailbox::default();
        let delivered = mailbox
            .publish_welter_consume(
                &r,
                &successful_ledger,
                source.registration(),
                caster,
                2,
                &[(player, session)],
            )
            .unwrap()
            .unwrap();
        assert_eq!(delivered.0, 1);
        assert_eq!(
            delivered.1[0].asset_binding,
            "canary.appearance:effect/hitbypoison"
        );
        assert_eq!(
            delivered.1[0].at,
            MovementLocalPosition {
                x: 101,
                y: 100,
                floor: 7
            }
        );
        assert_eq!(
            delivered.1[1].asset_binding,
            "canary.appearance:effect/magic_blue"
        );
        assert!(
            mailbox
                .publish_welter_consume(
                    &r,
                    &successful_ledger,
                    source.registration(),
                    caster,
                    2,
                    &[(player, session)]
                )
                .unwrap()
                .is_none()
        );
        let frames = mailbox.drain(&r, player, session).unwrap();
        assert_eq!(frames.len(), 1);
        let line = oteryn_protocol_oteryn::chat::decode_chat_line(&frames[0]).unwrap();
        assert!(
            matches!(line,oteryn_protocol_oteryn::chat::ChatLine::Local{speaker_name,text,..} if speaker_name=="The Welter"&&text=="<the welter devours his spawn and heals himself>")
        );
        assert!(
            mailbox
                .publish_welter_consume(
                    &r,
                    &successful_ledger,
                    source.registration(),
                    caster,
                    999,
                    &[(player, session)]
                )
                .unwrap()
                .is_none()
        );
        println!(
            "PASS real source-registry negative guards + 2s deterministic8% producer + actual owner consume/heal + replay/clock"
        );
    }
}
