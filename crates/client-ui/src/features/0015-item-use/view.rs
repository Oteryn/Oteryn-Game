/// Persistent prompt shown after transient action feedback expires.
pub fn targeting_hint() -> String {
    tr!("Use with: choose a target (Escape cancels)").into()
}
