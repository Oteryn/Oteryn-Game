use super::*;

#[test]
fn fatal_retention_keeps_the_first_error() {
    let mut error = None;
    retain_first_error(&mut error, ShellError::RendererResize);
    retain_first_error(&mut error, ShellError::RendererRender);
    assert_eq!(error, Some(ShellError::RendererResize));
}

#[test]
fn redraw_requires_a_configured_renderer() {
    assert!(redraw_eligible(Some(SurfacePhase::Configured)));
    assert!(!redraw_eligible(None));
    assert!(!redraw_eligible(Some(SurfacePhase::Unconfigured)));
    assert!(!redraw_eligible(Some(SurfacePhase::Suspended)));
    assert!(!redraw_eligible(Some(SurfacePhase::Lost)));
    assert!(!redraw_eligible(Some(SurfacePhase::Closing)));
}
