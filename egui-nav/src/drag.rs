use egui::Sense;

use crate::{NavAction, ReturnType};

pub(crate) enum DragDirection {
    Horizontal,
    Vertical,
}

pub(crate) struct Drag {
    id: egui::Id,
    content_rect: egui::Rect,
    direction: DragDirection,
    offset_from_rest: f32,
}

impl Drag {
    pub(crate) fn new(
        id: egui::Id,
        direction: DragDirection,
        content_rect: egui::Rect,
        offset_from_rest: f32,
    ) -> Self {
        Drag {
            id,
            content_rect,
            direction,
            offset_from_rest,
        }
    }

    fn content_size(&self) -> f32 {
        match self.direction {
            DragDirection::Horizontal => self.content_rect.width(),
            DragDirection::Vertical => self.content_rect.height(),
        }
    }

    pub(crate) fn handle(self, ui: &mut egui::Ui) -> Option<NavAction> {
        // Drag contents to transition back.
        // We must do this BEFORE adding content to the `Nav`,
        // or we will steal input from the widgets we contain.
        let content_response = ui.interact(self.content_rect, self.id, Sense::drag());

        if content_response.dragged() {
            return Some(NavAction::Dragging);
        } else if content_response.drag_stopped() || ui.ctx().drag_stopped_id().is_some() {
            // we've stopped dragging, check to see if the offset is
            // passed a certain point, to determine if we should return
            // or animate back

            if self.offset_from_rest > self.content_size() / 4.0 {
                return Some(NavAction::Returning(ReturnType::Drag));
            } else {
                return Some(NavAction::Resetting);
            }
        }

        None
    }
}

pub(crate) fn drag_delta(ui: &mut egui::Ui, direction: DragDirection) -> f32 {
    match direction {
        DragDirection::Horizontal => ui.input(|input| input.pointer.delta()).x,
        DragDirection::Vertical => ui.input(|input| input.pointer.delta()).y,
    }
}
