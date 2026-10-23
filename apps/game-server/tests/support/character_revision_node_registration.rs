//! Test-only access to the existing private registration helper; no fixture copy.
pub(crate) async fn registered_node(
    root: &crate::durability::DurabilityRoot,
    tag: u8,
) -> super::TestResult<crate::durability::runtime_scope_assignment::NodeIncarnationProof> {
    super::register(root, tag).await
}
