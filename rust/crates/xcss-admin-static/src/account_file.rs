use super::*;
use serde::{Deserialize, Serialize};
use xcss_fs_safety::{AdvisoryLock, AtomicFile, EntryName, PrivateDirectory, RelativePath};

pub(super) struct AccountFile {
    directory: PrivateDirectory,
    _lock: AdvisoryLock,
}
impl std::fmt::Debug for AccountFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AccountFile([protected])")
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AccountDocument {
    version: u32,
    accounts: Vec<AccountRecord>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AccountRecord {
    id: String,
    username: String,
    password_hash: String,
    session_version: u64,
}
const FILE: &str = "administrator-accounts.json";

/// Shared current-document validation for both runtime loading and read-only
/// diagnostics. The expected set describes stable configured identities.
fn read_accounts(
    directory: &PrivateDirectory,
    configured_ids: &[String],
) -> Result<Vec<AdministratorRecord>, Error> {
    let bytes = directory.read_bounded(&EntryName::new(FILE)?, 1024 * 1024)?;
    let saved: AccountDocument =
        serde_json::from_slice(&bytes).map_err(|_| Error::InvalidAccountFile)?;
    if saved.version != 1 || saved.accounts.len() != configured_ids.len() {
        return Err(Error::InvalidAccountFile);
    }
    let ids: HashSet<_> = configured_ids.iter().map(String::as_str).collect();
    if ids.len() != configured_ids.len()
        || saved
            .accounts
            .iter()
            .any(|record| !ids.contains(record.id.as_str()))
    {
        return Err(Error::InvalidAccountFile);
    }
    let records = saved
        .accounts
        .into_iter()
        .map(|record| {
            Ok(AdministratorRecord {
                administrator_id: Identifier::new(record.id)?,
                username: record.username,
                password_hash: record.password_hash,
                session_version: record.session_version,
                active: true,
                created_at_micros: 0,
                updated_at_micros: 0,
                last_login_at_micros: None,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    StaticAdministratorStore::new(records.clone())?;
    Ok(records)
}

pub(super) fn validate_accounts(
    directory: &PrivateDirectory,
    configured_ids: &[String],
) -> Result<(), Error> {
    read_accounts(directory, configured_ids).map(|_| ())
}
impl AccountFile {
    pub(super) fn open(
        directory: PrivateDirectory,
        configured: Vec<AdministratorRecord>,
    ) -> Result<(Self, Vec<AdministratorRecord>), Error> {
        let lock = AdvisoryLock::acquire(
            &directory,
            &RelativePath::new(".administrator-accounts.lock")?,
        )?;
        let file = Self {
            directory,
            _lock: lock,
        };
        let ids = configured
            .iter()
            .map(|record| record.administrator_id.to_string())
            .collect::<Vec<_>>();
        let saved = match read_accounts(&file.directory, &ids) {
            Ok(records) => Some(records),
            Err(Error::Filesystem(xcss_fs_safety::Error::Io(error)))
                if error.kind() == std::io::ErrorKind::NotFound =>
            {
                None
            }
            Err(error) => return Err(error),
        };
        let initializing = saved.is_none();
        let records = saved.unwrap_or(configured);
        // Reuse all Foundation identity, username and password-hash validation.
        StaticAdministratorStore::new(records.clone())?;
        if initializing {
            file.save(&records)?;
        }
        Ok((file, records))
    }
    pub(super) fn save(&self, records: &[AdministratorRecord]) -> Result<(), Error> {
        let document = AccountDocument {
            version: 1,
            accounts: records
                .iter()
                .map(|record| AccountRecord {
                    id: record.administrator_id.to_string(),
                    username: record.username.clone(),
                    password_hash: record.password_hash.clone(),
                    session_version: record.session_version,
                })
                .collect(),
        };
        let bytes = serde_json::to_vec(&document).map_err(|_| Error::InvalidAccountFile)?;
        AtomicFile::replace(&self.directory, &RelativePath::new(FILE)?, &bytes)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xcss_admin_core::{
        AdministratorManagementContext, AdministratorService, LoginContext, ManagementError,
    };
    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros()
            .try_into()
            .unwrap()
    }
    fn configured() -> Vec<AdministratorRecord> {
        vec![AdministratorRecord {
            administrator_id: Identifier::new("stable-account-id").unwrap(),
            username: "admin".into(),
            password_hash: xcss_admin_auth::hash_password("correct horse battery").unwrap(),
            active: true,
            session_version: 1,
            created_at_micros: 0,
            updated_at_micros: 0,
            last_login_at_micros: None,
        }]
    }
    #[tokio::test]
    async fn account_changes_survive_restart_without_recreating_sessions() {
        let temp = tempfile::tempdir().unwrap();
        let directory = PrivateDirectory::create(temp.path().join("accounts")).unwrap();
        let records = configured();
        let service = AdministratorService::new(
            StaticAdministratorStore::with_persistent_accounts(records.clone(), directory).unwrap(),
        );
        let login_context = LoginContext {
            source: "127.0.0.1".into(),
            request_id: None,
            now_micros: now(),
        };
        let login = service
            .login("admin", "correct horse battery", &login_context)
            .await
            .unwrap();
        let context = AdministratorManagementContext {
            identity: service
                .authenticate_session(&login.session_token, now())
                .await
                .unwrap(),
            request_id: None,
            now_micros: now(),
        };
        assert!(matches!(
            service
                .update_own_account(&context, "renamed", "incorrect password", None)
                .await,
            Err(ManagementError::InvalidCurrentPassword)
        ));
        service
            .update_own_account(
                &context,
                "renamed",
                "correct horse battery",
                Some("updated correct password"),
            )
            .await
            .unwrap();
        assert!(
            service
                .authenticate_session(&login.session_token, now())
                .await
                .is_err()
        );
        assert!(
            StaticAdministratorStore::with_persistent_accounts(
                records.clone(),
                PrivateDirectory::open_existing(temp.path().join("accounts")).unwrap()
            )
            .is_err()
        );
        drop(service);
        let store = StaticAdministratorStore::with_persistent_accounts(
            records,
            PrivateDirectory::open_existing(temp.path().join("accounts")).unwrap(),
        )
        .unwrap();
        assert_eq!(store.active_session_count().unwrap(), 0);
        let service = AdministratorService::new(store);
        assert!(
            service
                .login(
                    "admin",
                    "correct horse battery",
                    &LoginContext {
                        now_micros: now(),
                        ..login_context.clone()
                    }
                )
                .await
                .is_err()
        );
        let login = service
            .login(
                "renamed",
                "updated correct password",
                &LoginContext {
                    now_micros: now(),
                    ..login_context
                },
            )
            .await
            .unwrap();
        assert_eq!(login.administrator.user_id, "stable-account-id");
    }
    #[cfg(unix)]
    #[test]
    fn rejects_redirected_account_file() {
        let temp = tempfile::tempdir().unwrap();
        let directory = PrivateDirectory::create(temp.path().join("accounts")).unwrap();
        let outside = temp.path().join("outside");
        std::fs::write(&outside, b"must remain unchanged").unwrap();
        std::os::unix::fs::symlink(&outside, directory.path().join(FILE)).unwrap();
        assert!(
            StaticAdministratorStore::with_persistent_accounts(configured(), directory).is_err()
        );
        assert_eq!(std::fs::read(outside).unwrap(), b"must remain unchanged");
    }

    #[test]
    fn inspection_is_readonly_requires_current_accounts_and_exact_configured_ids() {
        let temp = tempfile::tempdir().unwrap();
        let directory = PrivateDirectory::create(temp.path().join("accounts")).unwrap();
        assert!(
            StaticAdministratorStore::validate_persistent_accounts(&directory, &["admin-1".into()])
                .is_err()
        );
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
        let store =
            StaticAdministratorStore::with_persistent_accounts(configured(), directory).unwrap();
        drop(store);
        let directory = PrivateDirectory::open_existing(temp.path().join("accounts")).unwrap();
        let before = std::fs::read(directory.path().join(FILE)).unwrap();
        let ids: Vec<String> = configured()
            .iter()
            .map(|record| record.administrator_id.to_string())
            .collect();
        StaticAdministratorStore::validate_persistent_accounts(&directory, &ids).unwrap();
        assert!(
            StaticAdministratorStore::validate_persistent_accounts(&directory, &["wrong".into()])
                .is_err()
        );
        assert_eq!(std::fs::read(directory.path().join(FILE)).unwrap(), before);
    }
}
