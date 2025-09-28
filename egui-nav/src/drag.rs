use egui::Pos2;

use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct DragDirection: u8 {
        const LeftToRight = 0b0001;
        const RightToLeft = 0b0010;
        const Vertical = 0b0100;
    }
}

pub(crate) struct Drag {
    pub(crate) id: egui::Id,
    content_rect: egui::Rect,
    direction: DragDirection,
    offset_from_rest: f32,
    threshold: f32, // if offset_from_rest is ABOVE threshold when drag is released, that means the drag MEETS the threshold
    angle: DragAngle,
}

impl Drag {
    pub(crate) fn new(
        id: egui::Id,
        direction: DragDirection,
        content_rect: egui::Rect,
        offset_from_rest: f32,
        threshold: f32,
        angle: DragAngle,
    ) -> Self {
        Drag {
            id,
            content_rect,
            direction,
            offset_from_rest,
            threshold,
            angle,
        }
    }

    pub(crate) fn handle(
        &mut self,
        ui: &mut egui::Ui,
        can_take_from: Vec<egui::Id>,
    ) -> Option<DragAction> {
        if ui.ctx().dragged_id().is_none()
            && ui.ctx().input(|i| {
                let pointer = &i.pointer;
                pointer.is_decidedly_dragging()
                    && pointer.primary_down()
                    && pointer
                        .press_origin()
                        .is_some_and(|origin| self.content_rect.contains(origin))
            })
        {
            ui.ctx().set_dragged_id(self.id);
        }

        let mut resp = None;
        if let Some(dragged_id) = ui.ctx().dragged_id() {
            let can_take_drag_id = can_take_from.contains(&dragged_id);
            if can_take_drag_id || dragged_id == self.id {
                if self.handle_dragging(ui, dragged_id, can_take_drag_id) {
                    resp = Some(DragAction::Dragging)
                }
            } else {
                if self.offset_from_rest > 0.0 {
                    resp = Some(DragAction::DragUnrelated);
                }
            }
        };

        if let Some(dragged_id) = ui.ctx().dragged_id() {
            if dragged_id == self.id && !ui.ctx().input(|i| i.pointer.primary_down()) {
                ui.ctx().stop_dragging();
            }
        }

        if let Some(stopped_id) = ui.ctx().drag_stopped_id() {
            if stopped_id == self.id {
                if let Some(state) = get_state(ui.ctx()) {
                    resp = match self.get_direction(&state) {
                        HandleDragDirection::CorrectDirection => Some(DragAction::DragReleased {
                            threshold_met: self.offset_from_rest >= self.threshold,
                        }),
                        HandleDragDirection::DirectionInconclusive => {
                            Some(DragAction::DragUnrelated)
                        }
                    };
                }
                remove_state(ui.ctx());
            }
        };

        resp
    }

    /// returns whether we are dragging in the correct direction
    fn handle_dragging(
        &mut self,
        ui: &mut egui::Ui,
        dragged_id: egui::Id,
        set_dragged: bool,
    ) -> bool {
        let ctx = ui.ctx();

        let vals = ui
            .ctx()
            .input(|i| Some((i.pointer.press_origin()?, i.pointer.latest_pos()?)));

        let Some((origin, latest)) = vals else {
            return false;
        };

        if !self.content_rect.contains(origin) {
            return false;
        }

        let Some(cur_direction) = cur_direction(origin, latest, self.angle) else {
            return false;
        };

        if !self.direction.contains(cur_direction) {
            return false;
        }

        self.insert_state(
            ui.ctx(),
            DragState {
                start_pos: origin,
                cur_direction,
            },
        );

        let _ = ui.interact(self.content_rect, self.id, egui::Sense::drag());

        if set_dragged && dragged_id != self.id {
            ctx.set_dragged_id(self.id);
        }

        true
    }

    fn get_direction(&self, state: &DragState) -> HandleDragDirection {
        if !self.content_rect.contains(state.start_pos) {
            // the start position isn't in the content rect, the interaction doesn't pertain to this widget
            return HandleDragDirection::DirectionInconclusive;
        }

        if self.direction.contains(state.cur_direction) {
            HandleDragDirection::CorrectDirection
        } else {
            HandleDragDirection::DirectionInconclusive
        }
    }

    fn insert_state(&mut self, ctx: &egui::Context, state: DragState) {
        ctx.data_mut(|d| d.insert_temp(state_id(), state));
    }
}

#[derive(Debug)]
enum HandleDragDirection {
    CorrectDirection,
    DirectionInconclusive,
}

#[derive(Debug, Clone)]
pub enum DragAction {
    Dragging,
    DragReleased { threshold_met: bool },
    DragUnrelated,
}

fn state_id() -> egui::Id {
    egui::Id::new("nav-drag-state")
}

pub fn get_state(ctx: &egui::Context) -> Option<DragState> {
    let id = state_id();
    ctx.data(|d| d.get_temp(id))
}

fn remove_state(ctx: &egui::Context) {
    ctx.data_mut(|d| d.remove::<DragState>(state_id()));
}

#[derive(Clone, Debug)]
pub struct DragState {
    pub(crate) start_pos: Pos2,
    pub(crate) cur_direction: DragDirection,
}

fn cur_direction(start: Pos2, cur_pos: Pos2, angle: DragAngle) -> Option<DragDirection> {
    let dx = start.x - cur_pos.x;
    let dy = start.y - cur_pos.y;

    let min_and = 8.0;

    // at least one value should be larger than `min_and`
    if dx.abs() < min_and && dy.abs() < min_and {
        return None;
    }

    let is_vertical = match angle {
        DragAngle::Balanced => dy.abs() > dx.abs(),
        DragAngle::VerticalNTimesEasier(n) => dy.abs() * n as f32 > dx.abs(), // we want to make vertical extremely easy to hit
    };

    let resp = Some(if is_vertical {
        DragDirection::Vertical
    } else if dx >= 0.0 {
        DragDirection::RightToLeft
    } else {
        DragDirection::LeftToRight
    });

    resp
}

#[derive(Clone, Copy, Debug)]
pub enum DragAngle {
    Balanced,
    VerticalNTimesEasier(u8),
}

pub(crate) fn drag_delta(ui: &mut egui::Ui, direction: DragDirection) -> f32 {
    let delta = ui.input(|input| input.pointer.delta());
    if direction.intersects(DragDirection::LeftToRight | DragDirection::RightToLeft) {
        delta.x
    } else if direction.contains(DragDirection::Vertical) {
        delta.y
    } else {
        0.0
    }
}
