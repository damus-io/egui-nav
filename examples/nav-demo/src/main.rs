use eframe::egui;
use egui::Frame;
use egui_demo_lib::{easy_mark::EasyMarkEditor, ColorTest};
use egui_nav::{DefaultNavTitle, DefaultTitleResponse, Nav, NavAction, NavUiType, PopupSheet};
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
            Ok(Box::new(MyApp {
                navigating: false,
                returning: false,
                routes: test_routes(),
                popup: None,
            }))
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
    popup: Option<Route>,
    navigating: bool,
    returning: bool,
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

enum OurNavAction {
    Navigating(Route),
    Popup(Route),
    Returning,
}

fn nav_ui(ui: &mut egui::Ui, app: &mut MyApp) {
    ui.visuals_mut().interact_cursor = Some(egui::CursorIcon::PointingHand);

    if let Some(popup) = app.popup {
        if let Some(bg_route) = app.routes.last() {
            let resp =
                PopupSheet::new(bg_route, &popup)
                    .navigating(app.navigating)
                    .returning(app.returning)
                    .show(ui, |ui, typ, bg_route| match typ {
                        NavUiType::Title => DefaultNavTitle::default()
                            .ui(ui, &vec![&bg_route])
                            .map(|n| match n {
                                DefaultTitleResponse::Back => OurNavAction::Returning,
                            }),

                        NavUiType::Body => match *bg_route {
                            Route::Editor => {
                                ui.vertical(|ui| {
                                    let mut action: Option<OurNavAction> = None;

                                    if ui.button("Color Test").clicked() {
                                        action = Some(OurNavAction::Navigating(Route::ColorTest));
                                    }

                                    if ui.button("Popup color test").clicked() {
                                        action = Some(OurNavAction::Popup(Route::ColorTest));
                                    }

                                    let _ = ui.button("Back");

                                    EasyMarkEditor::default().ui(ui);
                                    action
                                })
                                .inner
                            }

                            Route::ColorTest => {
                                ui.vertical(|ui| {
                                    let mut action: Option<OurNavAction> = None;
                                    if ui.button("Editor").clicked() {
                                        action = Some(OurNavAction::Navigating(Route::Editor));
                                    }
                                    let _ = ui.button("Back");
                                    ColorTest::default().ui(ui);
                                    action
                                })
                                .inner
                            }
                        },
                    });

            if let Some(NavAction::Returned) = resp.action {
                app.popup = None;
                app.returning = false;
            } else if let Some(NavAction::Navigated) = resp.action {
                app.navigating = false;
            }

            return;
        }
    }

    let response = Nav::new(&app.routes)
        .navigating(app.navigating)
        .returning(app.returning)
        .show(ui, |ui, typ, nav| match typ {
            NavUiType::Title => DefaultNavTitle::default()
                .ui(ui, nav.routes())
                .map(|n| match n {
                    DefaultTitleResponse::Back => OurNavAction::Returning,
                }),

            NavUiType::Body => match nav.top() {
                Route::Editor => {
                    ui.vertical(|ui| {
                        let mut action: Option<OurNavAction> = None;

                        if ui.button("Color Test").clicked() {
                            action = Some(OurNavAction::Navigating(Route::ColorTest));
                        }

                        if ui.button("Popup color test").clicked() {
                            action = Some(OurNavAction::Popup(Route::ColorTest));
                        }

                        if nav.routes().len() > 1 && ui.button("Back").clicked() {
                            action = Some(OurNavAction::Returning);
                        }

                        EasyMarkEditor::default().ui(ui);
                        action
                    })
                    .inner
                }

                Route::ColorTest => {
                    ui.vertical(|ui| {
                        let mut action: Option<OurNavAction> = None;
                        if ui.button("Editor").clicked() {
                            action = Some(OurNavAction::Navigating(Route::Editor));
                        }
                        if nav.routes().len() > 1 && ui.button("Back").clicked() {
                            action = Some(OurNavAction::Returning);
                        }
                        ColorTest::default().ui(ui);
                        action
                    })
                    .inner
                }
            },
        });

    if let Some(action) = response.response.or(response.title_response) {
        match action {
            OurNavAction::Navigating(route) => {
                app.navigating = true;
                app.routes.push(route);
            }
            OurNavAction::Returning => {
                app.returning = true;
            }
            OurNavAction::Popup(route) => {
                app.popup = Some(route);
                app.navigating = true;
            }
        }
    }

    if let Some(action) = response.action {
        if let NavAction::Returned = action {
            app.routes.pop();
            app.returning = false;
            println!("Popped route {:?}", app.routes);
        } else if let NavAction::Navigated = action {
            app.navigating = false;
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(Frame::new().outer_margin(egui::Margin::same(50)))
            .show(ctx, |ui| {
                let cells = 2;
                let width = ui.available_rect_before_wrap().width() / (cells as f32);

                egui_extras::StripBuilder::new(ui)
                    .sizes(egui_extras::Size::exact(width), cells)
                    .clip(false)
                    .horizontal(|mut strip| {
                        strip.cell(|ui| {
                            nav_ui(ui, self);
                        });

                        strip.cell(|ui| {
                            ui.painter().rect_filled(
                                ui.available_rect_before_wrap(),
                                0.0,
                                egui::Color32::from_rgb(0x20, 0x20, 0x20),
                            );
                        });
                    })
            });
    }
}
