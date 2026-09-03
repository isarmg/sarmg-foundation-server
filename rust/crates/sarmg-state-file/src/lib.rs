//! Linux server state-file invariants shared by online products and offline tools.
//!
//! This is deliberately narrower than a general path-sandbox library. It owns
//! one already-resolved private directory, direct child state files, stable
//! device/inode checks, and the instance/maintenance lock protocol.

use rustix::fs::{FlockOperation, OFlags, flock};
use std::{
    ffi::OsStr,
    fs::{self, DirBuilder, File, Metadata, OpenOptions},
    io,
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::{Component, Path, PathBuf},
};
use thiserror::Error;

pub const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
pub const PRIVATE_FILE_MODE: u32 = 0o600;
pub const INSTANCE_LOCK_FILE: &str = ".sarmg-instance.lock";
pub const MAINTENANCE_LOCK_FILE: &str = ".sarmg-maintenance.lock";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileIdentity {
    device: u64,
    inode: u64,
}

impl FileIdentity {
    pub const fn device(self) -> u64 {
        self.device
    }

    pub const fn inode(self) -> u64 {
        self.inode
    }

    fn from_metadata(metadata: &Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PrivateStateDirectory {
    path: PathBuf,
    identity: FileIdentity,
    owner_uid: u32,
}

impl PrivateStateDirectory {
    /// Create the directory when missing, then require an absolute, private,
    /// non-symlink directory owned by the effective process user.
    pub fn create(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        require_absolute(path)?;
        match fs::symlink_metadata(path) {
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let mut builder = DirBuilder::new();
                builder
                    .mode(PRIVATE_DIRECTORY_MODE)
                    .create(path)
                    .map_err(|source| Error::CreateDirectory {
                        path: path.to_path_buf(),
                        source,
                    })?;
            }
            Err(source) => {
                return Err(Error::InspectPath {
                    path: path.to_path_buf(),
                    source,
                });
            }
        }
        Self::open(path)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        require_absolute(path)?;
        let metadata = symlink_metadata(path)?;
        let owner_uid = rustix::process::geteuid().as_raw();
        require_directory(path, &metadata, owner_uid)?;
        Ok(Self {
            path: path.to_path_buf(),
            identity: FileIdentity::from_metadata(&metadata),
            owner_uid,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub const fn identity(&self) -> FileIdentity {
        self.identity
    }

    pub const fn owner_uid(&self) -> u32 {
        self.owner_uid
    }

    pub fn verify_identity(&self) -> Result<(), Error> {
        let metadata = symlink_metadata(&self.path)?;
        require_directory(&self.path, &metadata, self.owner_uid)?;
        require_identity(
            &self.path,
            self.identity,
            FileIdentity::from_metadata(&metadata),
        )
    }

    pub fn open_existing(&self, name: impl AsRef<OsStr>) -> Result<SecureStateFile, Error> {
        self.open_file(name.as_ref(), false)
    }

    pub fn create_file(&self, name: impl AsRef<OsStr>) -> Result<SecureStateFile, Error> {
        self.open_file(name.as_ref(), true)
    }

    pub fn try_instance_lock(&self) -> Result<InstanceLock, Error> {
        self.verify_identity()?;
        let maintenance = self.create_file(MAINTENANCE_LOCK_FILE)?;
        try_flock(
            maintenance.file(),
            FlockOperation::NonBlockingLockShared,
            LockKind::Maintenance,
        )?;
        let instance = self.create_file(INSTANCE_LOCK_FILE)?;
        try_flock(
            instance.file(),
            FlockOperation::NonBlockingLockExclusive,
            LockKind::Instance,
        )?;
        Ok(InstanceLock {
            _maintenance_guard: maintenance,
            _instance: instance,
        })
    }

    pub fn try_maintenance_lock(&self) -> Result<MaintenanceLock, Error> {
        self.verify_identity()?;
        let file = self.create_file(MAINTENANCE_LOCK_FILE)?;
        try_flock(
            file.file(),
            FlockOperation::NonBlockingLockExclusive,
            LockKind::Maintenance,
        )?;
        Ok(MaintenanceLock { _file: file })
    }

    fn open_file(&self, name: &OsStr, create: bool) -> Result<SecureStateFile, Error> {
        self.verify_identity()?;
        let relative = Path::new(name);
        require_direct_child(relative)?;
        let path = self.path.join(relative);
        let mut options = OpenOptions::new();
        options
            .read(true)
            .write(true)
            .create(create)
            .mode(PRIVATE_FILE_MODE)
            .custom_flags(OFlags::NOFOLLOW.bits() as i32);
        let file = options.open(&path).map_err(|source| Error::OpenFile {
            path: path.clone(),
            source,
        })?;
        let metadata = file.metadata().map_err(|source| Error::InspectPath {
            path: path.clone(),
            source,
        })?;
        require_file(&path, &metadata, self.owner_uid)?;
        let identity = FileIdentity::from_metadata(&metadata);
        let path_metadata = symlink_metadata(&path)?;
        require_file(&path, &path_metadata, self.owner_uid)?;
        require_identity(&path, identity, FileIdentity::from_metadata(&path_metadata))?;
        self.verify_identity()?;
        Ok(SecureStateFile {
            file,
            path,
            identity,
            owner_uid: self.owner_uid,
        })
    }
}

#[derive(Debug)]
pub struct SecureStateFile {
    file: File,
    path: PathBuf,
    identity: FileIdentity,
    owner_uid: u32,
}

impl SecureStateFile {
    pub fn file(&self) -> &File {
        &self.file
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub const fn identity(&self) -> FileIdentity {
        self.identity
    }

    /// Re-check both the open descriptor and its path. This detects replacement
    /// between the initial check and a database driver's independent open.
    pub fn verify_identity(&self) -> Result<(), Error> {
        let descriptor = self.file.metadata().map_err(|source| Error::InspectPath {
            path: self.path.clone(),
            source,
        })?;
        require_file(&self.path, &descriptor, self.owner_uid)?;
        require_identity(
            &self.path,
            self.identity,
            FileIdentity::from_metadata(&descriptor),
        )?;
        let path_metadata = symlink_metadata(&self.path)?;
        require_file(&self.path, &path_metadata, self.owner_uid)?;
        require_identity(
            &self.path,
            self.identity,
            FileIdentity::from_metadata(&path_metadata),
        )
    }
}

#[derive(Debug)]
pub struct InstanceLock {
    _maintenance_guard: SecureStateFile,
    _instance: SecureStateFile,
}

#[derive(Debug)]
pub struct MaintenanceLock {
    _file: SecureStateFile,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LockKind {
    Instance,
    Maintenance,
}

fn try_flock(file: &File, operation: FlockOperation, kind: LockKind) -> Result<(), Error> {
    flock(file, operation).map_err(|source| Error::LockUnavailable {
        kind,
        source: source.into(),
    })
}

fn require_absolute(path: &Path) -> Result<(), Error> {
    if path.is_absolute() {
        Ok(())
    } else {
        Err(Error::StateDirectoryNotAbsolute {
            path: path.to_path_buf(),
        })
    }
}

fn require_direct_child(path: &Path) -> Result<(), Error> {
    let mut components = path.components();
    if matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none() {
        Ok(())
    } else {
        Err(Error::InvalidChildName {
            name: path.to_path_buf(),
        })
    }
}

fn symlink_metadata(path: &Path) -> Result<Metadata, Error> {
    fs::symlink_metadata(path).map_err(|source| Error::InspectPath {
        path: path.to_path_buf(),
        source,
    })
}

fn require_directory(path: &Path, metadata: &Metadata, owner_uid: u32) -> Result<(), Error> {
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(Error::NotDirectory {
            path: path.to_path_buf(),
        });
    }
    require_owner_and_mode(path, metadata, owner_uid, PRIVATE_DIRECTORY_MODE)
}

fn require_file(path: &Path, metadata: &Metadata, owner_uid: u32) -> Result<(), Error> {
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(Error::NotRegularFile {
            path: path.to_path_buf(),
        });
    }
    if metadata.nlink() != 1 {
        return Err(Error::UnexpectedHardLinkCount {
            path: path.to_path_buf(),
            actual: metadata.nlink(),
        });
    }
    require_owner_and_mode(path, metadata, owner_uid, PRIVATE_FILE_MODE)
}

fn require_owner_and_mode(
    path: &Path,
    metadata: &Metadata,
    owner_uid: u32,
    expected_mode: u32,
) -> Result<(), Error> {
    if metadata.uid() != owner_uid {
        return Err(Error::UnexpectedOwner {
            path: path.to_path_buf(),
            expected: owner_uid,
            actual: metadata.uid(),
        });
    }
    let actual = metadata.mode() & 0o777;
    if actual != expected_mode {
        return Err(Error::UnexpectedMode {
            path: path.to_path_buf(),
            expected: expected_mode,
            actual,
        });
    }
    Ok(())
}

fn require_identity(
    path: &Path,
    expected: FileIdentity,
    actual: FileIdentity,
) -> Result<(), Error> {
    if expected == actual {
        Ok(())
    } else {
        Err(Error::FileIdentityChanged {
            path: path.to_path_buf(),
            expected,
            actual,
        })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("state directory must be absolute: {path}", path = .path.display())]
    StateDirectoryNotAbsolute { path: PathBuf },
    #[error("state file name must be one normal direct child: {name}", name = .name.display())]
    InvalidChildName { name: PathBuf },
    #[error("cannot inspect state path {path}: {source}", path = .path.display())]
    InspectPath { path: PathBuf, source: io::Error },
    #[error("cannot create private state directory {path}: {source}", path = .path.display())]
    CreateDirectory { path: PathBuf, source: io::Error },
    #[error("cannot securely open state file {path}: {source}", path = .path.display())]
    OpenFile { path: PathBuf, source: io::Error },
    #[error("state path is not a real directory: {path}", path = .path.display())]
    NotDirectory { path: PathBuf },
    #[error("state path is not one regular no-follow file: {path}", path = .path.display())]
    NotRegularFile { path: PathBuf },
    #[error("state path {path} has owner {actual}, expected {expected}", path = .path.display())]
    UnexpectedOwner {
        path: PathBuf,
        expected: u32,
        actual: u32,
    },
    #[error("state path {path} has mode {actual:o}, expected {expected:o}", path = .path.display())]
    UnexpectedMode {
        path: PathBuf,
        expected: u32,
        actual: u32,
    },
    #[error("state file {path} has {actual} hard links; exactly one is required", path = .path.display())]
    UnexpectedHardLinkCount { path: PathBuf, actual: u64 },
    #[error("state file identity changed for {path}: expected {expected:?}, found {actual:?}", path = .path.display())]
    FileIdentityChanged {
        path: PathBuf,
        expected: FileIdentity,
        actual: FileIdentity,
    },
    #[error("{kind:?} lock is unavailable: {source}")]
    LockUnavailable { kind: LockKind, source: io::Error },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};

    fn private_tempdir() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700))?;
        Ok(directory)
    }

    #[test]
    fn creates_and_reopens_secure_files() -> Result<(), Box<dyn std::error::Error>> {
        let parent = private_tempdir()?;
        let path = parent.path().join("state");
        let state = PrivateStateDirectory::create(&path)?;
        let file = state.create_file("state.sqlite3")?;
        assert_eq!(file.file().metadata()?.mode() & 0o777, 0o600);
        assert_eq!(
            file.identity(),
            state.open_existing("state.sqlite3")?.identity()
        );
        file.verify_identity()?;
        Ok(())
    }

    #[test]
    fn rejects_relative_public_symlink_and_hardlinked_state()
    -> Result<(), Box<dyn std::error::Error>> {
        assert!(matches!(
            PrivateStateDirectory::open("relative"),
            Err(Error::StateDirectoryNotAbsolute { .. })
        ));
        let parent = private_tempdir()?;
        let public = parent.path().join("public");
        fs::create_dir(&public)?;
        fs::set_permissions(&public, fs::Permissions::from_mode(0o755))?;
        assert!(matches!(
            PrivateStateDirectory::open(&public),
            Err(Error::UnexpectedMode { .. })
        ));
        let private = parent.path().join("private");
        fs::create_dir(&private)?;
        fs::set_permissions(&private, fs::Permissions::from_mode(0o700))?;
        let link = parent.path().join("link");
        symlink(&private, &link)?;
        assert!(matches!(
            PrivateStateDirectory::open(&link),
            Err(Error::NotDirectory { .. })
        ));
        let state = PrivateStateDirectory::open(&private)?;
        state.create_file("database")?;
        fs::hard_link(private.join("database"), private.join("copy"))?;
        assert!(matches!(
            state.open_existing("database"),
            Err(Error::UnexpectedHardLinkCount { .. })
        ));
        Ok(())
    }

    #[test]
    fn detects_replacement_and_coordinates_locks() -> Result<(), Box<dyn std::error::Error>> {
        let parent = private_tempdir()?;
        let state = PrivateStateDirectory::create(parent.path().join("state"))?;
        let file = state.create_file("database")?;
        fs::rename(file.path(), state.path().join("old"))?;
        state.create_file("database")?;
        assert!(matches!(
            file.verify_identity(),
            Err(Error::FileIdentityChanged { .. })
        ));
        drop(file);

        let instance = state.try_instance_lock()?;
        assert!(matches!(
            state.try_instance_lock(),
            Err(Error::LockUnavailable {
                kind: LockKind::Instance,
                ..
            })
        ));
        assert!(matches!(
            state.try_maintenance_lock(),
            Err(Error::LockUnavailable {
                kind: LockKind::Maintenance,
                ..
            })
        ));
        drop(instance);
        let _maintenance = state.try_maintenance_lock()?;
        assert!(matches!(
            state.try_instance_lock(),
            Err(Error::LockUnavailable {
                kind: LockKind::Maintenance,
                ..
            })
        ));
        Ok(())
    }
}
