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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use futures_util::StreamExt;

    use crate::JoinSetExt;

    #[test]
    fn test_default() {
        let x: Option<u32> = super::default();
        assert_eq!(x, None);
    }

    #[test]
    fn test_git_project_root_fail() {
        let path = std::env::current_dir().unwrap();
        std::env::set_current_dir("/").unwrap();
        assert!(super::git_project_root().is_err());
        std::env::set_current_dir(path).unwrap();
    }

    #[tokio::test]
    async fn test_join_set_ext() {
        let mut set = tokio::task::JoinSet::new();
        set.spawn(async { 1 });
        set.spawn(async { 2 });
        set.spawn(async { 3 });
        let mut stream = set.into_stream();
        assert_eq!(stream.next().await, Some(1));
        assert_eq!(stream.next().await, Some(2));
        assert_eq!(stream.next().await, Some(3));
        assert_eq!(stream.next().await, None);
    }

    /// tests the stream dropping while values are still being created
    ///
    /// does not really test any specific behavior, but it's a good way to see if the test
    /// hangs/panics
    #[tokio::test]
    async fn test_stream_early_drop() {
        let mut set = tokio::task::JoinSet::new();

        set.spawn(async {
            tokio::time::sleep(Duration::from_millis(500)).await;
            1
        });

        // dropped
        {
            set.into_stream();
        }

        tokio::time::sleep(Duration::from_millis(1_000)).await;
    }

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
