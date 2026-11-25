// Copyright 2026 Anud Inc.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Offscreen rendering for screenshots and timings. Needs a GPU or a software Vulkan driver.

use std::{rc::Rc, sync::Arc, time::Instant};

use ui::{
    AppContext, Bounds, HeadlessAppContext, WindowBounds, WindowOptions,
    assets::Assets,
    component::{Theme, ThemeMode},
    px, size,
    test::TestWindowExt,
};
use views_ir::Limits;
use views_runtime::{Host, MemoryStore};

use crate::ViewPane;

/// A rendered view.
pub struct Shot {
    pub image: image::RgbaImage,
    /// Time to build the pane and draw the first frame.
    pub first_frame_us: u128,
    /// Time to draw a second frame with no changes.
    pub warm_frame_us: u128,
    pub truncated: bool,
    /// Element building in the first and second frame (no layout or paint).
    pub build_us: (u64, u64),
}

pub fn context() -> HeadlessAppContext {
    let mut cx = HeadlessAppContext::with_platform(
        ui::platform::current_platform(true).text_system(),
        Arc::new(Assets),
        ui::platform::current_headless_renderer,
    );
    cx.update(ui::init);
    cx
}

/// Render `source` at `width` logical pixels wide in light or dark, then crop to content.
pub fn render(
    cx: &mut HeadlessAppContext,
    source: &str,
    dark: bool,
    width: f32,
    host: Rc<dyn Host>,
) -> Result<Shot, String> {
    cx.update(|cx| {
        Theme::change(
            if dark {
                ThemeMode::Dark
            } else {
                ThemeMode::Light
            },
            None,
            cx,
        )
    });
    let height = 1600.0;
    let src = source.to_string();
    let start = Instant::now();
    let (handle, pane) = cx
        .update(|cx| {
            ui::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: Default::default(),
                        size: size(px(width), px(height)),
                    })),
                    focus: false,
                    show: false,
                    ..Default::default()
                },
                cx,
                |_, cx| {
                    cx.new(|_| {
                        ViewPane::with_host(&src, host, MemoryStore::handle(), Limits::default())
                    })
                },
            )
        })
        .map_err(|e| e.to_string())?;
    cx.update_window(handle, |_, window, cx| window.render_frame(cx))
        .map_err(|e| e.to_string())?;
    let first_frame_us = start.elapsed().as_micros();
    let first_build = cx.update(|cx| pane.read(cx).build_us());
    let t = Instant::now();
    cx.update_window(handle, |_, window, cx| {
        window.refresh();
        window.render_frame(cx)
    })
    .map_err(|e| e.to_string())?;
    let warm_frame_us = t.elapsed().as_micros();
    let truncated = cx.update(|cx| pane.read(cx).truncated());
    let second_build = cx.update(|cx| pane.read(cx).build_us());
    let image = cx.capture_screenshot(handle).map_err(|e| e.to_string())?;
    let _ = cx.update_window(handle, |_, window, _| window.remove_window());
    Ok(Shot {
        image: crop(image),
        first_frame_us,
        warm_frame_us,
        truncated,
        build_us: (first_build, second_build),
    })
}

/// Trim trailing rows that are plain background, keeping a margin.
fn crop(img: image::RgbaImage) -> image::RgbaImage {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return img;
    }
    let bg = *img.get_pixel(w - 1, h - 1);
    let mut last = 0;
    for y in (0..h).rev() {
        if (0..w).any(|x| img.get_pixel(x, y) != &bg) {
            last = y;
            break;
        }
    }
    let keep = (last + 32).min(h);
    image::imageops::crop_imm(&img, 0, 0, w, keep).to_image()
}
