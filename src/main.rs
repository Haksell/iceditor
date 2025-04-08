use {
    iced::{
        Element, Font, Length, Task, Theme, highlighter, keyboard,
        widget::{
            button, column, container, horizontal_space, pick_list, row, text, text_editor, tooltip,
        },
    },
    rfd::AsyncFileDialog,
    std::{
        ffi::OsStr,
        io,
        path::{Path, PathBuf},
        sync::Arc,
    },
};

fn main() -> iced::Result {
    iced::application("iceditor", Editor::update, Editor::view)
        .theme(Editor::theme)
        .font(include_bytes!("../iceditor.ttf").as_slice())
        // .default_font(Font::MONOSPACE)
        .run_with(Editor::new)
}

#[derive(Debug, Clone)]
enum Message {
    New,
    Edit(text_editor::Action),
    Open,
    FileOpened(Result<(PathBuf, Arc<String>), Error>),
    Save,
    FileSaved(Result<PathBuf, Error>),
    ThemeSelected(highlighter::Theme),
}

#[derive(Debug, Clone)]
enum Error {
    DialogClosed,
    IoFailed(io::ErrorKind),
}

struct Editor {
    path: Option<PathBuf>,
    content: text_editor::Content,
    error: Option<Error>,
    theme: highlighter::Theme,
    is_dirty: bool,
}

impl Editor {
    fn new() -> (Self, Task<Message>) {
        let editor = Self {
            path: None,
            content: text_editor::Content::new(),
            error: None,
            theme: highlighter::Theme::SolarizedDark,
            is_dirty: true,
        };
        let command = Task::perform(load_file(default_file()), Message::FileOpened);

        (editor, command)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::New => {
                self.path = None;
                self.content = text_editor::Content::new();
                self.is_dirty = true;
            }
            Message::Edit(action) => {
                self.is_dirty = self.is_dirty || action.is_edit();
                self.error = None;
                self.content.perform(action);
            }
            Message::Open => {
                return Task::perform(pick_file(), Message::FileOpened);
            }
            Message::FileOpened(Ok((path, content))) => {
                self.path = Some(path);
                self.content = text_editor::Content::with_text(&content);
                self.is_dirty = false;
            }
            Message::FileOpened(Err(err)) => self.error = Some(err),
            Message::Save => {
                let text = self.content.text();
                return Task::perform(save_file(self.path.clone(), text), Message::FileSaved);
            }
            Message::FileSaved(Ok(path)) => {
                self.path = Some(path);
                self.is_dirty = false;
            }
            Message::FileSaved(Err(err)) => self.error = Some(err),
            Message::ThemeSelected(theme) => self.theme = theme,
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let controls = row![
            action_button(new_icon(), "New file", Some(Message::New)),
            action_button(open_icon(), "Open file", Some(Message::Open)),
            action_button(
                save_icon(),
                "Save file",
                self.is_dirty.then_some(Message::Save)
            ),
            horizontal_space(),
            pick_list(
                highlighter::Theme::ALL,
                Some(self.theme),
                Message::ThemeSelected
            )
        ]
        .spacing(10);

        let input = text_editor(&self.content)
            .height(Length::Fill)
            .on_action(Message::Edit)
            .highlight(
                self.path
                    .as_deref()
                    .and_then(Path::extension)
                    .and_then(OsStr::to_str)
                    .unwrap_or("rs"),
                self.theme,
            )
            .key_binding(|key_press| match key_press.key.as_ref() {
                keyboard::Key::Character("s") if key_press.modifiers.command() => {
                    Some(text_editor::Binding::Custom(Message::Save))
                }
                _ => text_editor::Binding::from_key_press(key_press),
            });

        let status_bar = {
            let status = if let Some(Error::IoFailed(error)) = self.error.as_ref() {
                text(error.to_string())
            } else {
                match self.path.as_deref().and_then(Path::to_str) {
                    Some(path) => text(path).size(14),
                    None => text("New file"),
                }
            };
            let position = {
                let (line, column) = self.content.cursor_position();
                text(format!("{}:{}", line + 1, column + 1))
            };
            row![status, horizontal_space(), position]
        };

        container(column![controls, input, status_bar].spacing(10))
            .padding(10)
            .into()
    }

    fn theme(&self) -> Theme {
        if self.theme.is_dark() {
            Theme::Dark
        } else {
            Theme::Light
        }
    }
}

fn action_button<'a>(
    content: Element<'a, Message>,
    label: &'a str,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let action = button(container(content).center_x(30));

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

fn icon<'a>(codepoint: char) -> Element<'a, Message> {
    const ICON_FONT: Font = Font::with_name("iceditor");

    text(codepoint).font(ICON_FONT).into()
}

fn new_icon<'a>() -> Element<'a, Message> {
    icon('\u{e800}')
}

fn open_icon<'a>() -> Element<'a, Message> {
    icon('\u{f115}')
}

fn save_icon<'a>() -> Element<'a, Message> {
    icon('\u{e801}')
}

fn default_file() -> PathBuf {
    PathBuf::from(format!("{}/src/main.rs", env!("CARGO_MANIFEST_DIR")))
}

async fn pick_file() -> Result<(PathBuf, Arc<String>), Error> {
    let handle = AsyncFileDialog::new()
        .set_title("Choose a text file...")
        .pick_file()
        .await
        .ok_or(Error::DialogClosed)?;

    load_file(handle.path().to_owned()).await
}

async fn load_file(path: PathBuf) -> Result<(PathBuf, Arc<String>), Error> {
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => Ok((path, Arc::new(content))),
        Err(err) => Err(Error::IoFailed(err.kind())),
    }
}

async fn save_file(path: Option<PathBuf>, text: String) -> Result<PathBuf, Error> {
    let path = if let Some(path) = path {
        path
    } else {
        AsyncFileDialog::new()
            .set_title("Choose a file name...")
            .save_file()
            .await
            .ok_or(Error::DialogClosed)
            .map(|handle| handle.path().to_owned())?
    };

    tokio::fs::write(&path, text)
        .await
        .map_err(|err| Error::IoFailed(err.kind()))?;

    Ok(path)
}
