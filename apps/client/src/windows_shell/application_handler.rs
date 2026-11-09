use super::*;

impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let (Some(window), Some(renderer)) = (&self.window, &mut self.renderer) {
            let size = window.inner_size();
            if renderer
                .resume(self.generation, size.width, size.height)
                .is_err()
            {
                self.fail(event_loop, ShellError::RendererResume);
            }
            return;
        }
        if self.window.is_some() {
            return;
        }
        let attributes = WindowAttributes::default()
            .with_title(tr!("Oteryn — Enter: connection and sign-in"))
            .with_active(self.smoke)
            .with_inner_size(LogicalSize::new(960.0, 540.0));
        let Ok(window) = event_loop.create_window(attributes) else {
            self.fail(event_loop, ShellError::WindowCreation);
            return;
        };
        let window = Arc::new(window);
        if self.smoke {
            self.window = Some(window);
            event_loop.exit();
            return;
        }
        self.hud = hud::Hud::new(&window);
        self.settings.attach(&window);
        if self.hud.is_none() {
            self.fail(event_loop, ShellError::WindowCreation);
            return;
        }
        let size = window.inner_size();
        // The map is loaded on the first resume and kept across suspends.
        let center = self
            .play
            .as_ref()
            .map_or((START.0, START.1, 7), |play| play.view.map_center());
        let world = if let Some(world) = &self.world
            && world.center() == center
        {
            Arc::clone(world)
        } else {
            let (world, reason) = World::from_env(center);
            if let Some(reason) = reason {
                lprintln!(
                    "Oteryn: drawing without map sprites: {reason}",
                    reason = reason
                );
            }
            let Ok(world) = world.map(Arc::new) else {
                self.fail(event_loop, ShellError::RendererInitialization);
                return;
            };
            self.world = Some(Arc::clone(&world));
            world
        };
        let shown = match &mut self.play {
            Some(_) => Ok(()),
            None => self.rebuild_offline(),
        };
        if shown.is_err() {
            self.fail(event_loop, ShellError::RendererInitialization);
            return;
        }
        let Ok(mut renderer) = WindowsRenderer::new(
            Arc::clone(&window),
            self.generation,
            size.width,
            size.height,
        ) else {
            self.fail(event_loop, ShellError::RendererInitialization);
            return;
        };
        renderer.set_atlas(world.atlas().clone());
        self.atlas = Some(world.atlas().clone());
        window.request_redraw();
        self.window = Some(window);
        self.renderer = Some(renderer);
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(renderer) = &mut self.renderer
            && renderer.suspend(self.generation).is_err()
        {
            self.fail(event_loop, ShellError::RendererSuspend);
        }
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if let Ok(events) = self.input.process_device_event(&event) {
            self.handle_input(event_loop, &events);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // Every window event goes through the adapter and router first; select or walk is
        // triggered only by the routed gameplay action.
        if let Ok(events) = self.input.process_window_event(&event) {
            self.handle_input(event_loop, &events);
        }
        match event {
            WindowEvent::CloseRequested => {
                if !self.settings.confirm_exit() {
                    return;
                }
                if let Some(renderer) = &mut self.renderer
                    && renderer.close(self.generation).is_err()
                {
                    self.fail(event_loop, ShellError::RendererClose);
                    return;
                }
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer
                    && renderer
                        .resize(self.generation, size.width, size.height)
                        .is_err()
                {
                    self.fail(event_loop, ShellError::RendererResize);
                }
            }
            WindowEvent::RedrawRequested => {
                self.redraw(event_loop);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(error) = self.apply_graphics() {
            self.fail(event_loop, error);
            return;
        }
        if self.settings.take_magical_archive_request()
            && let Some(play) = &self.play
        {
            play.link.magical_archive_open();
        }
        let open_states: Vec<_> = oteryn_client::play::WINDOW_ACTIONS
            .iter()
            .map(|(id, _)| {
                self.play
                    .as_ref()
                    .is_some_and(|play| play.link.window_open(id))
            })
            .collect();
        if let Some((id, events)) = self
            .settings
            .take_window_events(self.play.is_some(), &open_states)
            && let Some(play) = &self.play
        {
            play.link.window_toggle(id, &events);
        }
        if self.settings.take_native_sign_out() {
            if let Some(entry) = &mut self.entry {
                entry.native_sign_out();
            }
            if let Some(play) = &mut self.play {
                play.view.logout();
            }
            self.leave_play("Native sign-out requested. Clearing the saved device credential.");
        }
        self.poll_entry();
        if let Some(play) = &mut self.play {
            self.settings.restore(&mut play.view, &play.link);
            play.view.enable_resource_hud(true);
        }
        if let Some(play) = &mut self.play
            && let Err(end) = play.view.tick(&play.link)
        {
            match end {
                PlayEnd::Ended(class) => self.return_to_login(class),
                PlayEnd::LoggedOut => self.return_to_character_selection(),
            }
        }
        if let (Some(play), Some(window)) = (&self.play, &self.window) {
            play.link.item_drag_tick(&play.view, window);
        }
        if self.play.is_some()
            && let Some(window) = &self.window
        {
            oteryn_client::miniwindow::native::update(window);
        }
        self.update_hud();
        self.schedule_frames(event_loop);
    }
}
