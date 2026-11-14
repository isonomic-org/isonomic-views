//! Renders the playground offscreen. With `PLAYGROUND_SHOT=<path>` it also writes a PNG.

use std::sync::Arc;

use ui::{
    AppContext, Bounds, HeadlessAppContext, WindowBounds, WindowOptions, assets::Assets, px, size,
    test::TestWindowExt,
};
use views_playground::Playground;

#[test]
fn the_playground_renders() {
    let mut cx = HeadlessAppContext::with_platform(
        ui::platform::current_platform(true).text_system(),
        Arc::new(Assets),
        ui::platform::current_headless_renderer,
    );
    cx.update(ui::init);
    let (handle, _) = cx
        .update(|cx| {
            ui::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: Default::default(),
                        size: size(px(1280.), px(820.)),
                    })),
                    focus: false,
                    show: false,
                    ..Default::default()
                },
                cx,
                |window, cx| cx.new(|cx| Playground::new(window, cx)),
            )
        })
        .expect("window");
    cx.update_window(handle, |_, window, cx| window.render_frame(cx))
        .expect("frame");
    cx.update_window(handle, |_, window, cx| window.render_frame(cx))
        .expect("frame");
    let shot = cx.capture_screenshot(handle).expect("screenshot");
    assert!(shot.width() > 0);
    if let Ok(path) = std::env::var("PLAYGROUND_SHOT") {
        shot.save(path).expect("save");
    }
}
