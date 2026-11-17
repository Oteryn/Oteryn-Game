//! Queries the already compiled physical entry owners; return-only layers do not qualify.
use super::{Binding, NativeBindings};

impl NativeBindings {
    pub fn admits_entry_layer(&self, key: &str) -> bool {
        self.epoch != 0
            && self.bindings.iter().any(|binding| {
                matches!(binding, Binding::Layer { key: bound, exit_only: false } if bound == key)
            })
    }
}
