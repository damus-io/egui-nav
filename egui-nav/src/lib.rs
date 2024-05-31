use core::fmt::Display;

pub struct Nav<'r, T> {
    route: &'r [T],
}

impl<'r, T> Nav<'r, T> {
    /// Nav requires at least one route or it will panic
    pub fn new(route: &'r [T]) -> Self {
        // precondition: we must have at least one route. this simplifies
        // the rest of the control, and it's easy to catchbb
        assert!(route.len() > 0, "Nav routes cannot be empty");
        Nav { route }
    }

    pub fn top_route(&self) -> &'r T {
        &self.route[self.route.len() - 1]
    }

    /// Safer version of new if we're not sure if we will have non-empty routes
    pub fn try_new(route: &'r [T]) -> Option<Self> {
        if route.len() == 0 {
            None
        } else {
            Some(Nav { route })
        }
    }

    pub fn show<F, R>(&self, ui: &mut egui::Ui, show_route: F) -> R
    where
        F: Fn(&mut egui::Ui, &Nav<'_, T>) -> R,
        T: Display,
    {
        let route = self.top_route();
        ui.label(format!("< {}", route));
        show_route(ui, self)
    }
}
