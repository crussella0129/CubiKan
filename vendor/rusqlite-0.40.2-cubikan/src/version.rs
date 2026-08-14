use crate::ffi;
use std::ffi::CStr;
use std::str::Utf8Error;

/// Returns the SQLite version as an integer; e.g., `3016002` for version
/// 3.16.2.
///
/// See [`sqlite3_libversion_number()`](https://www.sqlite.org/c3ref/libversion.html).
#[inline]
#[must_use]
pub fn version_number() -> i32 {
    unsafe { ffi::sqlite3_libversion_number() }
}

/// Returns the SQLite version as a string; e.g., `"3.16.2"` for version 3.16.2.
///
/// See [`sqlite3_libversion()`](https://www.sqlite.org/c3ref/libversion.html).
///
/// # Panics
///
/// Panics when version is not valid UTF-8.
#[inline]
#[must_use]
pub fn version() -> &'static str {
    let cstr = unsafe { CStr::from_ptr(ffi::sqlite3_libversion()) };
    cstr.to_str()
        .expect("SQLite version string is not valid UTF8 ?!")
}

/// Returns the compile-time options reported by the linked SQLite library.
///
/// This is the non-SQL equivalent of `PRAGMA compile_options`. The returned
/// strings omit SQLite's conventional `SQLITE_` prefix and retain the order
/// reported by [`sqlite3_compileoption_get`](https://sqlite.org/c3ref/compileoption_get.html).
///
/// # Errors
///
/// Returns an error if the linked SQLite library reports a compile option that
/// is not valid UTF-8.
pub fn compile_options() -> Result<Vec<String>, Utf8Error> {
    let mut options = Vec::new();
    let mut index = 0;
    loop {
        // SAFETY: SQLite owns each returned NUL-terminated string for the
        // process lifetime; a null pointer marks the end of the sequence.
        let option = unsafe { ffi::sqlite3_compileoption_get(index) };
        if option.is_null() {
            break;
        }
        // SAFETY: sqlite3_compileoption_get returns a valid NUL-terminated
        // string whenever it returns a non-null pointer.
        options.push(unsafe { CStr::from_ptr(option) }.to_str()?.to_owned());
        index += 1;
    }
    Ok(options)
}

/// Reports whether the linked SQLite library was built with `option`.
///
/// SQLite accepts the option with or without its conventional `SQLITE_`
/// prefix. Supplying a string with an interior NUL returns `false`.
#[must_use]
pub fn compile_option_used(option: &str) -> bool {
    let Ok(option) = std::ffi::CString::new(option) else {
        return false;
    };
    // SAFETY: `option` is a valid NUL-terminated string and SQLite does not
    // retain the pointer after sqlite3_compileoption_used returns.
    unsafe { ffi::sqlite3_compileoption_used(option.as_ptr()) != 0 }
}

/// Non-pointer identity fields for one registered SQLite VFS.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VfsIdentity {
    /// Registered VFS name.
    pub name: String,
    /// SQLite VFS ABI version.
    pub version: i32,
    /// Size of the VFS-specific `sqlite3_file` implementation.
    pub os_file_size: i32,
    /// Maximum pathname length accepted by the VFS.
    pub maximum_pathname: i32,
    /// Whether SQLite currently selects this VFS as its default.
    pub is_default: bool,
}

/// Returns the complete VFS registration chain beginning with SQLite's
/// current default VFS.
///
/// This is a non-SQL inspection surface. Pointer-valued implementation fields
/// remain private; callers receive only stable identity and shape values.
///
/// # Errors
///
/// Returns an error if a registered VFS name is not valid UTF-8.
pub fn registered_vfses() -> Result<Vec<VfsIdentity>, Utf8Error> {
    // SAFETY: a null name requests SQLite's default VFS and performs SQLite's
    // documented automatic initialization. SQLite owns the returned chain.
    let default = unsafe { ffi::sqlite3_vfs_find(std::ptr::null()) };
    let mut current = default;
    let mut identities = Vec::new();
    while !current.is_null() {
        // SAFETY: every element in SQLite's registered VFS chain is a live
        // sqlite3_vfs object owned by SQLite for the duration of this call.
        let vfs = unsafe { &*current };
        // SAFETY: registered SQLite VFS names are non-null NUL-terminated
        // strings for the lifetime of their registration.
        let name = unsafe { CStr::from_ptr(vfs.zName) }.to_str()?.to_owned();
        identities.push(VfsIdentity {
            name,
            version: vfs.iVersion,
            os_file_size: vfs.szOsFile,
            maximum_pathname: vfs.mxPathname,
            is_default: current == default,
        });
        current = vfs.pNext;
    }
    Ok(identities)
}

#[cfg(test)]
mod tests {
    #[test]
    fn compile_option_wrappers_agree() {
        let options = super::compile_options().expect("SQLite options are UTF-8");
        assert!(!options.is_empty());
        assert!(
            options
                .iter()
                .all(|option| super::compile_option_used(option))
        );
        assert!(!super::compile_option_used("CUBIKAN_NOT_A_SQLITE_OPTION"));
        assert!(!super::compile_option_used("BAD\0OPTION"));
    }

    #[test]
    fn vfs_identity_wrapper_returns_a_single_default_head() {
        let identities = super::registered_vfses().expect("SQLite VFS names are UTF-8");
        assert!(!identities.is_empty());
        assert!(identities[0].is_default);
        assert!(identities.iter().skip(1).all(|identity| !identity.is_default));
    }
}
