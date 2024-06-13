use eframe::egui;
use egui::Frame;
use egui_demo_lib::{easy_mark::EasyMarkEditor, ColorTest};
use egui_nav::{Nav, NavAction};
use std::fmt;

fn test_routes() -> Vec<Route> {
    vec![Route::Editor, Route::ColorTest, Route::Editor]
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Nav Demo",
        options,
        Box::new(|_cc| {
            Box::new(MyApp {
                navigating: false,
                routes: test_routes(),
            })
        }),
    )
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
                Box::new(|cc| {
                    Box::new(MyApp {
                        routes: test_routes(),
                    })
                }),
            )
            .await
            .expect("failed to start eframe");
    });
}

#[derive(Default)]
struct MyApp {
    routes: Vec<Route>,
    navigating: bool,
}

#[derive(Copy, Clone, Debug)]
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
            .frame(Frame::none().outer_margin(egui::Margin::same(50.0)))
            .show(ctx, |ui| {
                ui.visuals_mut().interact_cursor = Some(egui::CursorIcon::PointingHand);
                let response = Nav::new(self.routes.clone())
                    .navigating(self.navigating)
                    .show(ui, |ui, nav| match nav.top() {
                        Route::Editor => {
                            ui.vertical(|ui| {
                                let mut navigating: Option<Route> = None;
                                if ui.button("Color Test").clicked() {
                                    navigating = Some(Route::ColorTest);
                                }

                                EasyMarkEditor::default().ui(ui);
                                navigating
                            })
                            .inner
                        }

                        Route::ColorTest => {
                            ui.vertical(|ui| {
                                let mut navigating: Option<Route> = None;
                                if ui.button("Editor").clicked() {
                                    navigating = Some(Route::Editor);
                                }
                                ColorTest::default().ui(ui);
                                navigating
                            })
                            .inner
                        }
                    });

                if let Some(nav) = response.inner {
                    self.navigating = true;
                    self.routes.push(nav);
                }

                if let Some(action) = response.action {
                    if let NavAction::Returned = action {
                        self.routes.pop();
                        println!("Popped route {:?}", self.routes);
                    } else if let NavAction::Navigated = action {
                        self.navigating = false;
                    }
                }
            });
    }
}
