mod impls;

use core::convert::Infallible;
use std::ffi::{OsStr, OsString};

pub use self::impls::*;
use crate::string::os_str::RAW_EMPTY;
use crate::string::Str;

impl Str for OsStr {
    type StringType = OsString;
    type HeapType = OsStr;
    type ConvertError = Infallible;

    #[cfg(unix)]
    #[inline]
    fn from_inline_data(bytes: &[u8]) -> &Self {
        use std::os::unix::ffi::OsStrExt;
        OsStr::from_bytes(bytes)
    }

    #[cfg(not(unix))]
    #[inline]
    fn from_inline_data(_bytes: &[u8]) -> &Self {
        // TODO: Does os_str_bytes have a feature to help with this? Didn't see one
        unreachable!("Raw byte slice conversion not supported on this platform");
    }

    #[inline]
    fn from_heap_data(bytes: &Self::HeapType) -> &Self {
        bytes
    }

    #[cfg(unix)]
    #[inline]
    fn try_from_raw_data(bytes: &[u8]) -> Result<&Self, Self::ConvertError> {
        Ok(Self::from_inline_data(bytes))
    }

    #[cfg(not(unix))]
    #[inline]
    fn try_from_raw_data(bytes: &[u8]) -> Result<&Self, Self::ConvertError> {
        // TODO: Use os_str_bytes for platforms other than unix
        unreachable!("Raw byte slice conversion not supported on this platform")
    }

    #[cfg(unix)]
    #[inline(always)]
    fn empty(&self) -> Option<&'static Self> {
        if self.length() == 0 {
            Some(Self::from_inline_data(RAW_EMPTY))
        } else {
            None
        }
    }

    #[cfg(not(unix))]
    #[inline(always)]
    fn empty(&self) -> Option<&'static Self> {
        None
    }

    #[inline(always)]
    fn length(&self) -> usize {
        self.len()
    }

    #[inline]
    fn as_heap_type(&self) -> &Self::HeapType {
        self
    }

    #[cfg(unix)]
    #[inline(always)]
    fn as_inline_ptr(&self) -> *const u8 {
        use std::os::unix::ffi::OsStrExt;
        self.as_bytes() as *const [u8] as *const u8
    }

    #[cfg(not(unix))]
    #[inline]
    fn as_inline_ptr(&self) -> *const u8 {
        // TODO: Does os_str_bytes have a feature to help with this? Didn't see one
        unreachable!("Conversion back to raw pointer not supported on this platform");
    }
}