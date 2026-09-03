//! Filesystem operations whose names encode their durability and link-safety guarantees.

use std::{
    ffi::OsStr,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelativePath(PathBuf);

impl RelativePath {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        if path.as_os_str().is_empty()
            || path
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(Error::UnsafeRelativePath(path.to_path_buf()));
        }
        Ok(Self(path.to_path_buf()))
    }
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

#[derive(Debug)]
pub struct PrivateDirectory {
    path: PathBuf,
    directory: File,
}

impl PrivateDirectory {
    pub fn create(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        if !path.is_absolute() {
            return Err(Error::AbsolutePathRequired(path.to_path_buf()));
        }
        match fs::create_dir(path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(source) => return Err(Error::Io(source)),
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
        }
        let metadata = fs::symlink_metadata(path)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(Error::UnsafeFileType(path.to_path_buf()));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if metadata.nlink() != 2 {
                return Err(Error::MultipleLinks(path.to_path_buf()));
            }
        }
        let directory = File::open(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            directory,
        })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn resolve(&self, relative: &RelativePath) -> PathBuf {
        self.path.join(relative.as_path())
    }
    pub fn sync(&self) -> Result<(), Error> {
        self.directory.sync_all()?;
        Ok(())
    }
}

pub struct AtomicFile;
impl AtomicFile {
    pub fn replace(
        directory: &PrivateDirectory,
        destination: &RelativePath,
        bytes: &[u8],
    ) -> Result<(), Error> {
        let destination_path = directory.resolve(destination);
        let parent = destination_path
            .parent()
            .ok_or_else(|| Error::UnsafeRelativePath(destination.as_path().to_path_buf()))?;
        if parent != directory.path() {
            fs::create_dir_all(parent)?;
        }
        let nonce = process_nonce();
        let file_name = destination_path
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| Error::UnsafeRelativePath(destination.as_path().to_path_buf()))?;
        let temporary = parent.join(format!(".{file_name}.{nonce}.tmp"));
        let result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            fs::rename(&temporary, &destination_path)?;
            sync_directory(parent)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
}

pub struct NoClobberPublish;
impl NoClobberPublish {
    pub fn publish(source: &Path, destination: &Path) -> Result<(), Error> {
        if destination.exists() {
            return Err(Error::DestinationExists(destination.to_path_buf()));
        }
        fs::hard_link(source, destination).map_err(Error::Io)?;
        let file = File::open(destination)?;
        file.sync_all()?;
        sync_directory(
            destination
                .parent()
                .ok_or_else(|| Error::UnsafeRelativePath(destination.to_path_buf()))?,
        )?;
        fs::remove_file(source)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InventoryLimits {
    pub max_entries: usize,
    pub max_total_bytes: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirectoryInventory {
    pub entries: usize,
    pub total_bytes: u64,
}

pub fn bounded_directory_inventory(
    path: &Path,
    limits: InventoryLimits,
) -> Result<DirectoryInventory, Error> {
    let mut pending = vec![path.to_path_buf()];
    let mut result = DirectoryInventory {
        entries: 0,
        total_bytes: 0,
    };
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let metadata = fs::symlink_metadata(entry.path())?;
            if metadata.file_type().is_symlink() {
                return Err(Error::UnsafeFileType(entry.path()));
            }
            result.entries = result.entries.checked_add(1).ok_or(Error::BudgetExceeded)?;
            result.total_bytes = result
                .total_bytes
                .checked_add(metadata.len())
                .ok_or(Error::BudgetExceeded)?;
            if result.entries > limits.max_entries || result.total_bytes > limits.max_total_bytes {
                return Err(Error::BudgetExceeded);
            }
            if metadata.is_dir() {
                pending.push(entry.path());
            }
        }
    }
    Ok(result)
}

pub fn sync_file_and_parent(path: &Path) -> Result<(), Error> {
    File::open(path)?.sync_all()?;
    sync_directory(
        path.parent()
            .ok_or_else(|| Error::UnsafeRelativePath(path.to_path_buf()))?,
    )
}
fn sync_directory(path: &Path) -> Result<(), Error> {
    File::open(path)?.sync_all()?;
    Ok(())
}
fn process_nonce() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    format!(
        "{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    )
}

#[cfg(target_os = "linux")]
pub mod linux {
    use super::{Error, RelativePath};
    use rustix::{
        fd::OwnedFd,
        fs::{Mode, OFlags, ResolveFlags, fstat, openat2},
    };
    use std::{fs::File, path::Path};

    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    pub struct FileIdentity {
        pub device: u64,
        pub inode: u64,
    }

    #[derive(Debug)]
    pub struct OpenAt2Root {
        root: File,
        identity: FileIdentity,
    }

    impl OpenAt2Root {
        pub fn open(path: &Path) -> Result<Self, Error> {
            let root = File::open(path)?;
            let stat = fstat(&root).map_err(std::io::Error::from)?;
            if !rustix::fs::FileType::from_raw_mode(stat.st_mode).is_dir() {
                return Err(Error::UnsafeFileType(path.to_path_buf()));
            }
            Ok(Self {
                root,
                identity: FileIdentity {
                    device: stat.st_dev,
                    inode: stat.st_ino,
                },
            })
        }
        pub const fn identity(&self) -> FileIdentity {
            self.identity
        }
        pub fn open_file(&self, path: &RelativePath, write: bool) -> Result<File, Error> {
            let mut flags = OFlags::CLOEXEC | OFlags::NOFOLLOW;
            flags |= if write { OFlags::RDWR } else { OFlags::RDONLY };
            let fd: OwnedFd = openat2(
                &self.root,
                path.as_path(),
                flags,
                Mode::empty(),
                ResolveFlags::BENEATH | ResolveFlags::NO_MAGICLINKS | ResolveFlags::NO_XDEV,
            )
            .map_err(std::io::Error::from)?;
            Ok(File::from(fd))
        }
        pub fn identity_of(&self, path: &RelativePath) -> Result<FileIdentity, Error> {
            let file = self.open_file(path, false)?;
            let stat = fstat(&file).map_err(std::io::Error::from)?;
            if stat.st_nlink != 1 {
                return Err(Error::MultipleLinks(path.as_path().to_path_buf()));
            }
            Ok(FileIdentity {
                device: stat.st_dev,
                inode: stat.st_ino,
            })
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("an absolute path is required: {0:?}")]
    AbsolutePathRequired(PathBuf),
    #[error("unsafe relative path: {0:?}")]
    UnsafeRelativePath(PathBuf),
    #[error("path is not a regular unlinked object: {0:?}")]
    UnsafeFileType(PathBuf),
    #[error("path has multiple links: {0:?}")]
    MultipleLinks(PathBuf),
    #[error("destination already exists: {0:?}")]
    DestinationExists(PathBuf),
    #[error("directory inventory budget exceeded")]
    BudgetExceeded,
    #[error(transparent)]
    Io(#[from] io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_escape_and_atomically_publishes() {
        assert!(RelativePath::new("../escape").is_err());
        let temp = tempfile::tempdir().unwrap();
        let root = PrivateDirectory::create(temp.path().join("state")).unwrap();
        let name = RelativePath::new("current").unwrap();
        AtomicFile::replace(&root, &name, b"one").unwrap();
        AtomicFile::replace(&root, &name, b"two").unwrap();
        assert_eq!(fs::read(root.resolve(&name)).unwrap(), b"two");
    }
    #[cfg(unix)]
    #[test]
    fn inventory_rejects_symlinks_and_budgets() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("a"), b"abc").unwrap();
        assert!(
            bounded_directory_inventory(
                temp.path(),
                InventoryLimits {
                    max_entries: 0,
                    max_total_bytes: 10
                }
            )
            .is_err()
        );
        symlink("a", temp.path().join("b")).unwrap();
        assert!(matches!(
            bounded_directory_inventory(
                temp.path(),
                InventoryLimits {
                    max_entries: 10,
                    max_total_bytes: 10
                }
            ),
            Err(Error::UnsafeFileType(_))
        ));
    }
}
