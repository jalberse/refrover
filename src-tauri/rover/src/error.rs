use notify_debouncer_full::notify;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Diesel(#[from] diesel::result::Error),
    #[error(transparent)]
    Tauri(#[from] tauri::Error),
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
    #[error(transparent)]
    Notify(#[from] notify::Error),
    #[error("Error converting PathBuf to String. Path is likely not valid UTF-8.")]
    PathBufToString,
    #[error("The path is not a directory.")]
    NotADirectory,
    #[error("The directory already exists in the database")]
    DirectoryAlreadyExistsInDb,
}


impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
      S: serde::ser::Serializer,
    {
      serializer.serialize_str(self.to_string().as_ref())
    }
}

/// Wraps an error with the UUID of the task that caused it.
/// Used by the frontend to clear information relevant to the task,
/// since it will not get a TaskEnd event from it due to the error.
#[derive(Serialize, Debug)]
pub struct TaskError {
  pub task_uuid: String,
  pub error: Error,
}
