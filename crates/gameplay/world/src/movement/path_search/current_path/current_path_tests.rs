#![allow(clippy::panic, clippy::unwrap_used)]
use super::*;
use crate::movement::path_budget::ChannelPathBudget;
fn at(x: i32) -> MovementLocalPosition {
    MovementLocalPosition { x, y: 0, floor: 7 }
}
struct Tiles {
    wait: bool,
}
impl PathTileReader for Tiles {
    async fn cost(&mut self, at: MovementLocalPosition) -> Option<u32> {
        if self.wait {
            std::future::pending::<()>().await;
        }
        (at.y == 0 && (0..4).contains(&at.x)).then_some(0)
    }
}
#[test]
fn same_kernel_orders_sync_and_current_routes_and_charges_cancelled_work() {
    let (expected, work) = measured_route(at(0), at(3), false, |at| {
        (at.y == 0 && (0..4).contains(&at.x)).then_some(0)
    });
    let mut budget = ChannelPathBudget::default();
    let mut tiles = Tiles { wait: false };
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    let actual = {
        let mut route = std::pin::pin!(measured_current_route(
            at(0),
            at(3),
            false,
            &mut tiles,
            &mut budget
        ));
        match route.as_mut().poll(&mut cx) {
            std::task::Poll::Ready(result) => result,
            std::task::Poll::Pending => panic!("ready tiles"),
        }
    };
    assert_eq!(actual, (expected, work));
    assert_eq!(budget.charged_work(), work);
    tiles.wait = true;
    {
        let mut route = std::pin::pin!(measured_current_route(
            at(0),
            at(3),
            false,
            &mut tiles,
            &mut budget
        ));
        assert!(route.as_mut().poll(&mut cx).is_pending());
    }
    assert_eq!(
        budget.charged_work(),
        work + 1,
        "cancelling SQL cannot refund already expanded nodes"
    );
}
