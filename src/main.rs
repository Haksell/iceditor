use std::{io, path::Path, sync::Arc};

use iced::{
    Application, Command, Element, Length, Settings, Theme, executor,
    widget::{button, column, container, horizontal_space, row, text, text_editor},
};

fn main() -> iced::Result {
    Editor::run(Settings::default())
}

struct Editor {
    content: text_editor::Content,
    error: Option<Error>,
}

#[derive(Debug, Clone)]
enum Message {
    Edit(text_editor::Action),
    Open,
    FileOpened(Result<Arc<String>, Error>),
}

#[derive(Debug, Clone)]
enum Error {
    DialogClosed,
    Io(io::ErrorKind),
}

impl Application for Editor {
    type Executor = executor::Default;
    type Flags = ();
    type Message = Message;
    type Theme = Theme;

    fn new(_: Self::Flags) -> (Self, Command<Message>) {
        (
            Self {
                content: text_editor::Content::new(),
                error: None,
            },
            Command::perform(
                load_file(format!("{}/src/main.rs", env!("CARGO_MANIFEST_DIR"))),
                Message::FileOpened,
            ),
        )
    }

    fn title(&self) -> String {
        String::from("A cool editor!")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Edit(action) => {
                self.content.edit(action);
            }
            Message::FileOpened(Ok(content)) => {
                self.content = text_editor::Content::with(&content);
            }
            Message::FileOpened(Err(err)) => {
                self.error = Some(err);
            }
            Message::Open => {
                return Command::perform(pick_file(), Message::FileOpened);
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let controls = row![button("Open").on_press(Message::Open)];

        let input = text_editor(&self.content).on_edit(Message::Edit);

        let position = {
            let (line, column) = self.content.cursor_position();
            text(format!("{}:{}", line + 1, column + 1))
        };
        let status_bar = row![horizontal_space(Length::Fill), position];

        container(column![controls, input, status_bar].spacing(10))
            .padding(10)
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

async fn pick_file() -> Result<Arc<String>, Error> {
    let handle = rfd::AsyncFileDialog::new()
        .set_title("Choose a text file...")
        .pick_file()
        .await
        .ok_or(Error::DialogClosed)?;

    load_file(&handle.path()).await
}

async fn load_file(path: impl AsRef<Path>) -> Result<Arc<String>, Error> {
    match tokio::fs::read_to_string(path).await {
        Ok(content) => Ok(Arc::new(content)),
        Err(error) => Err(Error::Io(error.kind())),
    }
}
