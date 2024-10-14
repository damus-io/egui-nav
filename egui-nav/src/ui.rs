pub trait NavUi<R> {
    type TitleResponse;

    fn title_ui(&self, ui: &mut egui::Ui, routes: &[R]) -> NavUiResponse<Self::TitleResponse>;
}

pub struct NavUiResponse<E> {
    pub title_response: Option<egui::InnerResponse<E>>,
}

impl<E> Default for NavUiResponse<E> {
    fn default() -> Self {
        Self {
            title_response: None,
        }
    }
}

impl<T> NavUiResponse<T> {
    pub fn new(response: egui::Response, title_response: T) -> Self {
        let title_response = Some(egui::InnerResponse::new(title_response, response));
        NavUiResponse { title_response }
    }

    pub fn none() -> Self {
        NavUiResponse {
            title_response: None,
        }
    }
}
