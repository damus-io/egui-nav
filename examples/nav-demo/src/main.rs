use eframe::egui;
use egui::Frame;
use egui_demo_lib::{easy_mark::EasyMarkEditor, ColorTest};
use egui_nav::Nav;
use std::fmt;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native("Nav Demo", options, Box::new(|_cc| Box::<MyApp>::default()))
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "the_canvas_id", // hardcode it
                web_options,
                Box::new(|cc| Box::<MyApp>::default()),
            )
            .await
            .expect("failed to start eframe");
    });
}

#[derive(Default)]
struct MyApp {}

enum Route {
    Editor,
    ColorTest,
}

impl fmt::Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Route::Editor => write!(f, "Editor"),
            Route::ColorTest => write!(f, "Color Test"),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(Frame::none())
            .show(ctx, |ui| {
                ui.visuals_mut().interact_cursor = Some(egui::CursorIcon::PointingHand);
                let route = &[Route::Editor, Route::ColorTest];
                Nav::new(route).show(ui, |ui, nav| match nav.top() {
                    Route::Editor => EasyMarkEditor::default().ui(ui),
                    Route::ColorTest => ColorTest::default().ui(ui),
                })
            });
    }
}
