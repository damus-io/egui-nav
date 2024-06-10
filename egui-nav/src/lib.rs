use core::fmt::Display;
use egui::{emath::TSTransform, vec2, LayerId, Order, Pos2, Rect, Sense, Stroke, Vec2};

pub struct Nav<T: Clone> {
    /// The back chevron stroke
    padding: f32,
    stroke: Option<Stroke>,
    chevron_size: Vec2,
    route: Vec<T>,
}

#[derive(Clone, Copy, Debug)]
pub enum NavAction {
    /// We're returning to the previous view
    Returning,

    /// We released the drag, but not far enough to actually return
    Resetting,

    /// We're dragging the view. We're not making a return decision yet
    Dragging,

    /// We've returning to the previous view. Pop your route!
    Returned,
}

impl NavAction {
    fn is_transitioning(&self) -> bool {
        match self {
            NavAction::Returning => true,
            NavAction::Resetting => true,
            NavAction::Dragging => true,
            NavAction::Returned => false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct State {
    offset: f32,
    action: Option<NavAction>,
    popped_min_rect: Option<Rect>,
}

impl State {
    fn is_transitioning(&self) -> bool {
        self.action.map_or(false, |s| s.is_transitioning())
    }
}

impl State {
    pub fn load(ctx: &egui::Context, id: egui::Id) -> Option<Self> {
        ctx.data_mut(|d| d.get_temp(id))
    }

    pub fn store(self, ctx: &egui::Context, id: egui::Id) {
        ctx.data_mut(|d| d.insert_temp(id, self));
    }
}

pub struct NavResponse<R> {
    pub inner: R,
    pub action: Option<NavAction>,
}

impl<T: Clone> Nav<T> {
    /// Nav requires at least one route or it will panic
    pub fn new(route: Vec<T>) -> Self {
        // precondition: we must have at least one route. this simplifies
        // the rest of the control, and it's easy to catchbb
        assert!(route.len() > 0, "Nav routes cannot be empty");
        let chevron_size = Vec2::new(14.0, 20.0);
        //let stroke = Stroke::new(2.0, Color32::GOLD);
        let stroke: Option<Stroke> = None;
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
        self.stroke = Some(stroke.into());
        self
    }

    pub fn chevron_size(mut self, size: Vec2) -> Self {
        self.chevron_size = size;
        self
    }

    /// Nav guarantees there is at least one route element
    pub fn top(&self) -> &T {
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
        let ind = self.route.len() as i32 - (n as i32) - 1;
        if ind < 0 {
            None
        } else {
            self.route.get(ind as usize)
        }
    }

    /// Safer version of new if we're not sure if we will have non-empty routes
    pub fn try_new(route: Vec<T>) -> Option<Self> {
        if route.len() == 0 {
            None
        } else {
            Some(Nav::new(route))
        }
    }

    fn header(
        &self,
        ui: &mut egui::Ui,
        label: String,
        back: Option<String>,
    ) -> Option<egui::Response> {
        let mut header_rect = ui.available_rect_before_wrap();
        header_rect.set_height(self.chevron_size.y + 4.0);

        let response = if let Some(back) = back {
            Some(ui.put(header_rect, |ui: &mut egui::Ui| {
                ui.horizontal_centered(|ui| {
                    let stroke = self
                        .stroke
                        .unwrap_or_else(|| Stroke::new(2.0, ui.visuals().hyperlink_color));

                    let chev_response = chevron(ui, self.padding, self.chevron_size, stroke);

                    let label_response = ui.add(
                        egui::Label::new(back)
                            .sense(Sense::click())
                            .selectable(false),
                    );

                    let response = chev_response.union(label_response);

                    if let Some(cursor) = ui.visuals().interact_cursor {
                        if response.hovered() {
                            ui.ctx().set_cursor_icon(cursor);
                        }
                    }

                    response
                })
                .inner
            }))
        } else {
            None
        };

        ui.put(header_rect, |ui: &mut egui::Ui| {
            ui.vertical_centered_justified(|ui| ui.add(egui::Label::new(label).selectable(false)))
                .inner
        });

        ui.advance_cursor_after_rect(header_rect);

        response
    }

    pub fn show<F, R>(&self, ui: &mut egui::Ui, show_route: F) -> NavResponse<R>
    where
        F: Fn(&mut egui::Ui, &Nav<T>) -> R,
        T: Display + Clone,
    {
        let id = ui.id().with("nav");
        let mut state = State::load(ui.ctx(), id).unwrap_or_default();

        if let Some(resp) = self.header(
            ui,
            self.top().to_string(),
            self.top_n(1).map(|r| r.to_string()),
        ) {
            if resp.clicked() {
                state.action = Some(NavAction::Returning);
            }
        }

        let available_rect = ui.available_rect_before_wrap();

        // We only handle dragging when there is more than 1 route
        if self.route.len() > 1 {
            // Drag contents to transition back.
            // We must do this BEFORE adding content to the `Nav`,
            // or we will steal input from the widgets we contain.
            let content_response = ui.interact(available_rect, id.with("drag"), Sense::drag());
            if content_response.dragged() {
                state.action = Some(NavAction::Dragging)
            } else if content_response.drag_stopped() {
                // we've stopped dragging, check to see if the offset is
                // passed a certain point, to determine if we should return
                // or animate back

                if state.offset > available_rect.width() / 2.0 {
                    state.action = Some(NavAction::Returning)
                } else {
                    state.action = Some(NavAction::Resetting)
                }
            }
        }

        if let Some(action) = state.action {
            match action {
                NavAction::Dragging => {
                    state.offset += ui.input(|input| input.pointer.delta()).x;
                    if state.offset < 0.0 {
                        state.offset = 0.0;
                    }
                }
                NavAction::Returned => {
                    state.action = None;
                }
                NavAction::Returning => {
                    // We're returning, move the current view off to the
                    // right until the entire view is gone.

                    if let Some(offset) =
                        spring_animate(state.offset, available_rect.width(), false)
                    {
                        ui.ctx().request_repaint();
                        state.offset = offset;
                    } else {
                        state.offset = 0.0;
                        state.action = Some(NavAction::Returned);
                    }
                }
                NavAction::Resetting => {
                    // If we're resetting, animate the current offset
                    // back to the current view

                    if let Some(offset) = spring_animate(state.offset, 0.0, true) {
                        ui.ctx().request_repaint();
                        state.offset = offset;
                    } else {
                        state.action = None
                    }
                }
            }
        }

        state.store(ui.ctx(), id);

        // transition rendering
        // behind transition layer
        let transitioning = state.is_transitioning();
        if transitioning {
            let id = ui.id().with("behind");
            let min_rect = state.popped_min_rect.unwrap_or(available_rect);
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
                LayerId::new(Order::Background, id),
                id,
                available_rect,
                clip,
            );

            // render the previous nav view in the background when
            // transitioning
            let nav = Nav {
                route: self.route[..self.route.len() - 1].to_vec(),
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

            let layer_id = if transitioning {
                // when transitioning, we need a new layer id otherwise the
                // view transform will transform more things than we want
                LayerId::new(Order::Foreground, id)
            } else {
                // if we don't use the same layer id as the ui, then we
                // will have scrollview mousescroll issues due to the way
                // rect_contains_pointer works with overlapping layers
                ui.layer_id()
            };

            let mut ui = egui::Ui::new(
                ui.ctx().clone(),
                layer_id,
                id,
                available_rect,
                available_rect,
            );

            let inner = if let Some(NavAction::Returned) = state.action {
                // to avoid a flicker, render the popped route when we
                // are in the returned state
                let nav = Nav {
                    route: self.route[..self.route.len() - 1].to_vec(),
                    ..*self
                };
                show_route(&mut ui, &nav)
            } else {
                show_route(&mut ui, self)
            };

            if state.offset != 0.0 {
                ui.ctx().transform_layer_shapes(
                    ui.layer_id(),
                    egui::emath::TSTransform::from_translation(Vec2::new(state.offset, 0.0)),
                );
            }

            NavResponse {
                inner,
                action: state.action,
            }
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

fn spring_animate(offset: f32, target: f32, left: bool) -> Option<f32> {
    let abs_offset = (offset - target).abs();
    if abs_offset > 0.0 {
        let sgn = (offset - target).signum();
        let amt = springy(abs_offset);
        let adj = amt * (if left { -1.0 } else { 1.0 });
        let adjusted = offset + adj;

        // if adjusting will flip a sign, then just set to 0
        if (offset - adj - target).signum() != sgn {
            None
        } else {
            Some(adjusted)
        }
    } else {
        // we've reset, we're not in any specific state anymore
        None
    }
}
