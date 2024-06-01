use core::fmt::Display;
use egui::{
    emath::TSTransform, pos2, vec2, Color32, LayerId, Order, Pos2, Rect, Sense, Stroke, Vec2,
};

pub struct Nav<'r, T> {
    /// The back chevron stroke
    padding: f32,
    stroke: Stroke,
    chevron_size: Vec2,
    route: &'r [T],
}

pub enum NavAction {
    Returning(f32),
}

#[derive(Clone, Copy, Debug, Default)]
struct State {
    offset: f32,
    popped_min_rect: Option<Rect>,
}

impl State {
    pub fn load(ctx: &egui::Context, id: egui::Id) -> Option<Self> {
        ctx.data_mut(|d| d.get_temp(id))
    }

    pub fn store(self, ctx: &egui::Context, id: egui::Id) {
        ctx.data_mut(|d| d.insert_temp(id, self));
    }
}

/*
pub struct NavResponse<R> {
    inner: R,
    response: egui::Response,
    action: Option<NavAction>,
}
*/

impl<'r, T> Nav<'r, T> {
    /// Nav requires at least one route or it will panic
    pub fn new(route: &'r [T]) -> Self {
        // precondition: we must have at least one route. this simplifies
        // the rest of the control, and it's easy to catchbb
        assert!(route.len() > 0, "Nav routes cannot be empty");
        let chevron_size = Vec2::new(14.0, 20.0);
        let stroke = Stroke::new(2.0, Color32::GOLD);
        let padding = 4.0;

        Nav {
            padding,
            stroke,
            chevron_size,
            route,
        }
    }

    pub fn chevron_padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
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

    fn header(&self, ui: &mut egui::Ui, label: String) -> egui::Response {
        ui.horizontal(|ui| {
            let r = chevron(ui, self.padding, self.chevron_size, self.stroke);
            let label_response = ui.add(
                egui::Label::new(label)
                    .sense(Sense::click())
                    .selectable(false),
            );

            let response = r.union(label_response);

            if let Some(cursor) = ui.visuals().interact_cursor {
                if response.hovered() {
                    ui.ctx().set_cursor_icon(cursor);
                }
            }

            if response.clicked() {}

            response
        })
        .inner
    }

    pub fn show<F, R>(&self, ui: &mut egui::Ui, show_route: F) -> R
    where
        F: Fn(&mut egui::Ui, &Nav<'_, T>) -> R,
        T: Display,
    {
        if let Some(under) = self.top_n(1) {
            let _r = self.header(ui, under.to_string());
        }

        let id = ui.id().with("nav");
        let mut state = State::load(ui.ctx(), id).unwrap_or_default();
        let available_rect = ui.available_rect_before_wrap();

        // Drag contents to transition back.
        // We must do this BEFORE adding content to the `Nav`,
        // or we will steal input from the widgets we contain.
        let content_response = ui.interact(available_rect, id.with("drag"), Sense::drag());

        if content_response.dragged() {
            state.offset += ui.input(|input| input.pointer.delta()).x;
        } else {
            // If we're not dragging, animate the current offset back to
            // the current or previous view depending on how much we are
            // offset

            let abs_offset = state.offset.abs();
            if abs_offset > 0.0 {
                let sgn = state.offset.signum();
                let amt = springy(state.offset);
                let adj = amt * sgn;
                let adjusted = state.offset - adj;

                // if adjusting will flip a sign, then just set to 0
                state.offset = if (state.offset - adj).signum() != sgn {
                    0.0
                } else {
                    adjusted
                };

                // since we're animating we need to request a repaint
                ui.ctx().request_repaint();
            }
        }

        state.store(ui.ctx(), id);

        // transition rendering
        if state.offset > 0.0 && self.route.len() >= 2 {
            // behind transition layer
            {
                let id = ui.id().with("behind");
                let min_rect = state.popped_min_rect.unwrap_or(available_rect);
                let progress = state.offset / available_rect.width();
                let initial_shift = -min_rect.width() * 0.1;
                let mut amt = initial_shift + springy(state.offset);
                if amt > 0.0 {
                    amt = 0.0;
                }

                //let clip_width = state.offset.max(available_rect.width());
                let clip = Rect::from_min_size(
                    available_rect.min,
                    vec2(state.offset - amt, available_rect.max.y),
                );

                let mut ui = egui::Ui::new(
                    ui.ctx().clone(),
                    LayerId::new(Order::Foreground, id),
                    id,
                    available_rect,
                    clip,
                );

                // render the previous nav view in the background when
                // transitioning
                let nav = Nav {
                    route: &self.route[..self.route.len() - 1],
                    ..*self
                };
                let _r = show_route(&mut ui, &nav);

                state.popped_min_rect = Some(ui.min_rect());

                if amt < 0.0 {
                    ui.ctx().transform_layer_shapes(
                        ui.layer_id(),
                        TSTransform::from_translation(Vec2::new(amt, 0.0)),
                    );
                }
            }

            // foreground layer
            {
                let id = ui.id().with("front");

                let mut ui = egui::Ui::new(
                    ui.ctx().clone(),
                    LayerId::new(Order::Foreground, id),
                    id,
                    available_rect,
                    available_rect,
                );

                // render the previous nav view in the background when
                // transitioning
                let r = show_route(&mut ui, self);

                ui.ctx().transform_layer_shapes(
                    ui.layer_id(),
                    egui::emath::TSTransform::from_translation(Vec2::new(state.offset, 0.0)),
                );

                r
            }
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

fn springy(offset: f32) -> f32 {
    ((offset.abs().powf(1.2) - 1.0) * 0.1).max(0.5)
}
