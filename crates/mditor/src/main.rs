mod custom_widget;
mod helper;
mod model;

use custom_widget::editor_canvas::EditorCanvas;
use helper::icon::{new_icon, open_icon, save_icon};
use iced::highlighter;
use iced::widget::{self, button, canvas, column, container, horizontal_space, row, text, tooltip};
use iced::{Center, Element, Font, Task, Theme};
use model::{editor_message::EditorMessage, error::Error};
use std::path::PathBuf;
use text_buffer::{TextBuffer, TextBufferBuilder};

pub fn main() -> iced::Result {
    iced::application("Mditor", Editor::update, Editor::view)
        .theme(Editor::theme)
        .default_font(Font::MONOSPACE)
        .run_with(Editor::new)
}

struct Editor {
    file: Option<PathBuf>,
    buffer: TextBuffer,
    theme: highlighter::Theme,
    is_loading: bool,
    is_dirty: bool,
}

impl Editor {
    pub fn new() -> (Self, Task<EditorMessage>) {
        (
            Self {
                file: None,
                buffer: TextBufferBuilder::new().finish(),
                theme: highlighter::Theme::SolarizedDark,
                is_loading: false,
                is_dirty: false,
            },
            Task::batch([widget::focus_next()]),
        )
    }

    pub fn update(&mut self, message: EditorMessage) -> Task<EditorMessage> {
        match message {
            EditorMessage::ActionPerformed => Task::none(),

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
            // TODO: implement saving
            EditorMessage::SaveFile => Task::none(),
            EditorMessage::FileSaved(_result) => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, EditorMessage> {
        let controls = row![
            action(new_icon(), "New file", Some(EditorMessage::NewFile)),
            action(
                open_icon(),
                "Open file",
                (!self.is_loading).then_some(EditorMessage::OpenFile)
            ),
            action(
                save_icon(),
                "Save file",
                self.is_dirty.then_some(EditorMessage::SaveFile)
            ),
        ]
        .align_y(Center)
        .height(iced::Length::Fixed(32.0));

        let status = row![
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
            text({
                // TODO: Implement actual coordinate tracking
                let (line, column) = (0, 0);
                format!("{}:{}", line + 1, column + 1)
            })
        ]
        .spacing(10);

        let canvas = canvas::Canvas::new(EditorCanvas::new(&self.buffer, Font::MONOSPACE, 14.0))
            .width(iced::Fill)
            .height(iced::Fill);

        column![controls, canvas, status,]
            .spacing(10)
            .padding(10)
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
    label: &'a str,
    on_press: Option<EditorMessage>,
) -> Element<'a, EditorMessage> {
    let action = button(iced::widget::center(content).width(30));

    if let Some(on_press) = on_press {
        tooltip(
            action.on_press(on_press),
            label,
            tooltip::Position::FollowCursor,
        )
        .style(container::rounded_box)
        .into()
    } else {
        action.style(button::secondary).into()
    }
}
