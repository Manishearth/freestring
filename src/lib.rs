extern crate libc;
extern crate memchr;

use memchr::memchr;
use std::{ffi, mem, ops, ptr, slice};

/// Rust's CString, but is safe to free
/// via `free()`. We guarantee this will
/// always have been allocated via `malloc`,
/// and always will be freed via `free()`.
pub struct FreeString {
    inner: *const u8,
    len: usize,
}

pub struct NulError(usize);
pub enum FromBytesWithNulError {
    NotNulTerminated,
    InteriorNul(usize),
}


impl FreeString {
    /// Construct from a byte buffer. Will return an error if any
    /// byte is null
    pub fn new(bytes: &[u8]) -> Result<Self, NulError> {
        match memchr(0, &bytes) {
            Some(i) => Err(NulError(i)),
            None => Ok(unsafe { Self::from_bytes_unchecked(bytes) }),
        }
    }

    /// Construct from a null terminated byte buffer. Will return an error
    /// if the last byte is not null, or if any other byte is null.
    pub fn from_bytes_with_nul(bytes: &[u8])
                               -> Result<FreeString, FromBytesWithNulError> {
        let nul_pos = memchr::memchr(0, bytes);
        if let Some(nul_pos) = nul_pos {
            if nul_pos + 1 != bytes.len() {
                return Err(FromBytesWithNulError::InteriorNul(nul_pos));
            }
            Ok(unsafe { Self::from_bytes_with_nul_unchecked(bytes) })
        } else {
            Err(FromBytesWithNulError::NotNulTerminated)
        }
    }

    /// Construct from some bytes which we know contain no null. This
    /// function will append a null terminator whilst constructing.
    ///
    /// Invariants:
    ///
    /// - No byte can be null
    pub unsafe fn from_bytes_unchecked(bytes: &[u8]) -> Self {
        // we turbofish [u8] here to ensure that we don't accidentally
        // size_of_val on &&[u8] or something
        let size = mem::size_of_val::<[u8]>(bytes);
        let buf = libc::malloc(size + mem::size_of::<u8>()) as *mut u8;

        if buf.is_null() {
            panic!("Out of memory")
        }

        let total_len = bytes.len().checked_add(1).expect("Overflow while allocating");

        ptr::copy_nonoverlapping(/* src */ bytes.as_ptr(),
                                 /* dest */ buf,
                                 /* len */ bytes.len());


        let slice = slice::from_raw_parts_mut(buf, total_len);

        // shove in a null terminator
        slice[total_len - 1] = 0;

        FreeString {
            inner: buf,
            len: total_len,
        }
    }

    /// Construct from some bytes which we know are null terminated.
    ///
    /// Invariants:
    ///
    /// - Last byte must be null
    /// - No other byte can be null
    pub unsafe fn from_bytes_with_nul_unchecked(bytes: &[u8]) -> Self {
        // we turbofish [u8] here to ensure that we don't accidentally
        // size_of_val on &&[u8] or something
        let size = mem::size_of_val::<[u8]>(bytes);
        let buf = libc::malloc(size) as *mut u8;

        if buf.is_null() {
            panic!("Out of memory")
        }

        ptr::copy_nonoverlapping(/* src */ bytes.as_ptr(),
                                 /* dest */ buf,
                                 /* len */ bytes.len());


        FreeString {
            inner: buf,
            len: bytes.len(),
        }
    }

    /// Get a raw pointer to the inner string. Suitable for giving to C
    #[inline]
    pub fn as_raw(&self) -> *const u8 {
        self.inner
    }

    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.inner, self.len) }
    }
}

impl Drop for FreeString {
    fn drop(&mut self) {
        unsafe { libc::free(self.inner as *mut u8 as *mut _) }
    }
}

impl ops::Deref for FreeString {
    type Target = ffi::CStr;    
    // the lifetime here isn't necessary, but it's
    // helpful to be clear here. from_bytes_with_nul_unchecked
    // will return a &'static ffi::CStr because the input
    // was 'static, and we don't want that to happen
    fn deref<'a>(&'a self) -> &'a ffi::CStr {
        unsafe { ffi::CStr::from_bytes_with_nul_unchecked(self.as_slice()) }
    }
}