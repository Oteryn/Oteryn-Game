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
        let menu_events = self.settings.take_window_events(self.play.is_some());
        if !menu_events.is_empty() && self.play.is_some() {
            self.handle_input(event_loop, &menu_events);
        }
        self.poll_entry();
        if let Some(play) = &mut self.play {
            self.settings.restore(&mut play.view, &play.link);
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
        self.update_hud();
        if let Some(window) = &self.window
            && redraw_eligible(
                self.renderer
                    .as_ref()
                    .map(|renderer| renderer.state().phase()),
            )
        {
            window.request_redraw();
        }
    }
}
