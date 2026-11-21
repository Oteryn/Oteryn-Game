//! Local marker editor. Window handles and action storage remain on the creating thread.
#![allow(unsafe_code)]
#[derive(Default)]
struct Actions(std::cell::RefCell<Vec<(usize, usize)>>);
impl Actions {
    fn dispatch(&self, mut action: impl FnMut(usize, usize)) {
        // Window procedures can enqueue notifications synchronously during dispatch.
        let pending = std::mem::take(&mut *self.0.borrow_mut());
        for (id, notification) in pending {
            action(id, notification);
        }
    }
}
#[cfg(test)]
#[path = "tests/native_actions.rs"]
mod tests;
