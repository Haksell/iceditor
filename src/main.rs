use {
    iced::{
        Application, Command, Element, Length, Settings, Theme, executor,
        widget::{button, column, container, horizontal_space, row, text, text_editor},
    },
    std::{
        io,
        path::{Path, PathBuf},
        sync::Arc,
    },
};

fn main() -> iced::Result {
    Editor::run(Settings::default())
}

struct Editor {
    path: Option<PathBuf>,
    content: text_editor::Content,
    error: Option<Error>,
}

#[derive(Debug, Clone)]
enum Message {
    Edit(text_editor::Action),
    Open,
    FileOpened(Result<(PathBuf, Arc<String>), Error>),
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
            Message::Edit(action) => {
                self.content.edit(action);
            }
            Message::FileOpened(Ok((path, content))) => {
                self.path = Some(path);
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

        let file_path = match self.path.as_deref().and_then(Path::to_str) {
            Some(path) => text(path).size(14),
            None => text(""),
        };

        let position = {
            let (line, column) = self.content.cursor_position();
            text(format!("{}:{}", line + 1, column + 1))
        };
        let status_bar = row![file_path, horizontal_space(Length::Fill), position];

        container(column![controls, input, status_bar].spacing(10))
            .padding(10)
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
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
        Err(error) => Err(Error::Io(error.kind())),
    }
}
