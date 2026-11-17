//! Close is queued until the window thread has released both GPU surfaces.
#[derive(Default)]
pub(super) struct WindowEvents {
    pub close_requested: bool,
    pub commands: Vec<usize>,
}
impl WindowEvents {
    pub fn request_close(&mut self) {
        self.close_requested = true;
        self.commands.clear();
    }
    pub fn command(&mut self, command: usize) {
        if !self.close_requested && self.commands.len() < 32 {
            self.commands.push(command);
        }
    }
}
/// HWND destruction must occur after every renderer borrowing its child HWNDs is dropped.
pub(super) fn close_with_surfaces<T>(surfaces: T, destroy_window: impl FnOnce()) {
    drop(surfaces);
    destroy_window();
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};
    struct Surface(Rc<RefCell<Vec<&'static str>>>);
    impl Drop for Surface {
        fn drop(&mut self) {
            self.0.borrow_mut().push("surface released");
        }
    }
    #[test]
    fn forge_native_close_releases_both_surfaces_before_window_and_ignores_commands()
    -> Result<(), &'static str> {
        let mut events = WindowEvents::default();
        events.command(110);
        events.request_close();
        events.command(111);
        assert!(events.close_requested);
        assert!(
            events.commands.is_empty(),
            "closing must not submit a queued paid action"
        );
        let order = Rc::new(RefCell::new(Vec::new()));
        let surfaces = [Surface(order.clone()), Surface(order.clone())];
        close_with_surfaces(surfaces, || {
            assert_eq!(&*order.borrow(), &["surface released", "surface released"]);
            order.borrow_mut().push("window destroyed");
        });
        assert_eq!(
            &*order.borrow(),
            &["surface released", "surface released", "window destroyed"]
        );
        Ok(())
    }
}
