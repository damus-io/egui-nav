use drag::Drag;
use egui::{emath::TSTransform, vec2, LayerId, Order, Rect, Vec2};

mod default_ui;
mod drag;
mod popup_sheet;
mod ui;
mod util;

pub use default_ui::{DefaultNavTitle, DefaultTitleResponse};
pub use drag::DragDirection;
pub use popup_sheet::{Percent, PopupResponse, PopupSheet};
pub use ui::NavUiType;

use crate::drag::{drag_delta, DragAngle};

pub struct Nav<'a, Route: Clone> {
    id_source: Option<egui::Id>,
    route: &'a [Route],
    navigating: bool,
    returning: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReturnType {
    Drag,
    Click,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NavAction {
    /// We're returning to the previous view
    Returning(ReturnType),

    /// We released the drag, but not far enough to actually return
    Resetting,

    /// We're dragging the view. We're not making a return decision yet
    Dragging,

    /// We've returning to the previous view. Pop your route!
    Returned(ReturnType),

    /// We've navigating to the next view.
    Navigating,

    /// We're finished navigating, push the route!
    Navigated,
}

impl NavAction {
    fn is_transitioning(&self) -> bool {
        match self {
            NavAction::Returning(_) => true,
            NavAction::Resetting => true,
            NavAction::Dragging => true,
            NavAction::Returned(_) => false,
            NavAction::Navigated => false,
            NavAction::Navigating => true,
        }
    }

    fn handle(
        self,
        ui: &mut egui::Ui,
        state: &mut State,
        drag_direction: DragDirection,
        navigated_offset: f32,
        returned_offset: f32,
    ) {
        match self {
            NavAction::Dragging => {
                state.offset += drag_delta(ui, drag_direction);
                if navigated_offset < returned_offset {
                    if state.offset < navigated_offset {
                        // we are outside the navigated boundary
                        state.offset = navigated_offset;
                    }

                    if state.offset > returned_offset {
                        // we are outside the returned boundary
                        state.offset = returned_offset;
                    }
                    return;
                }

                if navigated_offset > returned_offset {
                    if state.offset > navigated_offset {
                        // we are outside the navigated boundary
                        state.offset = navigated_offset;
                    }
                    if state.offset < returned_offset {
                        // we are outside the returned boundary
                        state.offset = returned_offset;
                    }
                    return;
                }
            }
            NavAction::Returned(_) => {
                state.action = None;
            }
            NavAction::Navigated => {
                state.action = None;
            }
            NavAction::Navigating => {
                let left = state.offset > navigated_offset;
                if let Some(offset) = spring_animate(state.offset, navigated_offset, left) {
                    ui.ctx().request_repaint();
                    state.offset = offset;
                } else {
                    state.action = Some(NavAction::Navigated);
                }
            }
            NavAction::Returning(return_type) => {
                // We're returning, move the current view off to the
                // returned_offset until the entire view is gone.

                let left = state.offset > returned_offset;
                if let Some(offset) = spring_animate(state.offset, returned_offset, left) {
                    ui.ctx().request_repaint();
                    state.offset = offset;
                } else {
                    state.offset = returned_offset;
                    state.action = Some(NavAction::Returned(return_type));
                }
            }
            NavAction::Resetting => {
                // If we're resetting, animate the current offset
                // back to the current view

                let left = state.offset > navigated_offset;
                if let Some(offset) = spring_animate(state.offset, navigated_offset, left) {
                    ui.ctx().request_repaint();
                    state.offset = offset;
                } else {
                    state.action = None
                }
            }
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
        self.action.is_some_and(|s| s.is_transitioning())
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
    pub response: R,
    pub title_response: R,
    pub action: Option<NavAction>,
    pub can_take_drag_from: Vec<egui::Id>,
}

impl<'a, Route: Clone> Nav<'a, Route> {
    pub fn new(route: &'a [Route]) -> Self {
        // precondition: we must have at least one route. this simplifies
        // the rest of the control, and it's easy to catchbb
        assert!(!route.is_empty(), "Nav routes cannot be empty");
        let navigating = false;
        let returning = false;
        let id_source = None;

        Nav {
            id_source,
            navigating,
            returning,
            route,
        }
    }

    pub fn id_source(mut self, id: egui::Id) -> Self {
        self.id_source = Some(id);
        self
    }

    /// Call this when you have just pushed a new value to your route and
    /// you want to animate to this new view
    pub fn navigating(mut self, navigating: bool) -> Self {
        self.navigating = navigating;
        self
    }

    /// Call this when you have just invoked an action to return to the
    /// previous view
    pub fn returning(mut self, returning: bool) -> Self {
        self.returning = returning;
        self
    }

    fn id(&self, ui: &egui::Ui) -> egui::Id {
        ui.id().with(("nav", self.id_source))
    }

    pub fn drag_id(&self, ui: &egui::Ui) -> egui::Id {
        self.id(ui).with("drag")
    }

    pub fn routes(&self) -> &[Route] {
        self.route
    }

    /// Nav guarantees there is at least one route element
    pub fn top(&self) -> &Route {
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
    pub fn top_n(&self, n: usize) -> Option<&Route> {
        util::arr_top_n(self.route, n)
    }

    pub fn show<F, R>(&self, ui: &mut egui::Ui, show_route: F) -> NavResponse<R>
    where
        F: Fn(&mut egui::Ui, NavUiType, &Nav<Route>) -> RouteResponse<R>,
    {
        let mut show_route = show_route;
        self.show_internal(ui, &mut show_route)
    }

    pub fn show_mut<F, R>(&self, ui: &mut egui::Ui, mut show_route: F) -> NavResponse<R>
    where
        F: FnMut(&mut egui::Ui, NavUiType, &Nav<Route>) -> RouteResponse<R>,
    {
        self.show_internal(ui, &mut show_route)
    }

    fn show_internal<F, R>(&self, ui: &mut egui::Ui, show_route: &mut F) -> NavResponse<R>
    where
        F: FnMut(&mut egui::Ui, NavUiType, &Nav<Route>) -> RouteResponse<R>,
    {
        let id = self.id(ui);
        let mut state = State::load(ui.ctx(), id).unwrap_or_default();

        let drag_rect = ui.available_rect_before_wrap();

        let title_response = show_route(ui, NavUiType::Title, self).response;
        let available_rect = ui.available_rect_before_wrap();

        // transition rendering
        // behind transition layer
        let transitioning = state.is_transitioning();
        if transitioning {
            let x_translate_amt = {
                let min_rect = state.popped_min_rect.unwrap_or(available_rect);
                let initial_shift = -min_rect.width() * 0.1;
                let mut amt = initial_shift + springy(state.offset);
                if amt > 0.0 {
                    amt = 0.0;
                }

                amt
            };

            let clip = Rect::from_min_size(
                available_rect.min + egui::vec2(-x_translate_amt, 0.0),
                vec2(state.offset, available_rect.max.y),
            );

            let translate_vec = egui::vec2(x_translate_amt, 0.0);
            let bg_nav = Nav {
                route: &self.route[..self.route.len() - 1],
                ..*self
            };

            let strength = 50.0; // fade strength (max is 255)
            let alpha = ((1.0 - (state.offset / available_rect.width())) * strength) as u8;
            let bg_resp = render_bg(
                ui,
                Some(translate_vec),
                clip,
                available_rect,
                Some(alpha),
                |ui| show_route(ui, NavUiType::Body, &bg_nav).can_take_drag_from,
            );

            state.popped_min_rect = Some(bg_resp.rect);
        };

        // foreground layer
        let fg_resp = {
            let clip = Rect::from_min_size(
                available_rect.min,
                vec2(
                    available_rect.max.x - available_rect.min.x - state.offset,
                    available_rect.max.y,
                ),
            );

            let layer_id = if transitioning {
                // when transitioning, we need a new layer id otherwise the
                // view transform will transform more things than we want
                LayerId::new(Order::Foreground, ui.id().with("fg"))
            } else {
                // if we don't use the same layer id as the ui, then we
                // will have scrollview MouseWheel scroll issues due to
                // the way rect_contains_pointer works with overlapping
                // layers
                ui.layer_id()
            };
            let response = render_fg(
                ui,
                ui.id(), // this must be ui.id() to not break scroll positions
                layer_id,
                Some(Vec2::new(state.offset, 0.0)),
                clip,
                available_rect,
                |ui| show_route(ui, NavUiType::Body, self),
            );
            response
        };

        let ids_to_expose = if self.routes().len() > 1 {
            Vec::new()
        } else {
            fg_resp.can_take_drag_from.clone()
        };

        // We only handle dragging when there is more than 1 route
        if self.route.len() > 1 {
            let content_rect = ui.available_rect_before_wrap();
            let mut cur_drag = Drag::new(
                self.drag_id(ui),
                DragDirection::LeftToRight,
                drag_rect,
                state.offset,
                content_rect.width() / 4.0,
                DragAngle::Balanced,
            );
            if let Some(action) = cur_drag.handle(ui, fg_resp.can_take_drag_from) {
                let nav_action = match action {
                    crate::drag::DragAction::Dragging => NavAction::Dragging,
                    crate::drag::DragAction::DragReleased { threshold_met } => {
                        if threshold_met {
                            NavAction::Returning(crate::ReturnType::Drag)
                        } else {
                            NavAction::Resetting
                        }
                    }
                    crate::drag::DragAction::DragUnrelated => NavAction::Resetting,
                };
                state.action = Some(nav_action);
            }
        }

        // This should probably override other actions?
        if self.navigating {
            if state.action != Some(NavAction::Navigating) {
                state.offset = available_rect.width();
                state.action = Some(NavAction::Navigating);
            }
        } else if self.returning && !matches!(state.action, Some(NavAction::Returning(_))) {
            state.action = Some(NavAction::Returning(ReturnType::Click));
        }

        if let Some(action) = state.action {
            action.handle(
                ui,
                &mut state,
                DragDirection::LeftToRight,
                0.0,
                available_rect.width(),
            );
        }
        if matches!(state.action, Some(NavAction::Returned(_))) {
            state.offset = 0.0;
        }

        state.store(ui.ctx(), id);

        NavResponse {
            response: fg_resp.response,
            title_response,
            action: state.action,
            can_take_drag_from: ids_to_expose,
        }
    }
}

fn springy(offset: f32) -> f32 {
    (offset.abs() * 0.3).max(0.2)
}

fn spring_animate(offset: f32, target: f32, left: bool) -> Option<f32> {
    // nothing left to animate, user released drag beyond target
    if (left && offset <= target) || (!left && offset >= target) {
        return None;
    }

    let abs_offset = (offset - target).abs();
    if abs_offset > 0.1 {
        // need some margin of error
        // some margin of error is needed
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

pub(crate) fn render_bg(
    ui: &mut egui::Ui,
    translate_vec: Option<egui::Vec2>, // whether to translate the rendered route
    clip: egui::Rect,                  // rect that should be clipped
    available_rect: egui::Rect,        // rect of viewing area
    alpha: Option<u8>,
    mut render_route: impl FnMut(&mut egui::Ui) -> Vec<egui::Id>,
) -> RenderBgResponse {
    let id = ui.id();

    let layer_id = LayerId::new(Order::Background, id);
    let mut ui = egui::Ui::new(
        ui.ctx().clone(),
        id,
        egui::UiBuilder::new()
            .layer_id(layer_id)
            .max_rect(available_rect),
    );
    ui.set_clip_rect(clip);

    let can_take_drag_from = render_route(&mut ui);

    let res = ui.min_rect();

    if let Some(alpha) = alpha {
        let fade_color = egui::Color32::from_black_alpha(alpha);

        ui.painter()
            .rect_filled(clip, egui::CornerRadius::default(), fade_color);
    }

    let Some(translate_vec) = translate_vec else {
        return RenderBgResponse {
            rect: res,
            can_take_drag_from,
        };
    };

    if translate_vec == Vec2::ZERO {
        return RenderBgResponse {
            rect: res,
            can_take_drag_from,
        };
    }

    ui.ctx()
        .transform_layer_shapes(ui.layer_id(), TSTransform::from_translation(translate_vec));

    return RenderBgResponse {
        rect: res,
        can_take_drag_from,
    };
}

struct RenderBgResponse {
    rect: egui::Rect,
    can_take_drag_from: Vec<egui::Id>,
}

pub(crate) fn render_fg<R>(
    ui: &mut egui::Ui,
    id: egui::Id,
    layer_id: LayerId,
    translate_vec: Option<egui::Vec2>, // whether to translate the rendered route
    clip: egui::Rect,
    available_rect: egui::Rect,
    mut render_route: impl FnMut(&mut egui::Ui) -> RouteResponse<R>,
) -> RouteResponse<R> {
    let mut ui = egui::Ui::new(
        ui.ctx().clone(),
        id,
        egui::UiBuilder::new()
            .layer_id(layer_id)
            .max_rect(available_rect),
    );
    ui.set_clip_rect(clip);

    let res = render_route(&mut ui);

    let Some(translate_vec) = translate_vec else {
        return res;
    };

    if translate_vec == Vec2::ZERO {
        return res;
    }

    ui.ctx().transform_layer_shapes(
        ui.layer_id(),
        egui::emath::TSTransform::from_translation(translate_vec),
    );

    res
}

pub struct RouteResponse<R> {
    pub response: R,
    pub can_take_drag_from: Vec<egui::Id>,
}
