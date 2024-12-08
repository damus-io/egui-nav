use crate::util;
use egui::{Pos2, Sense, Stroke, Vec2};
use std::fmt::Display;

#[derive(Clone, Copy)]
pub struct DefaultNavTitle {
    stroke: Option<Stroke>,
    chevron_size: Vec2,
    padding: f32,
}

impl Default for DefaultNavTitle {
    fn default() -> Self {
        Self {
            stroke: None,
            chevron_size: Vec2::new(14.0, 20.0),
            padding: 4.0,
        }
    }
}

pub enum DefaultTitleResponse {
    Back,
}

impl DefaultNavTitle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ui<R: Display>(&self, ui: &mut egui::Ui, routes: &[R]) -> Option<DefaultTitleResponse> {
        // default route ui
        let mut header_rect = ui.available_rect_before_wrap();
        header_rect.set_height(self.chevron_size.y + 4.0);

        let back = util::arr_top_n(routes, 1);
        let response = back.map(|back| {
            ui.put(header_rect, |ui: &mut egui::Ui| {
                ui.horizontal_centered(|ui| {
                    let stroke = self
                        .stroke
                        .unwrap_or_else(|| Stroke::new(2.0, ui.visuals().hyperlink_color));

                    let chev_response = chevron(ui, self.padding, self.chevron_size, stroke);

                    let label_response = ui.add(
                        egui::Label::new(back.to_string())
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
            })
        });

        if let Some(resp) = response {
            if resp.clicked() {
                Some(DefaultTitleResponse::Back)
            } else {
                None
            }
        } else {
            None
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
