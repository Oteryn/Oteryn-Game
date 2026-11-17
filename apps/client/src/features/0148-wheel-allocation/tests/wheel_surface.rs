#[test]
fn wheel_graph_actual_renderer_batch_uses_one384_pixel_cell()
-> Result<(), oteryn_renderer::BatchError> {
    assert!(super::frame(&Default::default())?.is_none());
    let (atlas, batch) = super::raster_frame(vec![0; 384 * 384 * 4])?;
    assert_eq!(batch.len(), 1);
    let view = oteryn_renderer::TileView::new(oteryn_renderer::TileCoord::new(0, 0), 384, 1, 1)?;
    assert_eq!(view.viewport_px(), (384, 384));
    assert!(oteryn_renderer::TileBatch::from_cells(&view, &atlas, &[0, 0]).is_err());
    Ok(())
}
