use crate::model::editor_message::EditorMessage;

use iced::{
    Font, Rectangle, Renderer,
    mouse::Cursor,
    widget::canvas::{self, Cache},
};
use text_buffer::TextBuffer;
use unicode_segmentation::UnicodeSegmentation;
// TODOS: figure out how to get factor for any font. Right now just a constant that align with iced's FONT::MONOSPACE
const MONO_CHAR_FACTOR: f32 = 0.585;

#[derive(Debug, Default)]
pub struct EditorCanvasCache {
    cache: std::cell::RefCell<Cache>,
    seen_version: std::cell::Cell<u64>,
}

pub struct EditorCanvas<'a> {
    buffer: &'a TextBuffer,
    font: Font,
    font_size: f32,
    spacing: f32,
    cursor_line: usize,
    cursor_col: usize,
    render_version: u64,
}

impl<'a> EditorCanvas<'a> {
    pub fn new(
        buffer: &'a TextBuffer,
        font: Font,
        font_size: f32,
        spacing: f32,
        cursor_line: usize,
        cursor_col: usize,
        render_version: u64,
    ) -> Self {
        EditorCanvas {
            buffer,
            font,
            font_size,
            spacing,
            cursor_line,
            cursor_col,
            render_version,
        }
    }
}

impl<'a> canvas::Program<crate::model::editor_message::EditorMessage> for EditorCanvas<'a> {
    type State = EditorCanvasCache;

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &iced::Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<iced::widget::canvas::Geometry<iced::Renderer>> {
        let char_width = (self.font_size * MONO_CHAR_FACTOR).max(1.0);

        // Invalidate cache if version changed
        if state.seen_version.get() != self.render_version {
            state.cache.borrow_mut().clear();
            state.seen_version.set(self.render_version);
        }

        let geometry = state
            .cache
            .borrow_mut()
            .draw(renderer, bounds.size(), |frame| {
                let lines = self.buffer.get_lines_content();
                let line_count = self.buffer.get_line_count();

                let line_height = self.font_size * self.spacing;
                let gutter_pad_left = 24.0;
                let gutter_pad_right = 36.0;

                let mut n = line_count.max(1);
                let mut digit_count = 0usize;
                while n > 0 {
                    digit_count += 1;
                    n /= 10;
                }
                let gutter_width =
                    gutter_pad_left + (digit_count as f32) * char_width + gutter_pad_right;

                // Gutter
                let gutter_bg = iced::Color::from_rgba8(39, 40, 34, 1.0);
                frame.fill_rectangle(
                    iced::Point::new(0.0, 0.0),
                    iced::Size::new(gutter_width, bounds.height),
                    gutter_bg,
                );

                let number_color = iced::Color::from_rgba8(180, 180, 180, 1.0);
                let text_color = iced::Color::from_rgba8(255, 255, 255, 1.0);

                let mut y = 0.0;

                for (i, line) in lines.iter().enumerate() {
                    if y > bounds.height + line_height {
                        break;
                    }

                    let number_str = (i + 1).to_string();
                    let number_len = number_str.len() as f32;
                    let number_width = number_len * char_width;
                    let number_x = gutter_width - gutter_pad_right - number_width;

                    frame.fill_text(iced::widget::canvas::Text {
                        content: number_str,
                        font: self.font,
                        size: self.font_size.into(),
                        color: number_color,
                        position: iced::Point::new(number_x, y),
                        ..Default::default()
                    });

                    let x_text = gutter_width;
                    frame.fill_text(iced::widget::canvas::Text {
                        color: text_color,
                        content: line.clone(),
                        font: self.font,
                        size: self.font_size.into(),
                        position: iced::Point::new(x_text, y),
                        ..Default::default()
                    });

                    y += line_height;
                }

                let caret_line = self.cursor_line as f32;
                let caret_col = self.cursor_col as f32;
                let caret_x = gutter_width + caret_col * char_width;
                let caret_y_top = caret_line * line_height;
                let caret_color = iced::Color::from_rgba8(255, 255, 255, 0.8);
                let caret_width = 1.0;
                frame.fill_rectangle(
                    iced::Point::new(caret_x.floor(), caret_y_top),
                    iced::Size::new(caret_width, line_height),
                    caret_color,
                );
            });

        vec![geometry]
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> (
        canvas::event::Status,
        Option<crate::model::editor_message::EditorMessage>,
    ) {
        use iced::mouse;

        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(p) = cursor.position_in(bounds) {
                    let line_height = self.font_size * self.spacing;
                    let char_width = MONO_CHAR_FACTOR * self.font_size;

                    let mut n = self.buffer.get_line_count().max(1);
                    let mut digit_count = 0usize;
                    while n > 0 {
                        digit_count += 1;
                        n /= 10;
                    }
                    let gutter_width = 24.0 + (digit_count as f32) * char_width + 36.0;

                    let mut line = (p.y / line_height).floor().max(0.0) as usize;
                    let line_count = self.buffer.get_line_count();
                    if line_count > 0 {
                        line = line.min(line_count.saturating_sub(1));
                    } else {
                        line = 0;
                    }
                    let approx_col = ((p.x - gutter_width).max(0.0) / char_width)
                        .round()
                        .max(0.0) as usize;

                    let line_text = self.buffer.get_line_content(line + 1);
                    let grapheme_len = line_text.graphemes(true).count();
                    let column = approx_col.min(grapheme_len);

                    state.cache.borrow_mut().clear();
                    return (
                        canvas::event::Status::Captured,
                        Some(EditorMessage::SetCursor { line, column }),
                    );
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {}
            _ => {}
        }
        (canvas::event::Status::Ignored, None)
    }
}
