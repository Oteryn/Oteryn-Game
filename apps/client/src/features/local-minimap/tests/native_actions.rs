use super::Actions;

#[test]
fn reentrant_notifications_are_queued_for_the_next_dispatch() {
    for command in [(207, 1), (109, 0)] {
        let actions = Actions::default();
        actions.0.borrow_mut().push(command);
        let mut dispatched = Vec::new();
        actions.dispatch(|id, notification| {
            dispatched.push((id, notification));
            actions.0.borrow_mut().push((201, 0x300));
        });
        assert_eq!(dispatched, [command]);
        assert_eq!(*actions.0.borrow(), [(201, 0x300)]);
        actions.dispatch(|id, notification| dispatched.push((id, notification)));
        assert_eq!(dispatched, [command, (201, 0x300)]);
        assert!(actions.0.borrow().is_empty());
    }
}
