use iced::{
    Font, Point, Renderer, Size, Theme,
    mouse::Cursor,
    widget::canvas::{self, Cache},
};
use text_buffer::TextBuffer;

#[derive(Debug, Default)]
pub struct EditorCanvasCache {
    cache: Cache,
}

pub struct EditorCanvas<'a> {
    buffer: &'a TextBuffer,
    font: Font,
    font_size: f32,
    spacing: f32,
}

impl<'a> EditorCanvas<'a> {
    pub fn new(buffer: &'a TextBuffer, font: Font, font_size: f32, spacing: f32) -> Self {
        EditorCanvas {
            buffer,
            font,
            font_size,
            spacing,
        }
    }
}

impl<'a, Message> canvas::Program<Message> for EditorCanvas<'a> {
    type State = EditorCanvasCache;

    fn draw(
        &self,
        state: &Self::State,
        _renderer: &Renderer,
        _theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let geometry = state.cache.draw(_renderer, bounds.size(), |frame| {
            let lines = self.buffer.get_lines_content();
            let line_count = self.buffer.get_line_count();

            let line_height = self.font_size * self.spacing;

            let gutter_pad_left = 24.0;
            let gutter_pad_right = 36.0;

            let mono_char_factor = 0.62_f32;
            let char_width = self.font_size * mono_char_factor;

            let mut n = line_count.max(1);
            let mut digit_count = 0usize;
            while n > 0 {
                digit_count += 1;
                n /= 10;
            }

            let gutter_width =
                gutter_pad_left + (digit_count as f32) * char_width + gutter_pad_right;
            let number_color = iced::Color::from_rgba8(180, 180, 180, 1.0);
            let text_color = iced::Color::from_rgba8(255, 255, 255, 1.0);

            let mut y = self.font_size;

            for (i, line) in lines.iter().enumerate() {
                if y > bounds.height + line_height {
                    break;
                }

                // Right-align the number using the monospace width approximation
                let number_str = (i + 1).to_string();
                let number_len = number_str.len() as f32;
                let number_width = number_len * char_width;
                let number_x = gutter_width - gutter_pad_right - number_width;

                frame.fill_text(canvas::Text {
                    content: number_str,
                    font: self.font,
                    size: self.font_size.into(),
                    color: number_color,
                    position: Point::new(number_x, y),
                    ..Default::default()
                });

                // Draw line content shifted by gutter width
                let x_text = gutter_width;

                frame.fill_text(canvas::Text {
                    color: text_color,
                    content: line.clone(),
                    font: self.font,
                    size: self.font_size.into(),
                    position: Point::new(x_text, y),
                    ..Default::default()
                });

                y += line_height;
            }
        });
        vec![geometry]
    }

    fn update(
        &self,
        _state: &mut Self::State,
        _event: canvas::Event,
        _bounds: iced::Rectangle,
        _cursor: Cursor,
    ) -> (canvas::event::Status, Option<Message>) {
        _state.cache.clear();
        (canvas::event::Status::Ignored, None)
    }
}
