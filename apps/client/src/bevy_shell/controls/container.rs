use super::*;
impl HostOwner {
    pub(in crate::bevy_shell) fn take_container_tool_request(
        &mut self,
    ) -> Option<ContainerToolRequest> {
        self.controls.container_tool.take()
    }
    pub(in crate::bevy_shell) fn request_container_tool(
        &mut self,
        tool: ContainerTool,
        position: Option<(i32, i32, i16)>,
        view: Option<(u8, oteryn_session::ItemHandle)>,
    ) -> bool {
        if self.modal_active() {
            return false;
        }
        let Some(play) = &self.play else {
            return false;
        };
        if play.view.gameplay_input_blocked(&play.link)
            || position.is_some_and(|position| position.2 != play.view.satellite_player().2)
            || view.is_some_and(|(id, handle)| {
                !play
                    .link
                    .container_panel()
                    .views
                    .as_ref()
                    .is_some_and(|views| {
                        views.views.iter().any(|view| {
                            view.view_id == id
                                && view.container_handle == handle
                                && view.browse.is_none()
                        })
                    })
            })
        {
            return false;
        }
        use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers};
        let (code, modifiers) = match tool {
            ContainerTool::BrowseField => {
                (5, Modifiers::one(Modifier::Control).with(Modifier::Shift))
            }
            ContainerTool::Depot => (15, Modifiers::one(Modifier::Control).with(Modifier::Shift)),
            ContainerTool::Search => (4, Modifiers::one(Modifier::Control).with(Modifier::Shift)),
            ContainerTool::Sort => (22, Modifiers::one(Modifier::Control).with(Modifier::Alt)),
        };
        let Ok(code) = KeyCode::new(code) else {
            return false;
        };
        // Menu actions call the existing canonical opener, independent of the user's keyboard binding.
        let events = [NormalizedInputEvent::Key {
            code,
            state: ButtonState::Pressed,
            modifiers,
            repeat: false,
        }];
        let opened = match tool {
            ContainerTool::BrowseField => play.link.browse_field_input(&events),
            ContainerTool::Depot => play.link.depot_input(&events),
            ContainerTool::Search => play.link.depot_search_input(&events),
            ContainerTool::Sort => play.link.container_sort_input(&events),
        };
        // The Depot already has its own retained window (game_ui::depot) over `depot_panel().open`.
        if opened && tool != ContainerTool::Depot {
            self.controls.container_tool = Some(ContainerToolRequest {
                generation: self.generation(),
                tool,
                position,
                view,
            });
        }
        opened
    }
}
