//! Draw the current world and process screenshot requests after a successful presentation.
use super::*;
impl Application {
    pub(super) fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let scene = self
            .play
            .as_ref()
            .map(|play| play.view.scene())
            .or(self.scene.as_ref());
        if let (Some(renderer), Some(scene)) = (&mut self.renderer, scene)
            && redraw_eligible(Some(renderer.state().phase()))
        {
            if self.atlas.as_ref() != Some(scene.atlas()) {
                renderer.set_atlas(scene.atlas().clone());
                self.atlas = Some(scene.atlas().clone());
            }
            if renderer
                .render_batches(self.generation, scene.tiles(), scene.sprites())
                .is_err()
            {
                self.fail(event_loop, ShellError::RendererRender);
            } else {
                self.settings.screenshots(
                    renderer,
                    scene,
                    self.play.as_ref().map(|play| &play.link),
                );
            }
        }
    }
}
