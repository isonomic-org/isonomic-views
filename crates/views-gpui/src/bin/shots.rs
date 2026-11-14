//! Render views offscreen to PNG files: `views-shots <out-dir> <file.view>...`.
//! Writes `<name>-light.png` and `<name>-dark.png` and prints one line per file.

use std::rc::Rc;

fn main() {
    let mut args = std::env::args().skip(1);
    let out = args.next().unwrap_or_else(|| "target/screenshots".into());
    let _ = std::fs::create_dir_all(&out);
    let mut cx = views_gpui::headless::context();
    let mut failed = 0;
    for path in args {
        let src = std::fs::read_to_string(&path).unwrap_or_default();
        let name = std::path::Path::new(&path)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        for dark in [false, true] {
            let mode = if dark { "dark" } else { "light" };
            match views_gpui::headless::render(
                &mut cx,
                &src,
                dark,
                720.0,
                Rc::new(views_gpui::fixtures::host()),
            ) {
                Ok(s) => {
                    let file = format!("{out}/{name}-{mode}.png");
                    match s.image.save(&file) {
                        Ok(()) => println!(
                            "{file} {}x{} first={}us warm={}us{}",
                            s.image.width(),
                            s.image.height(),
                            s.first_frame_us,
                            s.warm_frame_us,
                            if s.truncated { " TRUNCATED" } else { "" }
                        ),
                        Err(e) => {
                            failed += 1;
                            eprintln!("{file}: {e}");
                        }
                    }
                }
                Err(e) => {
                    failed += 1;
                    eprintln!("{path} ({mode}): {e}");
                }
            }
        }
    }
    std::process::exit(if failed > 0 { 1 } else { 0 });
}
