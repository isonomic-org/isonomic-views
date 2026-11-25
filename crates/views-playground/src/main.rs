// Copyright 2026 Anud Inc.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Runs the playground.

use ui::AppContext;
use views_playground::Playground;

fn main() {
    ui::application().run(|cx| {
        ui::init(cx);
        let options = ui::WindowOptions {
            window_bounds: Some(ui::WindowBounds::centered(
                ui::size(ui::px(1280.), ui::px(820.)),
                cx,
            )),
            ..Default::default()
        };
        if let Err(e) = ui::open_window(options, cx, |window, cx| {
            cx.new(|cx| Playground::new(window, cx))
        }) {
            eprintln!("could not open the playground window: {e}");
        }
    });
}
