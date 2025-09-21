use crate::custom_widget::editor_canvas::EditorCanvas;
use crate::model::{editor_message::EditorMessage, error::Error};
use iced::border::Radius;
use iced::widget::{
    self, button, canvas, column, container, horizontal_rule, horizontal_space, row, rule,
    scrollable, text,
};
use iced::{Border, Center, Element, Font, Shadow, Task, Theme};
use iced::{Length, highlighter};
use std::path::PathBuf;
use text_buffer::{TextBuffer, TextBufferBuilder};

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
}

impl App {
    pub fn new() -> (Self, Task<EditorMessage>) {
        (
            Self {
                file: None,
                buffer: TextBufferBuilder::new().finish(),
                theme: highlighter::Theme::SolarizedDark,
                is_loading: false,
                is_dirty: false,
                active: false,
                line: 0,
                col: 0,
            },
            Task::batch([widget::focus_next()]),
        )
    }

    pub fn update(&mut self, message: EditorMessage) -> Task<EditorMessage> {
        match message {
            EditorMessage::NewFile => {
                if !self.is_loading {
                    self.file = None;
                    self.buffer = TextBufferBuilder::new().finish();
                    self.is_dirty = false;
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
                    self.is_dirty = false;
                }
                Task::none()
            }
            EditorMessage::SaveFile => Task::none(),
            EditorMessage::FileSaved(_result) => Task::none(),
            EditorMessage::ActivateEditor => {
                self.active = true;
                Task::none()
            }
            EditorMessage::DeactivateEditor => {
                self.active = false;
                Task::none()
            }
            EditorMessage::SetCursor { line, column } => {
                // State is 0-based; buffer API is 1-based. Clamp accordingly.
                let last_line0 = self.buffer.get_line_count().saturating_sub(1);
                self.line = line.min(last_line0);

                let max_col1 = self.buffer.get_line_max_column(self.line + 1);
                let max_col0 = max_col1.saturating_sub(1);
                self.col = column.min(max_col0);

                self.active = true;
                Task::none()
            }
            EditorMessage::Insert(to_insert) => {
                self.buffer.insert_at(self.line, self.col, &to_insert);
                Task::none()
            }
            EditorMessage::Backspace => Task::none(),
            EditorMessage::Enter => Task::none(),
            EditorMessage::MoveLeft => Task::none(),
            EditorMessage::MoveRight => Task::none(),
            EditorMessage::MoveUp => Task::none(),
            EditorMessage::MoveDown => Task::none(),
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
            row![scrollable(
                canvas::Canvas::new(EditorCanvas::new(
                    &self.buffer,
                    Font::MONOSPACE,
                    FONT_SIZE,
                    LINE_SPACING,
                    self.line,
                    self.col
                ))
                .width(iced::Fill)
                .height(Length::Fixed(content_height + 850.0)),
            )]
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
