//! Reuse the existing private Charm catalogue in the promotion read regression.
#[allow(
    dead_code,
    reason = "only character_authority_postgres consumes this shared promotion fixture"
)]
pub(crate) fn catalogue() -> crate::domain::charm::CharmCatalogue {
    super::catalogue()
}
