//! MAP-OVERLAY-1c (D1451): the operator's `world-reset` command of `oteryn-game-ops`.
//!
//! `initialize` records epoch 0 of a World through the explicit Control initializer;
//! `publish` is the operator path of the only RETIRING publisher (`publish_world_reset`,
//! D1272). Both take their bundle identity from qualified content, never from the command
//! line: the map revision is the qualified House catalogue's and the digests are the
//! qualified room's. The node completes a published reset at its next boot
//! (`recover_world_reset`), which mints the source map again under epoch N+1.
//!
//! Every input of a publication is deterministic, its retirement binding included, so an
//! identical re-run after an unknown outcome is the replay: it reports the committed
//! publication instead of a second job.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use super::DurabilityError;
use super::DurabilityRoot;
use super::world_reset::{
    ResetCalendar, ResetCalendarPolicy, WorldResetArtifactDigests, WorldResetError,
    WorldResetTarget,
};
use super::world_reset_port::{CatalogueSide, WorldResetHouseSource};
use crate::foundation::WorldId;

pub const WORLD_RESET_USAGE: &str = "world-reset initialize --world <uuid> \
--calendar-revision <n> | world-reset publish --world <uuid> --epoch <n> --calendar-revision <n>: \
`initialize` records reset epoch 0 of the World from the active bundle \
(`OTERYN_NATIVE_GAMEPLAY_MANIFEST`, as for `content activate`) with the operator credential; \
`publish` closes admission and publishes the reset of the active epoch to the staged target \
bundle (`OTERYN_WORLD_RESET_TARGET_MANIFEST`, or the active bundle when none is staged) with \
the [world_reset] reset owner credential of the ops config. The node completes a published \
reset at its next boot. An identical re-run after an unknown outcome is the replay.";

/// One parsed `world-reset` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldResetCommand {
    Initialize {
        world: WorldId,
        calendar: ResetCalendar,
    },
    Publish {
        world: WorldId,
        reset_epoch: u64,
        calendar: ResetCalendar,
    },
}

/// The failure class of a `world-reset` command; the ops tool maps it to its exit status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldResetCommandFailure {
    Usage(&'static str),
    Input(String),
    Rejected(String),
    Unavailable(String),
    Ambiguous(String),
}

impl std::fmt::Display for WorldResetCommandFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(detail) => formatter.write_str(detail),
            Self::Input(detail)
            | Self::Rejected(detail)
            | Self::Unavailable(detail)
            | Self::Ambiguous(detail) => formatter.write_str(detail),
        }
    }
}

fn world_of(text: &str) -> Result<WorldId, WorldResetCommandFailure> {
    let invalid = || WorldResetCommandFailure::Input("world".into());
    let bytes = text.as_bytes();
    if bytes.len() != 36 || [8, 13, 18, 23].iter().any(|index| bytes[*index] != b'-') {
        return Err(invalid());
    }
    let digits: Vec<u8> = bytes.iter().copied().filter(|byte| *byte != b'-').collect();
    if digits.len() != 32 || !digits.iter().all(u8::is_ascii_hexdigit) {
        return Err(invalid());
    }
    let mut id = [0_u8; 16];
    for (slot, pair) in id.iter_mut().zip(digits.chunks_exact(2)) {
        let pair = std::str::from_utf8(pair).map_err(|_| invalid())?;
        *slot = u8::from_str_radix(pair, 16).map_err(|_| invalid())?;
    }
    WorldId::decode(&id).map_err(|_| invalid())
}

fn number_of(label: &'static str, text: &str) -> Result<u64, WorldResetCommandFailure> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(WorldResetCommandFailure::Input(label.into()));
    }
    text.parse()
        .map_err(|_| WorldResetCommandFailure::Input(label.into()))
}

impl WorldResetCommand {
    /// Parses `world-reset initialize|publish` from the command words and options. Every
    /// option is consumed; an unknown or missing one refuses the command.
    pub fn parse(
        words: &[String],
        options: &mut BTreeMap<String, String>,
    ) -> Result<Self, WorldResetCommandFailure> {
        let usage = || WorldResetCommandFailure::Usage(WORLD_RESET_USAGE);
        if words.len() != 2 || words[0] != "world-reset" {
            return Err(usage());
        }
        let mut take = |name: &str| options.remove(name).ok_or_else(usage);
        let command = match words[1].as_str() {
            "initialize" => {
                let world = take("world")?;
                let revision = take("calendar-revision")?;
                Self::Initialize {
                    world: world_of(&world)?,
                    calendar: calendar_of(&revision)?,
                }
            }
            "publish" => {
                let world = take("world")?;
                let epoch = take("epoch")?;
                let revision = take("calendar-revision")?;
                Self::Publish {
                    world: world_of(&world)?,
                    reset_epoch: number_of("epoch", &epoch)?,
                    calendar: calendar_of(&revision)?,
                }
            }
            _ => return Err(usage()),
        };
        if !options.is_empty() {
            return Err(usage());
        }
        Ok(command)
    }

    /// The World whose content the command qualifies.
    pub const fn world(&self) -> WorldId {
        match self {
            Self::Initialize { world, .. } | Self::Publish { world, .. } => *world,
        }
    }

    /// Whether the command runs as the reset owner's login (`publish`), not the operator's.
    pub const fn publishes(&self) -> bool {
        matches!(self, Self::Publish { .. })
    }
}

fn calendar_of(text: &str) -> Result<ResetCalendar, WorldResetCommandFailure> {
    let revision = number_of("calendar-revision", text)?;
    if revision == 0 {
        return Err(WorldResetCommandFailure::Input("calendar-revision".into()));
    }
    Ok(ResetCalendar {
        policy: ResetCalendarPolicy::TibiaServerSaveBerlin10,
        revision,
    })
}

/// The retirement binding of a publication: a domain-separated digest of its whole target,
/// so an identical command always publishes the identical cause.
pub(crate) fn retirement_binding(target: &WorldResetTarget) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(b"oteryn-world-reset-publication-v1\0");
    digest.update(target.world.as_bytes());
    digest.update(target.reset_epoch.to_be_bytes());
    let length = |bytes: &[u8]| u32::try_from(bytes.len()).unwrap_or(u32::MAX).to_be_bytes();
    digest.update(length(target.map_revision.as_bytes()));
    digest.update(target.map_revision.as_bytes());
    digest.update(target.digests.server);
    digest.update(target.digests.client);
    digest.update(target.digests.frame);
    digest.update(length(target.calendar.policy.as_str().as_bytes()));
    digest.update(target.calendar.policy.as_str().as_bytes());
    digest.update(target.calendar.revision.to_be_bytes());
    digest.finalize().into()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn failure(error: WorldResetError, retry: &'static str) -> WorldResetCommandFailure {
    match error {
        WorldResetError::Unavailable(DurabilityError::CommitOutcomeUnknown) => {
            WorldResetCommandFailure::Ambiguous(format!("world reset outcome unknown; {retry}"))
        }
        WorldResetError::Unavailable(error) => {
            WorldResetCommandFailure::Unavailable(format!("world reset unavailable ({error:?})"))
        }
        WorldResetError::InvalidInitialization | WorldResetError::UnqualifiedTarget => {
            WorldResetCommandFailure::Input(format!("world reset bundle refused ({error:?})"))
        }
        error => WorldResetCommandFailure::Rejected(format!("world reset refused ({error:?})")),
    }
}

impl DurabilityRoot {
    /// Whether exactly this publication is the World's RETIRING reset. Read-only.
    async fn world_reset_published(
        &self,
        target: &WorldResetTarget,
        binding: [u8; 32],
    ) -> Result<bool, WorldResetError> {
        let target = target.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                    let published: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_reset_jobs WHERE world_id=encode($1,'hex')::uuid AND phase='RETIRING' AND reset_epoch=$2::text::numeric AND target_digest=$3 AND target_map_revision=$4 AND target_server_digest=$5 AND target_client_digest=$6 AND target_frame_digest=$7 AND target_calendar_policy=$8 AND target_calendar_revision=$9::text::numeric)")
                        .bind(target.world.as_bytes().as_slice()).bind(target.reset_epoch.to_string())
                        .bind(binding.as_slice()).bind(&target.map_revision)
                        .bind(target.digests.server.as_slice()).bind(target.digests.client.as_slice()).bind(target.digests.frame.as_slice())
                        .bind(target.calendar.policy.as_str()).bind(target.calendar.revision.to_string())
                        .fetch_one(&mut *tx).await?;
                    super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(published)
                })
            })
            .await
            .map_err(WorldResetError::from)
    }
}

/// Runs one parsed command. `active` and `target` are the qualified rooms' artifact digests;
/// `houses` supplies both qualified House catalogues, whose map revisions name the bundles.
/// Returns the operator event line.
pub(crate) async fn execute_world_reset_command(
    root: &DurabilityRoot,
    command: &WorldResetCommand,
    active: WorldResetArtifactDigests,
    target: WorldResetArtifactDigests,
    houses: &(dyn WorldResetHouseSource + Sync),
) -> Result<String, WorldResetCommandFailure> {
    let catalogue = |side, label: &'static str| {
        houses
            .reset_houses(command.world(), side)
            .map_err(|_| WorldResetCommandFailure::Input(format!("{label} House catalogue")))
    };
    match command {
        WorldResetCommand::Initialize { world, calendar } => {
            let map_revision = catalogue(CatalogueSide::Active, "active")?.map_revision;
            match root
                .initialize_world_reset_epoch(*world, &map_revision, active, *calendar)
                .await
            {
                Ok(true) => Ok(format!(
                    "world_reset=initialized reset_epoch=0 map_revision={map_revision} server_digest={} client_digest={} frame_binding={} calendar_revision={}",
                    hex(&active.server),
                    hex(&active.client),
                    hex(&active.frame),
                    calendar.revision
                )),
                Ok(false) => Err(WorldResetCommandFailure::Rejected(
                    "world reset initialization is not granted or conflicts with the recorded epoch history".into(),
                )),
                Err(error) => Err(failure(error, "re-run the same command")),
            }
        }
        WorldResetCommand::Publish {
            world,
            reset_epoch,
            calendar,
        } => {
            let publication = WorldResetTarget {
                world: *world,
                reset_epoch: *reset_epoch,
                map_revision: catalogue(CatalogueSide::Target, "target")?.map_revision,
                digests: target,
                calendar: *calendar,
            };
            let binding = retirement_binding(&publication);
            let replayed = match root
                .publish_world_reset(&publication, binding, houses)
                .await
            {
                Ok(()) => false,
                Err(WorldResetError::ConflictingCause) => {
                    if !root
                        .world_reset_published(&publication, binding)
                        .await
                        .map_err(|error| failure(error, "re-run the same command"))?
                    {
                        return Err(WorldResetCommandFailure::Rejected(
                            "another World reset is already published for this World or epoch"
                                .into(),
                        ));
                    }
                    true
                }
                Err(error) => return Err(failure(error, "re-run the same command")),
            };
            Ok(format!(
                "world_reset=published phase=RETIRING reset_epoch={reset_epoch} map_revision={} server_digest={} client_digest={} frame_binding={} calendar_revision={} retirement_binding={} replayed={replayed}",
                publication.map_revision,
                hex(&target.server),
                hex(&target.client),
                hex(&target.frame),
                calendar.revision,
                hex(&binding)
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    const WORLD: &str = "019a0000-0000-7000-8000-000000000001";

    fn parse(
        words: &[&str],
        options: &[(&str, &str)],
    ) -> Result<WorldResetCommand, WorldResetCommandFailure> {
        let words: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
        let mut options = options
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect();
        WorldResetCommand::parse(&words, &mut options)
    }

    #[test]
    fn world_reset_commands_parse_exactly_their_options() {
        let world = world_of(WORLD).unwrap();
        let calendar = ResetCalendar {
            policy: ResetCalendarPolicy::TibiaServerSaveBerlin10,
            revision: 1,
        };
        let publish = parse(
            &["world-reset", "publish"],
            &[("world", WORLD), ("epoch", "0"), ("calendar-revision", "1")],
        )
        .unwrap();
        assert_eq!(
            publish,
            WorldResetCommand::Publish {
                world,
                reset_epoch: 0,
                calendar
            }
        );
        assert!(publish.publishes());
        let initialize = parse(
            &["world-reset", "initialize"],
            &[("world", WORLD), ("calendar-revision", "1")],
        )
        .unwrap();
        assert_eq!(
            initialize,
            WorldResetCommand::Initialize { world, calendar }
        );
        assert!(!initialize.publishes());
        let usage = Err(WorldResetCommandFailure::Usage(WORLD_RESET_USAGE));
        for (words, options) in [
            (
                &["world-reset", "publish"][..],
                &[("world", WORLD), ("calendar-revision", "1")][..],
            ),
            (
                &["world-reset", "initialize"][..],
                &[("world", WORLD), ("calendar-revision", "1"), ("epoch", "0")][..],
            ),
            (
                &["world-reset"][..],
                &[("world", WORLD), ("calendar-revision", "1")][..],
            ),
            (
                &["world-reset", "activate"][..],
                &[("world", WORLD), ("calendar-revision", "1")][..],
            ),
            (
                &["world-reset", "initialize", "now"][..],
                &[("world", WORLD), ("calendar-revision", "1")][..],
            ),
        ] {
            assert_eq!(parse(words, options), usage);
        }
        for (world, epoch, revision) in [
            ("019a0000-0000-7000-8000-00000000000g", "0", "1"),
            ("019a0000000070008000000000000001", "0", "1"),
            (WORLD, "-1", "1"),
            (WORLD, "+0", "1"),
            (WORLD, "18446744073709551616", "1"),
            (WORLD, "0", "0"),
        ] {
            let refused = parse(
                &["world-reset", "publish"],
                &[
                    ("world", world),
                    ("epoch", epoch),
                    ("calendar-revision", revision),
                ],
            );
            assert!(
                matches!(refused, Err(WorldResetCommandFailure::Input(_))),
                "{refused:?}"
            );
        }
    }

    #[test]
    fn the_retirement_binding_is_the_whole_target() {
        let target = WorldResetTarget {
            world: world_of(WORLD).unwrap(),
            reset_epoch: 0,
            map_revision: "map-2".into(),
            digests: WorldResetArtifactDigests {
                server: [4; 32],
                client: [5; 32],
                frame: [6; 32],
            },
            calendar: ResetCalendar {
                policy: ResetCalendarPolicy::TibiaServerSaveBerlin10,
                revision: 1,
            },
        };
        let binding = retirement_binding(&target);
        assert_eq!(binding, retirement_binding(&target.clone()));
        let variants = [
            WorldResetTarget {
                reset_epoch: 1,
                ..target.clone()
            },
            WorldResetTarget {
                map_revision: "map-3".into(),
                ..target.clone()
            },
            WorldResetTarget {
                digests: WorldResetArtifactDigests {
                    frame: [7; 32],
                    ..target.digests
                },
                ..target.clone()
            },
            WorldResetTarget {
                calendar: ResetCalendar {
                    revision: 2,
                    ..target.calendar
                },
                ..target.clone()
            },
        ];
        for variant in variants {
            assert_ne!(retirement_binding(&variant), binding);
        }
    }
}
