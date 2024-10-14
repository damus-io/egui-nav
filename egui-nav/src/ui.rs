
/// What part of the nav are we rendering? We opt to using a single
/// callback rendering to avoid borrow issues
pub enum NavUiType {
    Title,
    Body,
}

