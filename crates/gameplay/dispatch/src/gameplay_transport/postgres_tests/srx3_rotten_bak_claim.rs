//! A source global phase is decided at FIRST think, never on room entry.
use super::{TestResult, fixture::Harness};
use crate::foundation::RuntimeScopeRefV1;
pub(super) async fn assert_mask(h: &Harness, scope: RuntimeScopeRefV1, rows: i64) -> TestResult {
    let RuntimeScopeRefV1::Instance { instance_id, .. } = scope else {
        return Err("actual Bakragore source Instance".into());
    };
    let (count, minimum, maximum): (i64, i16, i16) = sqlx::query_as(
        "SELECT count(*),coalesce(min(p.essence_mask),-1)::smallint,coalesce(max(p.essence_mask),-1)::smallint FROM game_rotten_bak_phase_preparations p JOIN game_activity_handoff_groups g USING(handoff_id) WHERE g.instance_id=encode($1,'hex')::uuid"
    ).bind(instance_id.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        count, rows,
        "entry records only origin; the actual reserved FIRST think creates exactly one global phase decision receipt"
    );
    if rows == 1 {
        assert_eq!(
            (minimum, maximum),
            (0, 0),
            "the real empty global phase yields an explicit zero, never inferred absent preparation"
        );
    }
    Ok(())
}
