use eframe::egui;
use egui::{Direction, Frame, Layout};
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
                Box::new(|_cc| Box::<MyApp>::default()),
            )
            .await
            .expect("failed to start eframe");
    });
}

#[derive(Default)]
struct MyApp {}

enum Route {
    Home,
    Profile(String),
}

impl fmt::Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Route::Home => write!(f, "Home"),
            Route::Profile(name) => write!(f, "{}'s Profile", name),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(Frame::none())
            .show(ctx, |ui| {
                let route = &[Route::Home, Route::Profile("bob".to_string())];
                Nav::new(route).show(ui, |ui, nav| match nav.top_route() {
                    Route::Home => ui.label("Home body"),
                    Route::Profile(name) => ui.label("Profile body"),
                });
            });
    }
}
