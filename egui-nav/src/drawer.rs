use egui::{LayerId, Order};

use crate::{
    drag::DragAngle, render_bg, render_fg, Drag, DragDirection, NavAction, RouteResponse, State,
};

pub struct NavDrawer<'a, Route: Clone> {
    id_source: Option<egui::Id>,
    bg_route: &'a Route,
    drawer_route: &'a Route,
    drawer_end_offset: f32,
    navigating: bool,
    returning: bool,
    drawer_focused: bool,
}

impl<'a, Route: Clone> NavDrawer<'a, Route> {
    pub fn new(bg_route: &'a Route, drawer_route: &'a Route) -> Self {
        Self {
            id_source: None,
            bg_route,
            drawer_route,
            drawer_end_offset: 0.0,
            navigating: false,
            returning: false,
            drawer_focused: false,
        }
    }

    pub fn opened_offset(mut self, drawer_end_x: f32) -> Self {
        self.drawer_end_offset = drawer_end_x;
        self
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

    pub fn drawer_focused(mut self, focused: bool) -> Self {
        self.drawer_focused = focused;
        self
    }

    fn id(&self, ui: &egui::Ui) -> egui::Id {
        ui.id().with(("nav-drawer", self.id_source))
    }

    pub fn drag_id(&self, ui: &egui::Ui) -> egui::Id {
        self.id(ui).with("drag")
    }

    pub fn show<F, R>(&self, ui: &mut egui::Ui, show_route: F) -> DrawerResponse<R>
    where
        F: Fn(&mut egui::Ui, &Route) -> RouteResponse<R>,
    {
        let mut show_route = show_route;

        self.show_internal(ui, &mut show_route)
    }

    pub fn show_mut<F, R>(&self, ui: &mut egui::Ui, mut show_route: F) -> DrawerResponse<R>
    where
        F: FnMut(&mut egui::Ui, &Route) -> RouteResponse<R>,
    {
        self.show_internal(ui, &mut show_route)
    }

    fn show_internal<F, R>(&self, ui: &mut egui::Ui, show_route: &mut F) -> DrawerResponse<R>
    where
        F: FnMut(&mut egui::Ui, &Route) -> RouteResponse<R>,
    {
        let id = self.id(ui);
        let mut state = State::load(ui.ctx(), id).unwrap_or_default();

        let rest = 0.0;
        let max = self.drawer_end_offset;

        let (drawer_rect, bg_rect) = ui
            .available_rect_before_wrap()
            .split_left_right_at_x(state.offset);

        let drag_content_rect = ui.available_rect_before_wrap();

        let can_take_drag_from = if state.offset == rest {
            show_route(ui, self.bg_route).can_take_drag_from
        } else {
            let avail_rect = ui.available_rect_before_wrap();
            let alpha = if state.offset <= rest {
                None
            } else {
                let t = ((self.drawer_end_offset - state.offset) / self.drawer_end_offset)
                    .clamp(0.0, 1.0);
                Some(((1.0 - t) * 200.0).round() as u8)
            };

            render_bg(ui, None, bg_rect, avail_rect, alpha, |ui| {
                show_route(ui, self.bg_route).can_take_drag_from
            })
            .can_take_drag_from
        };

        let mut drag = Drag::new(
            self.drag_id(ui),
            if self.drawer_focused {
                DragDirection::all()
            } else {
                DragDirection::LeftToRight
            },
            drag_content_rect,
            if self.drawer_focused {
                (state.offset - self.drawer_end_offset).abs()
            } else {
                state.offset
            },
            0.1,
            if self.drawer_focused {
                DragAngle::Balanced
            } else {
                DragAngle::VerticalNTimesEasier(5)
            },
        );

        if self.navigating {
            if state.action != Some(NavAction::Navigating) {
                state.action = Some(NavAction::Navigating);
            }
        } else if self.returning && !matches!(state.action, Some(NavAction::Returning(_))) {
            state.offset = self.drawer_end_offset;
            state.action = Some(NavAction::Returning(crate::ReturnType::Click));
        }

        's: {
            let Some(action) = drag.handle(ui, can_take_drag_from) else {
                break 's;
            };

            let nav_action = match action.clone() {
                crate::drag::DragAction::Dragging => NavAction::Dragging,
                crate::drag::DragAction::DragReleased { threshold_met } => {
                    if self.drawer_focused {
                        if threshold_met {
                            NavAction::Returning(crate::ReturnType::Drag)
                        } else {
                            NavAction::Resetting
                        }
                    } else {
                        if threshold_met {
                            NavAction::Navigating
                        } else {
                            NavAction::Returning(crate::ReturnType::Drag)
                        }
                    }
                }
                crate::drag::DragAction::DragUnrelated => NavAction::Resetting,
            };
            state.action = Some(nav_action);
        }

        if let Some(action) = state.action {
            action.handle(ui, &mut state, DragDirection::LeftToRight, max, rest);
        }

        if state.offset == rest {
            state.store(ui.ctx(), id);
            return DrawerResponse {
                drawer_response: None,
                action: state.action,
            };
        }

        let bg_resp = ui.allocate_rect(bg_rect, egui::Sense::click());

        if bg_resp.clicked() {
            state.action = Some(NavAction::Returning(crate::ReturnType::Click));
        }

        let drawer_response = Some(
            render_fg(
                ui,
                id.with("fg"),
                LayerId::new(Order::Foreground, id.with("fg")),
                None,
                drawer_rect,
                drawer_rect,
                |ui| show_route(ui, self.drawer_route),
            )
            .response,
        );

        state.store(ui.ctx(), id);

        DrawerResponse {
            drawer_response,
            action: state.action,
        }
    }
}

pub struct DrawerResponse<R> {
    pub drawer_response: Option<R>,
    pub action: Option<NavAction>,
}
