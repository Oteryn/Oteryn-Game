use super::*;
impl CoinAuthority {
    pub fn webhook(&mut self, body: &[u8], signature: &str) -> Result<(), ProviderError> {
        let event = self.provider.verify(body, signature)?;
        if let Some((original, result)) = self.events.get(&event.event_id) {
            return if original == body {
                *result
            } else {
                Err(ProviderError::Replay)
            };
        }
        let result = self.apply_event(&event);
        self.events.insert(event.event_id, (body.to_vec(), result));
        result
    }
    fn apply_event(&mut self, event: &Event) -> Result<(), ProviderError> {
        let Some((_, order)) = self
            .orders
            .values_mut()
            .find(|(_, o)| o.order_id == event.order_id)
        else {
            return Err(ProviderError::Binding);
        };
        if order.price != event.price {
            return Err(ProviderError::Binding);
        }
        match event.kind {
            EventKind::Paid => {
                if !matches!(
                    order.state,
                    PurchaseState::AwaitingPayment | PurchaseState::Processing
                ) {
                    return Ok(());
                }
                self.balance.total_coins += order.coins;
                self.balance.available_coins += order.coins;
                self.balance.transferable_coins += order.coins;
                self.credits += 1;
                order.state = PurchaseState::Paid;
                order.code = "SANDBOX_PAID".into();
            }
            EventKind::Refund | EventKind::Chargeback => {
                if order.state == PurchaseState::Chargeback && event.kind == EventKind::Refund {
                    return Ok(());
                }
                if order.state == PurchaseState::Paid {
                    if self.balance.available_coins >= order.coins {
                        self.balance.total_coins -= order.coins;
                        self.balance.available_coins -= order.coins;
                        self.balance.transferable_coins =
                            self.balance.transferable_coins.saturating_sub(order.coins);
                        self.reversals += 1;
                    } else {
                        self.balance.review_required = true;
                        order.review_required = true;
                    }
                } else if !matches!(
                    order.state,
                    PurchaseState::Refunded | PurchaseState::Chargeback
                ) {
                    self.balance.review_required = true;
                    order.review_required = true;
                }
                order.state = if matches!(event.kind, EventKind::Refund) {
                    PurchaseState::Refunded
                } else {
                    PurchaseState::Chargeback
                };
                order.code = "SANDBOX_REVERSAL_OR_REVIEW".into();
            }
        }
        order.checkout_url = None;
        order.receipt_id = Some(event.event_id.clone());
        order.revision += 1;
        self.balance.revision += 1;
        Ok(())
    }
}
