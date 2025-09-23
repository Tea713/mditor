use crate::custom_widget::editor_canvas::EditorCanvas;
use crate::model::{editor_message::EditorMessage, error::Error};
use iced::border::Radius;
use iced::keyboard::Key;
use iced::keyboard::key::Named;
use iced::widget::{
    button, canvas, column, container, horizontal_rule, horizontal_space, row, rule, scrollable,
    text, text_input,
};
use iced::{
    Border, Center, Element, Event, Font, Shadow, Subscription, Task, Theme, event, window,
};
use iced::{Length, highlighter};
use std::path::PathBuf;
use text_buffer::{TextBuffer, TextBufferBuilder};
use unicode_segmentation::UnicodeSegmentation;

// TODO: implement size and spacing settings
const FONT_SIZE: f32 = 14.0;
const LINE_SPACING: f32 = 1.4;

pub struct App {
    file: Option<PathBuf>,
    buffer: TextBuffer,
    theme: highlighter::Theme,
    is_loading: bool,
    is_dirty: bool,
    active: bool,
    line: usize,
    col: usize,
    preferred_col: Option<usize>, // preserve horizontal position when moving up/down
    render_version: u64,
    input_value: String,
    input_id: text_input::Id,
}

impl App {
    pub fn new() -> (Self, Task<EditorMessage>) {
        let app = Self {
            file: None,
            buffer: TextBufferBuilder::new().finish(),
            theme: highlighter::Theme::SolarizedDark,
            is_loading: false,
            is_dirty: false,
            active: false,
            line: 0,
            col: 0,
            preferred_col: None,
            render_version: 0,
            input_value: String::new(),
            input_id: text_input::Id::unique(),
        };
        let task = text_input::focus(app.input_id.clone());
        (app, task)
    }

    pub fn update(&mut self, message: EditorMessage) -> Task<EditorMessage> {
        match message {
            EditorMessage::NewFile => {
                if !self.is_loading {
                    self.file = None;
                    self.buffer = TextBufferBuilder::new().finish();
                    self.is_dirty = false;
                    self.render_version = self.render_version.wrapping_add(1);
                }
                Task::none()
            }

            EditorMessage::OpenFile => {
                if self.is_loading {
                    Task::none()
                } else {
                    self.is_loading = true;
                    Task::perform(open(), EditorMessage::FileOpened)
                }
            }
            EditorMessage::FileOpened(result) => {
                self.is_loading = false;
                self.is_dirty = false;
                if let Ok((path, chunks)) = result {
                    self.file = Some(path);

                    let mut builder = TextBufferBuilder::new();
                    for s in chunks {
                        builder.accept_chunk(&s);
                    }
                    self.buffer = builder.finish();
                    self.input_value.clear();
                    self.set_cursor(0, 0);
                    self.is_dirty = false;
                    self.render_version = self.render_version.wrapping_add(1);
                }
                Task::none()
            }
            EditorMessage::SaveFile => Task::none(),
            EditorMessage::FileSaved(_result) => Task::none(),
            EditorMessage::ActivateEditor => {
                self.active = true;
                text_input::focus(self.input_id.clone())
            }
            EditorMessage::DeactivateEditor => {
                self.active = false;
                Task::none()
            }
            EditorMessage::SetCursor { line, column } => {
                self.set_cursor(line, column);
                self.preferred_col = Some(self.col);
                text_input::focus(self.input_id.clone())
            }
            EditorMessage::Insert(to_insert) => {
                self.insert(to_insert.as_str());
                text_input::focus(self.input_id.clone())
            }
            EditorMessage::Backspace => {
                self.backspace();
                text_input::focus(self.input_id.clone())
            }
            EditorMessage::Enter => {
                self.enter();
                text_input::focus(self.input_id.clone())
            }
            EditorMessage::MoveLeft => {
                self.cursor_left();
                text_input::focus(self.input_id.clone())
            }
            EditorMessage::MoveRight => {
                self.cursor_right();
                text_input::focus(self.input_id.clone())
            }
            EditorMessage::MoveUp => {
                self.cursor_up();
                text_input::focus(self.input_id.clone())
            }
            EditorMessage::MoveDown => {
                self.cursor_down();
                text_input::focus(self.input_id.clone())
            }
        }
    }

    pub fn view(&self) -> Element<'_, EditorMessage> {
        let controls = container(
            row![
                action(text("New").size(12), Some(EditorMessage::NewFile)),
                action(text("Open File...").size(12), Some(EditorMessage::OpenFile)),
                action(text("Save File").size(12), Some(EditorMessage::SaveFile)),
            ]
            .align_y(Center)
            .height(Length::Fixed(20.0))
            .spacing(8),
        )
        .width(Length::Fill)
        .padding([2, 8])
        .style(top_bar_bg);

        let status = container(row![
            text(if let Some(path) = &self.file {
                let path = path.display().to_string();
                if path.len() > 60 {
                    format!("...{}", &path[path.len() - 40..])
                } else {
                    path
                }
            } else {
                String::from("New file")
            }),
            horizontal_space(),
            text(format!("{}:{}", self.line + 1, self.col + 1))
        ])
        .padding([2, 8])
        .width(Length::Fill)
        .style(bottom_bar_bg);

        let content_height = self.buffer.get_line_count() as f32 * FONT_SIZE * LINE_SPACING;

        let canvas = container(
            row![
                scrollable(
                    canvas::Canvas::new(EditorCanvas::new(
                        &self.buffer,
                        Font::MONOSPACE,
                        FONT_SIZE,
                        LINE_SPACING,
                        self.line,
                        self.col,
                        self.render_version,
                    ))
                    .width(iced::Fill)
                    .height(Length::Fixed(content_height + 850.0)),
                ),
                // Hidden text_input to receive text runs & IME
                container(
                    text_input("", &self.input_value)
                        .on_input(EditorMessage::Insert)
                        .on_submit(EditorMessage::Enter)
                        .id(self.input_id.clone())
                        .size(1)
                        .padding(0)
                )
                .width(Length::Fixed(1.0))
                .height(Length::Fixed(1.0)),
            ]
            .height(iced::Fill),
        )
        .style(editor_bg)
        .height(iced::Fill);

        column![
            controls,
            horizontal_rule(1).style(black_rule),
            canvas,
            horizontal_rule(1).style(black_rule),
            status,
        ]
        .into()
    }

    pub fn theme(&self) -> Theme {
        if self.theme.is_dark() {
            Theme::Dark
        } else {
            Theme::Light
        }
    }

    pub fn subscription(&self) -> Subscription<EditorMessage> {
        if self.active {
            // Listen to all runtime events
            event::listen_with(map_runtime_event)
        } else {
            Subscription::none()
        }
    }

    fn set_cursor(&mut self, line: usize, column: usize) {
        let last_line0 = self.buffer.get_line_count().saturating_sub(1);
        self.line = line.min(last_line0);

        let line_text = self.buffer.get_line_content(self.line + 1);
        let max_col0 = grapheme_count(&line_text);
        self.col = column.min(max_col0);

        self.active = true;
        self.render_version = self.render_version.wrapping_add(1);
    }

    fn insert(&mut self, to_insert: &str) {
        self.input_value = to_insert.to_string();

        let current_line = self.buffer.get_line_content(self.line + 1);
        let byte_col0 = byte_col_for_grapheme_col(&current_line, self.col);
        self.buffer
            .insert_at(self.line + 1, byte_col0 + 1, to_insert);

        if to_insert.contains('\n') {
            let parts: Vec<&str> = to_insert.split('\n').collect();
            self.line += parts.len() - 1;
            self.col = parts.last().map(|s| grapheme_count(s)).unwrap_or(0);
        } else {
            self.col += grapheme_count(to_insert);
        }

        let line_text = self.buffer.get_line_content(self.line + 1);
        let max_col0 = grapheme_count(&line_text);
        if self.col > max_col0 {
            self.col = max_col0;
        }
        self.preferred_col = Some(self.col);
        self.input_value.clear();
        self.is_dirty = true;
        self.render_version = self.render_version.wrapping_add(1);
    }

    fn enter(&mut self) {
        let current_line = self.buffer.get_line_content(self.line + 1);
        let byte_col0 = byte_col_for_grapheme_col(&current_line, self.col);
        self.buffer.insert_at(self.line + 1, byte_col0 + 1, "\n");
        self.line += 1;
        self.col = 0;
        self.preferred_col = Some(self.col);
        self.is_dirty = true;
        self.render_version = self.render_version.wrapping_add(1);
        self.input_value.clear();
    }

    fn backspace(&mut self) {
        if self.col > 0 {
            let line_text = self.buffer.get_line_content(self.line + 1);
            let caret_byte = byte_col_for_grapheme_col(&line_text, self.col);
            let prev_start_byte = byte_col_for_grapheme_col(&line_text, self.col - 1);
            let len_bytes = caret_byte.saturating_sub(prev_start_byte);
            if len_bytes > 0 {
                self.buffer
                    .delete_at(self.line + 1, prev_start_byte + 1, len_bytes);
            }
            self.col -= 1;
        } else if self.line > 0 {
            let prev_line1 = self.line;
            let prev_text_before = self.buffer.get_line_content(prev_line1);
            let prev_end_col1 = self.buffer.get_line_length(prev_line1) + 1;
            self.buffer.delete_at(prev_line1, prev_end_col1, 1);
            self.line -= 1;
            self.col = grapheme_count(&prev_text_before);
        }
        self.render_version = self.render_version.wrapping_add(1);
        self.input_value.clear();
    }

    fn cursor_left(&mut self) {
        if self.col > 0 {
            self.set_cursor(self.line, self.col.saturating_sub(1));
        } else if self.line > 0 {
            let prev_line = self.line - 1;
            let end_prev = grapheme_count(&self.buffer.get_line_content(prev_line + 1));
            self.set_cursor(prev_line, end_prev);
        }
        self.preferred_col = Some(self.col);
    }

    fn cursor_right(&mut self) {
        let max_col0 = grapheme_count(&self.buffer.get_line_content(self.line + 1));
        if self.col < max_col0 {
            self.set_cursor(self.line, self.col + 1);
        } else if self.line + 1 < self.buffer.get_line_count() {
            self.set_cursor(self.line + 1, 0);
        }
        self.preferred_col = Some(self.col);
    }

    fn cursor_up(&mut self) {
        if self.line == 0 {
            return;
        }
        let desired = self.preferred_col.unwrap_or(self.col);
        self.set_cursor(self.line.saturating_sub(1), desired);
    }

    fn cursor_down(&mut self) {
        if self.line + 1 >= self.buffer.get_line_count() {
            return;
        }
        let desired = self.preferred_col.unwrap_or(self.col);
        self.set_cursor(self.line + 1, desired);
    }
}

async fn open() -> Result<(PathBuf, Vec<String>), Error> {
    let file = rfd::AsyncFileDialog::new()
        .set_title("Open a text file...")
        .pick_file()
        .await
        .ok_or(Error::DialogClosed)?;

    let path = file.path().to_path_buf();

    let chunks =
        TextBufferBuilder::read_chunks_from_path(&path).map_err(|e| Error::IoError(e.kind()))?;

    Ok((path, chunks))
}

fn action<'a, EditorMessage: Clone + 'a>(
    content: impl Into<Element<'a, EditorMessage>>,
    on_press: Option<EditorMessage>,
) -> Element<'a, EditorMessage> {
    let action = button(iced::widget::center(content).width(iced::Shrink))
        .padding([2, 2])
        .style(transparent_button);

    if let Some(on_press) = on_press {
        action.on_press(on_press).into()
    } else {
        action.into()
    }
}

fn transparent_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let base_text = palette.text;
    let accent = iced::Color::from_rgb8(128, 128, 128);

    let mut style = button::Style {
        background: None,
        text_color: base_text,
        border: Border {
            radius: Radius::from(4),
            ..Default::default()
        },
        ..Default::default()
    };

    match status {
        button::Status::Hovered => {
            style.background = Some(iced::Background::Color(iced::Color { a: 0.05, ..accent }));
        }
        button::Status::Pressed => {
            style.background = Some(iced::Background::Color(iced::Color { a: 0.1, ..accent }));
        }
        button::Status::Disabled => {
            style.text_color = iced::Color {
                a: 0.4,
                ..base_text
            };
        }
        _ => {}
    }

    style
}

fn black_rule(_: &iced::Theme) -> rule::Style {
    rule::Style {
        color: iced::Color::BLACK,
        width: 1,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
    }
}

fn top_bar_bg(_: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        text_color: None,
        background: Some(iced::Background::Color(iced::Color::from_rgba8(
            22, 23, 19, 1.0,
        ))),
        border: iced::Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        shadow: iced::Shadow {
            ..Default::default()
        },
    }
}

fn editor_bg(_: &Theme) -> container::Style {
    container::Style {
        text_color: None,
        background: Some(iced::Background::Color(iced::Color::from_rgba8(
            39, 40, 34, 1.0,
        ))),
        border: iced::Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        shadow: Shadow {
            ..Default::default()
        },
    }
}

fn bottom_bar_bg(_: &Theme) -> container::Style {
    container::Style {
        text_color: None,
        background: Some(iced::Background::Color(iced::Color::from_rgba8(
            32, 33, 28, 1.0,
        ))),
        border: iced::Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        shadow: Shadow {
            ..Default::default()
        },
    }
}

fn grapheme_count(s: &str) -> usize {
    s.graphemes(true).count()
}

fn byte_col_for_grapheme_col(line: &str, grapheme_col0: usize) -> usize {
    // Return 0-based byte column corresponding to a 0-based grapheme column
    if grapheme_col0 == 0 {
        return 0;
    }
    let mut bytes = 0usize;
    for (i, g) in line.graphemes(true).enumerate() {
        if i >= grapheme_col0 {
            break;
        }
        bytes += g.len();
    }
    bytes
}

fn map_runtime_event(ev: Event, _status: event::Status, _id: window::Id) -> Option<EditorMessage> {
    if let Event::Keyboard(iced::keyboard::Event::KeyPressed { key, .. }) = ev {
        match key {
            Key::Named(Named::Backspace) => Some(EditorMessage::Backspace),
            Key::Named(Named::ArrowLeft) => Some(EditorMessage::MoveLeft),
            Key::Named(Named::ArrowRight) => Some(EditorMessage::MoveRight),
            Key::Named(Named::ArrowUp) => Some(EditorMessage::MoveUp),
            Key::Named(Named::ArrowDown) => Some(EditorMessage::MoveDown),
            _ => None,
        }
    } else {
        None
    }
}
