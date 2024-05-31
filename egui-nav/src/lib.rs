use core::fmt::Display;
use egui::{Color32, Pos2, Sense, Stroke, Vec2};

pub struct Nav<'r, T> {
    /// The back chevron stroke
    stroke: Stroke,
    chevron_size: Vec2,
    route: &'r [T],
}

impl<'r, T> Nav<'r, T> {
    /// Nav requires at least one route or it will panic
    pub fn new(route: &'r [T]) -> Self {
        // precondition: we must have at least one route. this simplifies
        // the rest of the control, and it's easy to catchbb
        assert!(route.len() > 0, "Nav routes cannot be empty");
        let chevron_size = Vec2::new(14.0, 20.0);
        let stroke = Stroke::new(2.0, Color32::GOLD);

        Nav {
            stroke,
            chevron_size,
            route,
        }
    }

    pub fn stroke(mut self, stroke: impl Into<Stroke>) -> Self {
        self.stroke = stroke.into();
        self
    }

    pub fn chevron_size(mut self, size: Vec2) -> Self {
        self.chevron_size = size;
        self
    }

    /// Nav guarantees there is at least one route element
    pub fn top(&self) -> &'r T {
        &self.route[self.route.len() - 1]
    }

    /// Get the Route at some position near the top of the stack
    ///
    /// Example:
    ///
    /// For route &[Route::Home, Route::Profile]
    ///
    ///   - routes.top_n(0) for the top route, Route::Profile
    ///   - routes.top_n(1) for the route immediate before the top route, Route::Home
    ///
    pub fn top_n(&self, n: usize) -> Option<&T> {
        self.route.get(self.route.len() - 1 - n)
    }

    /// Safer version of new if we're not sure if we will have non-empty routes
    pub fn try_new(route: &'r [T]) -> Option<Self> {
        if route.len() == 0 {
            None
        } else {
            Some(Nav::new(route))
        }
    }

    pub fn show<F, R>(&self, ui: &mut egui::Ui, show_route: F) -> R
    where
        F: Fn(&mut egui::Ui, &Nav<'_, T>) -> R,
        T: Display,
    {
        let route = self.top();
        if let Some(under) = self.top_n(1) {
            let _back_response = ui
                .horizontal(|ui| {
                    let r = chevron(ui, 4.0, self.chevron_size, self.stroke);
                    ui.label(format!("{}", under));
                    r
                })
                .inner;

            show_route(ui, self)
        } else {
            show_route(ui, self)
        }
    }
}

fn chevron(ui: &mut egui::Ui, pad: f32, size: Vec2, stroke: impl Into<Stroke>) -> egui::Response {
    let (r, painter) = ui.allocate_painter(size, Sense::click());

    let min = r.rect.min;
    let max = r.rect.max;

    let apex = Pos2::new(min.x + pad, min.y + size.y / 2.0);
    let top = Pos2::new(max.x - pad, min.y + pad);
    let bottom = Pos2::new(max.x - pad, max.y - pad);

    let stroke = stroke.into();
    painter.line_segment([apex, top], stroke);
    painter.line_segment([apex, bottom], stroke);

    r
}
