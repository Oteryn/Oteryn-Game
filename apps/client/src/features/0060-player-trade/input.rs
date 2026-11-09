use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::player_trade::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn player_trade_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.player_trade, intent)
            && self
                .commands
                .send(PlayCommand::PlayerTrade(command))
                .is_err()
        {
            mailbox.player_trade.pending = false;
            mailbox.player_trade.feedback = Some(tr!("The session has ended"));
        }
    }
    pub fn player_trade_choose_partner(&self, partner: oteryn_session::EntityRef) -> bool {
        oteryn_client_ui::player_trade::picker::select_partner(
            &mut lock(&self.mailbox).player_trade,
            partner,
        )
    }
    pub fn player_trade_choose_item(&self, row: usize) -> bool {
        oteryn_client_ui::player_trade::picker::select_item(
            &mut lock(&self.mailbox).player_trade,
            row,
        )
    }
    pub fn player_trade_offer_selected(&self) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) =
            oteryn_client_ui::player_trade::picker::offer_selected(&mut mailbox.player_trade)
            && self
                .commands
                .send(PlayCommand::PlayerTrade(command))
                .is_err()
        {
            mailbox.player_trade.pending = false;
            mailbox.player_trade.feedback = Some(tr!("The session has ended"));
        }
    }
    pub fn player_trade_open_container(&self, row: usize) {
        let mut mailbox = lock(&self.mailbox);
        open_container(&mut mailbox.player_trade, row, &self.commands);
    }

    pub fn player_trade_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::player_trade::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).player_trade.open = true;
        }
        true
    }
}

impl crate::play::PlayView {
    pub fn player_trade_context_target(
        &self,
        tile: oteryn_renderer::TileCoord,
    ) -> Option<oteryn_session::EntityRef> {
        self.entities
            .iter()
            .rev()
            .find(|entity| {
                entity.kind == oteryn_session::EntityKind::Player
                    && entity.position.floor == self.floor
                    && oteryn_renderer::TileCoord::new(entity.position.x, entity.position.y) == tile
            })
            .map(|entity| entity.entity)
    }
    pub fn player_trade_context_input(&self, link: &PlayLink, clicks: &[(i32, i32)]) -> bool {
        if !lock(&link.mailbox).player_trade.selected {
            return false;
        }
        let mut consumed = false;
        for &(x, y) in clicks {
            if let Some(tile) =
                crate::input::click_tile(self.scene().view(), f64::from(x), f64::from(y))
                && let Some(partner) = self.player_trade_context_target(tile)
            {
                consumed = true;
                #[cfg(windows)]
                super::view::context_menu(link, partner);
                #[cfg(not(windows))]
                let _ = link.player_trade_choose_partner(partner);
            }
        }
        consumed
    }
}

impl PlayLink {
    /// Run before modal/key handlers: even a consumed batch updates pointer and focus state.
    pub fn player_trade_track_pointer(&self, events: &[NormalizedInputEvent]) -> Vec<(i32, i32)> {
        use oteryn_input_actions::{ButtonState, Modifiers, MouseButton};
        let mut mailbox = lock(&self.mailbox);
        let mut clicks = Vec::new();
        for event in events {
            match event {
                NormalizedInputEvent::PointerMoved { position, .. } => {
                    mailbox.player_trade.context_pointer =
                        Some((position.x().get(), position.y().get()));
                }
                NormalizedInputEvent::FocusChanged { focused: false }
                | NormalizedInputEvent::DeviceLost => {
                    mailbox.player_trade.context_pointer = None;
                    clicks.clear();
                }
                NormalizedInputEvent::MouseButton {
                    button: MouseButton::SECONDARY,
                    state: ButtonState::Pressed,
                    modifiers,
                } if *modifiers == Modifiers::NONE => {
                    if let Some(pointer) = mailbox.player_trade.context_pointer {
                        clicks.push(pointer);
                    }
                }
                _ => {}
            }
        }
        clicks
    }
}

pub(in crate::play) fn open_container(
    panel: &mut oteryn_client_ui::player_trade::PlayerTradePanel,
    row: usize,
    commands: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
) {
    use oteryn_session::container_tree::{ContainerViewIntent, encode_container_view_intent};
    if !panel.selected || !panel.items.can_open || panel.items.pending || panel.pending {
        return;
    }
    let Some((_, item)) = oteryn_client_ui::player_trade::picker::rows(&panel.items)
        .get(row)
        .copied()
    else {
        return;
    };
    let Ok(payload) = encode_container_view_intent(&ContainerViewIntent::Open {
        handle: item.handle,
        replace_view: None,
    }) else {
        return;
    };
    panel.items.pending = true;
    if commands
        .send(PlayCommand::Feature(
            oteryn_client_runtime::raw_feature::FeatureCommand {
                capability: 14,
                kind: 21,
                payload,
            },
        ))
        .is_err()
    {
        panel.items.pending = false;
        panel.feedback = Some(tr!("The session has ended"));
    }
}
