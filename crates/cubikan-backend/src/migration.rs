use std::path::Path;

use crate::{BackendError, MigrationError};

/// Rejects the retired schema-v1-to-v2 migration surface without touching its path.
///
/// Schema v3 is a fresh-only projection introduced after the required-origin
/// rebaseline. The historical migration cannot manufacture an origin for a v1
/// row, so retaining a successful migration path would create synthetic
/// attribution.
pub(crate) fn migrate_v1_to_v2(_path: &Path) -> Result<(), MigrationError> {
    Err(BackendError::UnsupportedSchemaVersion { found: 1 }.into())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::*;

    #[test]
    fn test_retired_migration_rejects_before_filesystem_access() {
        let path = PathBuf::from(format!(
            "{}/cubikan-retired-migration-{}-missing/parent/database.sqlite3",
            std::env::temp_dir().display(),
            std::process::id()
        ));
        assert!(!path.exists());

        assert_eq!(
            migrate_v1_to_v2(&path),
            Err(MigrationError::Backend(
                BackendError::UnsupportedSchemaVersion { found: 1 }
            ))
        );
        assert!(!path.exists());
        assert!(!path.parent().expect("fixture has a parent").exists());

        // Keep the assertion visibly read-only: no cleanup should be needed.
        assert!(matches!(
            fs::metadata(&path),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound
        ));
    }
}
