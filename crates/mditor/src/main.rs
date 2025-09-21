mod app;
mod custom_widget;
mod helper;
mod model;

use app::App;
use iced::Font;

pub fn main() -> iced::Result {
    iced::application("Mditor", App::update, App::view)
        .theme(App::theme)
        .default_font(Font::MONOSPACE)
        .run_with(App::new)
}
