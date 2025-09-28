use crate::{render_bg, render_fg, Drag, NavAction, NavUiType, RouteResponse, State};

pub struct PopupSheet<'a, Route: Clone> {
    id_source: Option<egui::Id>,
    bg_route: &'a Route,
    fg_route: &'a Route,
    split_percentage: Percent,
    navigating: bool,
    returning: bool,
}

#[derive(Copy, Clone, Debug)]
pub struct Percent(u8); // 0–100

impl Percent {
    pub fn new(p: u8) -> Option<Self> {
        if p > 100 {
            return None;
        }
        Some(Self(p))
    }

    #[inline]
    pub fn of(&self, val: f32) -> f32 {
        // Multiply with const factor: percent * 0.01
        val * (self.0 as f32 * 0.01)
    }
}

pub struct PopupResponse<R> {
    pub response: R,
    pub action: Option<NavAction>,
}

impl<'a, Route: Clone> PopupSheet<'a, Route> {
    pub fn new(bg_route: &'a Route, fg_route: &'a Route) -> Self {
        Self {
            bg_route,
            fg_route,
            split_percentage: Percent(50),
            navigating: false,
            returning: false,
            id_source: None,
        }
    }

    pub fn with_split_percent_from_top(mut self, percent: Percent) -> Self {
        self.split_percentage = percent;
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

    pub fn show<F, R>(&self, ui: &mut egui::Ui, show_route: F) -> PopupResponse<R>
    where
        F: Fn(&mut egui::Ui, NavUiType, &Route) -> R,
    {
        let mut show_route = show_route;

        self.show_internal(ui, &mut show_route)
    }

    pub fn show_mut<F, R>(&self, ui: &mut egui::Ui, mut show_route: F) -> PopupResponse<R>
    where
        F: FnMut(&mut egui::Ui, NavUiType, &Route) -> R,
    {
        self.show_internal(ui, &mut show_route)
    }

    fn show_internal<F, R>(&self, ui: &mut egui::Ui, show_route: &mut F) -> PopupResponse<R>
    where
        F: FnMut(&mut egui::Ui, NavUiType, &Route) -> R,
    {
        let id = ui.id().with(("bottom_sheet", self.id_source));

        let max_height = {
            let rect = ui.available_rect_before_wrap();
            rect.top() + self.split_percentage.of(rect.bottom() - rect.top())
        };
        let mut state = State::load(ui.ctx(), id).unwrap_or(State {
            offset: max_height,
            action: None,
            popped_min_rect: None,
        });

        let (bg_rect, content_rect) = ui
            .available_rect_before_wrap()
            .split_top_bottom_at_y(state.offset);

        let offset_from_rest = state.offset - max_height;
        let drag = Drag::new(
            id,
            crate::DragDirection::Vertical,
            content_rect,
            offset_from_rest,
        );

        if let Some(action) = drag.handle(ui) {
            state.action = Some(action);
        }

        if self.navigating {
            if state.action != Some(NavAction::Navigating) {
                state.offset = content_rect.bottom();
                state.action = Some(NavAction::Navigating);
            }
        } else if self.returning && !matches!(state.action, Some(NavAction::Returning(_))) {
            state.offset = max_height;
            state.action = Some(NavAction::Returning(crate::ReturnType::Click));
        }

        let max_size = content_rect.bottom();
        if let Some(action) = state.action {
            action.handle(
                ui,
                &mut state,
                crate::DragDirection::Vertical,
                max_height,
                max_size,
            );
        }

        let alpha = {
            let t = ((max_size - state.offset) / (max_size)).clamp(0.0, 1.0);
            (t * 255.0).round() as u8
        };

        let bg_resp = render_bg(ui, None, bg_rect, bg_rect, Some(alpha), |ui| {
            show_route(ui, NavUiType::Title, self.bg_route);
            show_route(ui, NavUiType::Body, self.bg_route);
            Vec::new()
        });
        state.popped_min_rect = Some(bg_resp.rect);

        let bg_resp = ui.allocate_rect(bg_rect, egui::Sense::click());

        if bg_resp.clicked() {
            state.action = Some(NavAction::Returning(crate::ReturnType::Click));
        }

        state.store(ui.ctx(), id);

        let response = render_fg(
            ui,
            id.with("fg"),
            egui::LayerId::new(egui::Order::Foreground, id.with("fg")),
            None,
            content_rect,
            content_rect,
            |ui| {
                let r = if matches!(state.action, Some(NavAction::Returned(_))) {
                    show_route(ui, NavUiType::Body, self.bg_route)
                } else {
                    show_route(ui, NavUiType::Body, self.fg_route)
                };

                RouteResponse {
                    response: r,
                    can_take_drag_from: Vec::new(),
                }
            },
        )
        .response;

        PopupResponse {
            response,
            action: state.action,
        }
    }
}
