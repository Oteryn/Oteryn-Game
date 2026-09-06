//! Audit-only probes copied into an untracked integration-test target in a disposable
//! checkout. They characterize the pinned implementation; they are not product fixes.

#[test]
fn characterize_pkce_length_boundary_in_actual_rust() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_identity::PkceMaterial;
    assert!(PkceMaterial::from_entropy(&[0x42; 31]).is_err());
    let minimum = PkceMaterial::from_entropy(&[0x42; 32])?;
    let maximum = PkceMaterial::from_entropy(&[0x42; 96])?;
    let above = PkceMaterial::from_entropy(&[0x42; 97])?;
    assert_eq!(minimum.verifier().expose().len(), 43);
    assert_eq!(maximum.verifier().expose().len(), 128);
    assert_eq!(above.verifier().expose().len(), 130);
    println!("AUDIT_FINDING_PKCE: actual Rust accepts 97 entropy bytes and returns 130 verifier characters; this is characterization, not a fixed regression");
    Ok(())
}

#[test]
fn characterize_zero_size_and_suspend_render_contract() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_foundation::ProcessGeneration;
    use oteryn_renderer::{SurfaceEvent, SurfacePhase, SurfaceState};
    let generation = ProcessGeneration::new(1);
    let mut state = SurfaceState::new(generation);
    state.apply(SurfaceEvent::Resize { generation, width: 640, height: 480 })?;
    state.apply(SurfaceEvent::Configured { generation })?;
    state.apply(SurfaceEvent::Resize { generation, width: 0, height: 0 })?;
    assert_eq!(state.phase(), SurfacePhase::Suspended);
    assert!(state.apply(SurfaceEvent::Presented { generation, suboptimal: false }).is_err());
    println!("AUDIT_RENDER_STATE: presentation after zero-size resize is rejected; application event-loop handling must suppress render or resume correctly");
    Ok(())
}

#[cfg(windows)]
#[test]
fn real_windows_renderer_creation_and_present() -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use oteryn_foundation::ProcessGeneration;
    use oteryn_renderer::WindowsRenderer;
    use winit::application::ApplicationHandler;
    use winit::event::WindowEvent;
    use winit::event_loop::{ActiveEventLoop, EventLoop};
    use winit::platform::windows::EventLoopBuilderExtWindows;
    use winit::window::{Window, WindowId};
    struct Probe { result: Option<Result<u64, String>> }
    impl ApplicationHandler for Probe {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            if self.result.is_some() { return; }
            self.result = Some((|| {
                let window = Arc::new(event_loop.create_window(Window::default_attributes().with_title("Oteryn isolated audit renderer probe")).map_err(|e| e.to_string())?);
                let size = window.inner_size();
                let generation = ProcessGeneration::new(1);
                let mut renderer = WindowsRenderer::new(window, generation, size.width.max(1), size.height.max(1)).map_err(|e| e.to_string())?;
                let deadline = Instant::now() + Duration::from_secs(5);
                while renderer.state().presented_frames() == 0 && Instant::now() < deadline {
                    renderer.render(generation).map_err(|e| e.to_string())?;
                    std::thread::sleep(Duration::from_millis(10));
                }
                let frames = renderer.state().presented_frames();
                if frames == 0 { return Err("no frame presented within bounded probe".to_owned()); }
                renderer.close(generation).map_err(|e| e.to_string())?;
                Ok(frames)
            })());
            event_loop.exit();
        }
        fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
    }
    let mut builder = EventLoop::builder();
    builder.with_any_thread(true);
    let event_loop = builder.build()?;
    let mut probe = Probe { result: None };
    event_loop.run_app(&mut probe)?;
    match probe.result {
        Some(Ok(frames)) => { println!("AUDIT_REAL_DX12_PRESENT: frames={frames}"); Ok(()) },
        Some(Err(reason)) => Err(std::io::Error::other(reason).into()),
        None => Err(std::io::Error::other("renderer callback was not executed").into()),
    }
}
