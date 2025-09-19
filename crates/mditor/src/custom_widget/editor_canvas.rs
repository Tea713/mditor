use iced::{
    Font, Point, Renderer, Theme,
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
}

impl<'a> EditorCanvas<'a> {
    pub fn new(buffer: &'a TextBuffer, font: Font, font_size: f32) -> Self {
        EditorCanvas {
            buffer,
            font,
            font_size,
        }
    }
}

impl<'a, Message> canvas::Program<Message> for EditorCanvas<'a> {
    type State = EditorCanvasCache;

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        cursor: Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let geometry = state.cache.draw(renderer, bounds.size(), |frame| {
            let lines = self.buffer.get_lines_content();
            let line_height = self.font_size * 1.4;
            let x = 0.0;
            let mut y = self.font_size;

            for line in lines {
                frame.fill_text(canvas::Text {
                    color: iced::Color {
                        r: 255.0,
                        g: 255.0,
                        b: 255.0,
                        a: 1.0,
                    },
                    content: line,
                    font: self.font,
                    size: self.font_size.into(),
                    position: Point::new(x, y),
                    ..Default::default()
                });

                y += line_height;
                if y > bounds.height + line_height {
                    break;
                }
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
        // TODO: implement real update thingy
        // right now just clear the state which will simply redraw the canvas I think
        _state.cache.clear();
        (canvas::event::Status::Ignored, None)
    }
}
