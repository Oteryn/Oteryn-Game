//! G-8 owner-only intervals. The statistics UUID is independent of admission identity.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Session {
    pub world_id: String,
    pub started_at: String,
    pub ended_at: Vec<String>,
}

/// Validate a complete interval replacement. An unknown end stays [], never extrapolated.
pub fn valid(sessions: &[Session], observed_at: i64) -> bool {
    observed_at >= 0
        && sessions.len() <= 1
        && sessions.iter().all(|session| {
            super::id(&session.world_id)
                && super::time(&session.started_at)
                && session.started_at.parse::<i64>().is_ok_and(|start| {
                    start <= observed_at
                        && session.ended_at.len() <= 1
                        && session.ended_at.iter().all(|end| {
                            super::time(end)
                                && end
                                    .parse::<i64>()
                                    .is_ok_and(|end| end >= start && end <= observed_at)
                        })
                })
        })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "typed synthetic interval fixture assertions"
)]
mod tests {
    use super::*;
    const WORLD: &str = "01934f10-7c02-7001-805b-3b1122334401";
    fn open() -> Session {
        Session {
            world_id: WORLD.into(),
            started_at: "1790000000".into(),
            ended_at: vec![],
        }
    }
    #[test]
    fn online_intervals_preserve_unknown_end_and_validate_exclusive_closure() {
        let session = open();
        assert!(valid(std::slice::from_ref(&session), 1790000000));
        assert!(session.ended_at.is_empty());
        let closed = Session {
            ended_at: vec!["1790003600".into()],
            ..session.clone()
        };
        assert!(valid(&[closed], 1790003600));
        let zero = Session {
            ended_at: vec![session.started_at.clone()],
            ..session
        };
        assert!(valid(&[zero], 1790000000));
        assert!(valid(&[], 1790000000));
    }
    #[test]
    fn online_intervals_refuse_future_negative_and_multiple_records_without_clamping() {
        let mut session = open();
        session.ended_at.push("1789999999".into());
        assert!(!valid(&[session.clone()], 1790003600));
        session.ended_at[0] = "1790003601".into();
        assert!(!valid(&[session.clone()], 1790003600));
        session.ended_at[0] = "01790003600".into();
        assert!(!valid(&[session.clone()], 1790003600));
        session.ended_at = vec!["1790003600".into(), "1790003600".into()];
        assert!(!valid(&[session], 1790003600));
        assert!(!valid(&[open(), open()], 1790003600));
        assert!(!valid(&[open()], 1789999999));
        assert!(!valid(&[open()], -1));
        assert!(serde_json::from_str::<Session>(r#"{"world_id":"01934f10-7c02-7001-805b-3b1122334401","started_at":"1790000000","ended_at":null}"#).is_err());
    }
    #[test]
    fn online_interval_wire_matches_generated_open_closed_and_withdrawn_goldens() {
        use super::super::{Body, Snapshot, encode};
        let open_wire = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../docs/contracts/fixtures/public-projections/character_online_session_open_v2.json"
        ));
        let closed_wire = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../docs/contracts/fixtures/public-projections/character_online_session_closed_v2.json"
        ));
        let withdrawn_wire = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../docs/contracts/fixtures/public-projections/character_online_session_withdrawn_v2.json"
        ));
        for (wire, revision, observed, sessions) in [
            (open_wire, 1, 1790000000, vec![open()]),
            (
                closed_wire,
                2,
                1790003600,
                vec![Session {
                    ended_at: vec!["1790003600".into()],
                    ..open()
                }],
            ),
            (withdrawn_wire, 3, 1790003700, vec![]),
        ] {
            let snapshot = Snapshot {
                subject: "01934f10-7c08-7001-805b-3b1122334401".into(),
                world_id: "01934f10-7c04-7001-805b-3b1122334401".into(),
                projection_epoch: 1,
                projection_revision: revision,
                source_observed_at: observed,
                body: Body::OnlineSessions(sessions),
            };
            assert_eq!(encode("oteryn:game:primary", &snapshot).unwrap(), wire);
        }
    }
    #[test]
    fn online_interval_negative_shared_fixtures_are_refused() {
        let invalid_null = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../docs/contracts/fixtures/public-projections/character_online_session_invalid_null_end_v2.json"
        ));
        let before = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../docs/contracts/fixtures/public-projections/character_online_session_invalid_end_before_start_v2.json"
        ));
        let future = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../docs/contracts/fixtures/public-projections/character_online_session_invalid_future_end_v2.json"
        ));
        let value: serde_json::Value = serde_json::from_str(invalid_null).unwrap();
        assert!(serde_json::from_value::<Vec<Session>>(value["sessions"].clone()).is_err());
        for wire in [before, future] {
            let value: serde_json::Value = serde_json::from_str(wire).unwrap();
            let records: Vec<Session> = serde_json::from_value(value["sessions"].clone()).unwrap();
            assert!(!valid(&records, 1790000000));
        }
    }
}
