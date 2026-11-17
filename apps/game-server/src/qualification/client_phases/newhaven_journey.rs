//! Explicit content gaps in the Newhaven player route.

pub(super) const CONTENT_GAPS: [&str; 1] = [
    "NEWHAVEN_STEP content-legion-helmet FAIL wrong content: Dawnport claim mapped as Newhaven quest",
];

pub(super) fn report_content_gaps() {
    for row in CONTENT_GAPS {
        super::evidence(row);
    }
}
