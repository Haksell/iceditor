use {
    iced::{
        Application, Command, Element, Font, Length, Settings, Theme, executor,
        highlighter::{self, Highlighter},
        theme,
        widget::{button, column, container, horizontal_space, row, text, text_editor, tooltip},
    },
    std::{
        io,
        path::{Path, PathBuf},
        sync::Arc,
    },
};

fn main() -> iced::Result {
    Editor::run(Settings {
        // default_font: Font::MONOSPACE,
        fonts: vec![include_bytes!("../iceditor.ttf").as_slice().into()],
        ..Settings::default()
    })
}

#[derive(Debug, Clone)]
enum Message {
    New,
    Edit(text_editor::Action),
    Open,
    FileOpened(Result<(PathBuf, Arc<String>), Error>),
    Save,
    FileSaved(Result<PathBuf, Error>),
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
}

impl Application for Editor {
    type Executor = executor::Default;
    type Flags = ();
    type Message = Message;
    type Theme = Theme;

    fn new(_: Self::Flags) -> (Self, Command<Message>) {
        let editor = Self {
            path: None,
            content: text_editor::Content::new(),
            error: None,
        };
        let command = Command::perform(load_file(default_file()), Message::FileOpened);

        (editor, command)
    }

    fn title(&self) -> String {
        String::from("A cool editor!")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::New => {
                self.path = None;
                self.content = text_editor::Content::new();
            }
            Message::Edit(action) => {
                self.content.edit(action);
                self.error = None;
            }
            Message::Open => {
                return Command::perform(pick_file(), Message::FileOpened);
            }
            Message::FileOpened(Ok((path, content))) => {
                self.path = Some(path);
                self.content = text_editor::Content::with(&content);
            }
            Message::FileOpened(Err(err)) => {
                self.error = Some(err);
            }
            Message::Save => {
                let text = self.content.text();
                return Command::perform(save_file(self.path.clone(), text), Message::FileSaved);
            }
            Message::FileSaved(Ok(path)) => {
                self.path = Some(path);
            }
            Message::FileSaved(Err(err)) => {
                self.error = Some(err);
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let controls = row![
            action_button(new_icon(), "New file", Message::New),
            action_button(open_icon(), "Open file", Message::Open),
            action_button(save_icon(), "Save file", Message::Save),
        ]
        .spacing(10);

        let input = text_editor(&self.content)
            .on_edit(Message::Edit)
            .highlight::<Highlighter>(
                highlighter::Settings {
                    theme: highlighter::Theme::SolarizedDark,
                    extension: self
                        .path
                        .as_ref()
                        .and_then(|path| path.extension()?.to_str())
                        .unwrap_or("rs")
                        .to_string(),
                },
                |highlight, _| highlight.to_format(),
            );

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
            row![status, horizontal_space(Length::Fill), position]
        };

        container(column![controls, input, status_bar].spacing(10))
            .padding(10)
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

fn action_button<'a>(
    content: Element<'a, Message>,
    label: &str,
    on_press: Message,
) -> Element<'a, Message> {
    tooltip(
        button(container(content).width(30).center_x())
            .on_press(on_press)
            .padding([5, 10]),
        label,
        tooltip::Position::FollowCursor,
    )
    .style(theme::Container::Box)
    .into()
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
    let handle = rfd::AsyncFileDialog::new()
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
        rfd::AsyncFileDialog::new()
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
