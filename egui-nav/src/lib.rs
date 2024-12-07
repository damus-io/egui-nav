use egui::{emath::TSTransform, vec2, LayerId, Order, Rect, Sense, Vec2};

mod default_ui;
mod router;
mod ui;
mod util;

use tracing::debug;

pub use default_ui::{DefaultNavTitle, DefaultTitleResponse};
pub use router::{AsRoutes, HasRouter, Router};
pub use ui::NavUiType;

pub struct Nav<'a, Route: AsRoutes, Rtr: HasRouter<Route>> {
    id_source: Option<egui::Id>,
    router: &'a mut Rtr,
    popped: u32,
    route_type: std::marker::PhantomData<Route>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NavAction {
    /// We're returning to the previous view
    Returning,

    /// We released the drag, but not far enough to actually return
    Resetting,

    /// We're dragging the view. We're not making a return decision yet
    Dragging,

    /// We've returning to the previous view. Pop your route!
    Returned,

    /// We've navigating to the next view.
    Navigating,

    /// We're finished navigating, push the route!
    Navigated,
}

impl NavAction {
    fn is_transitioning(&self) -> bool {
        match self {
            NavAction::Returning => true,
            NavAction::Resetting => true,
            NavAction::Dragging => true,
            NavAction::Returned => false,
            NavAction::Navigated => false,
            NavAction::Navigating => true,
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
    pub response: R,
    pub title_response: R,
    pub action: Option<NavAction>,
}

impl<'a, Routes, Rtr> Nav<'a, Routes, Rtr>
where
    //Routes: AsRoutes<Route = Routes>,
    Routes: AsRoutes,
    Rtr: HasRouter<Routes>,
{
    pub fn new(router: &'a mut Rtr) -> Nav<'a, Routes, Rtr> {
        // precondition: we must have at least one route. this simplifies
        // the rest of the control, and it's easy to catchbb
        assert!(
            !router.get_router().routes().is_empty(),
            "Nav routes cannot be empty"
        );
        let id_source = None;
        let popped = 0;
        let route_type = std::marker::PhantomData {};

        Nav {
            id_source,
            router,
            popped,
            route_type,
        }
    }

    pub fn id_source(mut self, id: egui::Id) -> Self {
        self.id_source = Some(id);
        self
    }

    pub fn context(&mut self) -> &mut Rtr {
        self.router
    }

    fn router(&mut self) -> &mut Router<Routes> {
        self.context().get_router()
    }

    pub fn routes(&mut self) -> &[Routes::Route] {
        let popn = self.popped as usize;
        let len = self.router().routes().len();
        &self.router().routes()[..len - popn]
    }

    pub fn top(&mut self) -> &Routes::Route {
        let popn = self.popped as usize;
        self.router().top_n(popn).expect("pop ok")
    }

    fn virtual_pop(&mut self) {
        self.popped += 1;
    }

    fn virtual_unpop(&mut self) {
        self.popped -= 1;
    }

    pub fn show<F, R>(&mut self, ui: &mut egui::Ui, show_route: F) -> NavResponse<R>
    where
        F: Fn(&mut egui::Ui, NavUiType, &mut Self) -> R,
    {
        let mut show_route = show_route;
        self.show_internal(ui, &mut show_route)
    }

    pub fn show_mut<F, R>(&mut self, ui: &mut egui::Ui, mut show_route: F) -> NavResponse<R>
    where
        F: FnMut(&mut egui::Ui, NavUiType, &mut Self) -> R,
    {
        self.show_internal(ui, &mut show_route)
    }

    fn show_internal<F, R>(&mut self, ui: &mut egui::Ui, show_route: &mut F) -> NavResponse<R>
    where
        F: FnMut(&mut egui::Ui, NavUiType, &mut Self) -> R,
    {
        let id = ui.id().with(("nav", self.id_source));
        let mut state = State::load(ui.ctx(), id).unwrap_or_default();

        // We only handle dragging when there is more than 1 route
        if self.routes().len() > 1 {
            // Drag contents to transition back.
            // We must do this BEFORE adding content to the `Nav`,
            // or we will steal input from the widgets we contain.
            let available_rect = ui.available_rect_before_wrap();
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

        let title_response = show_route(ui, NavUiType::Title, self);

        let available_rect = ui.available_rect_before_wrap();

        // This should probably override other actions?
        if self.router().is_navigating() {
            if state.action != Some(NavAction::Navigating) {
                state.offset = available_rect.width();
                state.action = Some(NavAction::Navigating);
            }
        } else if self.router().is_returning() && state.action != Some(NavAction::Returning) {
            state.action = Some(NavAction::Returning);
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
                NavAction::Navigated => {
                    state.action = None;
                }
                NavAction::Navigating => {
                    debug!("offset {}", state.offset);
                    if let Some(offset) = spring_animate(state.offset, 0.0, true) {
                        ui.ctx().request_repaint();
                        state.offset = offset;
                    } else {
                        debug!("navigated, setting navigating to false");
                        state.action = Some(NavAction::Navigated);
                    }
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
            let id = ui.id().with("bg");
            let min_rect = state.popped_min_rect.unwrap_or(available_rect);
            let initial_shift = -min_rect.width() * 0.1;
            let mut amt = initial_shift + springy(state.offset);
            if amt > 0.0 {
                amt = 0.0;
            }

            //let clip_width = state.offset.max(available_rect.width());
            let clip = Rect::from_min_size(
                available_rect.min + egui::vec2(-amt, 0.0),
                vec2(state.offset, available_rect.max.y),
            );

            let layer_id = LayerId::new(Order::Background, id);
            let mut ui = egui::Ui::new(
                ui.ctx().clone(),
                layer_id,
                ui.id(),
                egui::UiBuilder::new().max_rect(available_rect),
            );
            ui.set_clip_rect(clip);

            // render the previous nav view in the background when
            // transitioning
            self.virtual_pop();
            let _r = show_route(&mut ui, NavUiType::Body, self);
            self.virtual_unpop();

            state.popped_min_rect = Some(ui.min_rect());

            let strength = 50.0; // fade strength (max is 255)
            let alpha = ((1.0 - (state.offset / available_rect.width())) * strength) as u8;
            let fade_color = egui::Color32::from_black_alpha(alpha);

            ui.painter()
                .rect_filled(clip, egui::Rounding::default(), fade_color);

            if amt < 0.0 {
                ui.ctx().transform_layer_shapes(
                    ui.layer_id(),
                    TSTransform::from_translation(Vec2::new(amt, 0.0)),
                );
            }
        }

        // foreground layer
        {
            let id = ui.id().with("fg");

            let layer_id = if transitioning {
                // when transitioning, we need a new layer id otherwise the
                // view transform will transform more things than we want
                LayerId::new(Order::Foreground, id)
            } else {
                // if we don't use the same layer id as the ui, then we
                // will have scrollview MouseWheel scroll issues due to
                // the way rect_contains_pointer works with overlapping
                // layers
                ui.layer_id()
            };

            let clip = Rect::from_min_size(
                available_rect.min,
                vec2(
                    available_rect.max.x - available_rect.min.x - state.offset,
                    available_rect.max.y,
                ),
            );

            let mut ui = egui::Ui::new(
                ui.ctx().clone(),
                layer_id,
                ui.id(),
                egui::UiBuilder::new().max_rect(available_rect),
            );
            ui.set_clip_rect(clip);

            let response = if let Some(NavAction::Returned) = state.action {
                // to avoid a flicker, render the popped route when we
                // are in the returned state
                self.virtual_pop();
                let r = show_route(&mut ui, NavUiType::Body, self);
                self.virtual_unpop();
                r
            } else {
                show_route(&mut ui, NavUiType::Body, self)
            };

            if state.offset != 0.0 {
                ui.ctx().transform_layer_shapes(
                    ui.layer_id(),
                    egui::emath::TSTransform::from_translation(Vec2::new(state.offset, 0.0)),
                );
            }

            // handle these after rendering to avoid ui popping effects
            if let Some(action) = state.action {
                match action {
                    NavAction::Returned => {
                        debug!("returned, popping route");
                        self.router().pop();
                        state.action = None;
                    }
                    NavAction::Navigated => {
                        debug!("navigated, settings navigating to false");
                        self.router().set_navigating(false);
                        state.action = None;
                    }
                    _ => {}
                }
            }

            NavResponse {
                title_response,
                response,
                action: state.action,
            }
        }
    }
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
