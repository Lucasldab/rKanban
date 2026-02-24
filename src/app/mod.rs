#[derive(Default)]
pub struct App {
    pub quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self { quit: false }
    }
}
