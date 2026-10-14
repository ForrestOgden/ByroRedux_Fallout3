//! Minimal, native iOS-compatible ABI over the real ByroRedux archive reader.
//! Not a port of world simulation, graphics, or scripting.

use byroredux_bsa::BsaArchive;
use std::ffi::{c_char, CStr};

/// ABI version for the Swift host. Nonzero means a linked Rust library,
/// *not* successful Fallout 3 gameplay.
#[unsafe(no_mangle)]
pub extern "C" fn fo3_ios_abi_version() -> u32 {
    1
}

/// Inspect the header/table of a local BSA using ByroRedux's reader.
///
/// # Safety
/// `path` must be a valid, NUL-terminated C string for the duration
/// of the call, or null.
/// Returns: 0 valid BSA, -1 null, -2 non-UTF8 path, 1 unreadable/invalid BSA.
/// This never copies or uploads any game asset bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fo3_ios_probe_bsa(path: *const c_char) -> i32 {
    if path.is_null() {
        return -1;
    }
    // SAFETY: caller guarantees a live NUL-terminated pointer.
    let c_path = unsafe { CStr::from_ptr(path) };
    let Ok(path) = c_path.to_str() else {
        return -2;
    };
    match BsaArchive::open(path) {
        Ok(_) => 0,
        Err(_) => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn abi_is_versioned() {
        assert_eq!(fo3_ios_abi_version(), 1);
    }

    #[test]
    fn null_path_is_rejected() {
        assert_eq!(unsafe { fo3_ios_probe_bsa(std::ptr::null()) }, -1);
    }

    #[test]
    fn nonexistent_archive_is_rejected_without_reading_game_data() {
        let p = CString::new("/__fallout3_ios_uninstalled__/absent.bsa").unwrap();
        assert_eq!(unsafe { fo3_ios_probe_bsa(p.as_ptr()) }, 1);
    }
}
