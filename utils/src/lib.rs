use std::path::PathBuf;

use tokio::task::JoinSet;
use tokio_stream::wrappers::ReceiverStream;

pub mod discretize;

// pub type SyncBoxStream<'a, T> = Pin<Box<dyn futures_util::Stream<Item = T> + Send + Sync + 'a>>;
pub type Stream<T> = futures_util::stream::BoxStream<'static, T>;

/// Yield the default value for a type that implements [`Default`].
///
/// This is a copy of the [`default`] function from the standard library, which is not yet stable.
#[must_use]
pub fn default<T: Default>() -> T {
    T::default()
}

/// # Errors
/// If the current directory is not inside a git repository.
pub fn git_project_root() -> anyhow::Result<PathBuf> {
    let mut path = std::env::current_dir()?;
    loop {
        if path.join(".git").exists() {
            return Ok(path);
        }
        if !path.pop() {
            return Err(anyhow::anyhow!("Could not find git project root"));
        }
    }
}

/// Get a directory with respect to the git project root.
/// # Errors
/// If the current directory is not inside a git repository.
pub fn dir(dir: impl AsRef<str>) -> anyhow::Result<PathBuf> {
    let mut path = git_project_root()?;
    path.push(dir.as_ref());
    Ok(path)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_git_project_root() {
        let path = super::git_project_root().unwrap();
        assert!(path.join("Cargo.toml").exists());
    }

    #[test]
    fn test_dir() {
        let path = super::dir("Cargo.toml").unwrap();
        assert!(path.exists());
    }
}

pub trait JoinSetExt {
    type Item;
    fn into_stream(self) -> ReceiverStream<Self::Item>;
}

impl<Item: Send + 'static> JoinSetExt for JoinSet<Item> {
    type Item = Item;

    fn into_stream(mut self) -> ReceiverStream<Self::Item> {
        let (tx, rx) = tokio::sync::mpsc::channel(20);
        tokio::spawn(async move {
            while let Some(Ok(result)) = self.join_next().await {
                if tx.send(result).await.is_err() {
                    return;
                }
            }
        });
        ReceiverStream::new(rx)
    }
}
