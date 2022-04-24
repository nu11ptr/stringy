mod impls;

use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;
use core::convert::Infallible;

pub use self::impls::*;
use crate::string::{Str, Utf8Error};

/// Empty raw string constant
pub const EMPTY: &[u8] = b"";

impl Str for [u8] {
    type StringType = Vec<u8>;
    type HeapType = [u8];
    type ConvertError = Infallible;

    #[inline]
    fn from_inline_data(bytes: &[u8]) -> &Self {
        bytes
    }

    #[inline]
    fn from_heap_data(bytes: &Self::HeapType) -> &Self {
        Self::from_inline_data(bytes)
    }

    #[inline]
    fn try_from_raw_data(bytes: &[u8]) -> Result<&Self, Self::ConvertError> {
        Ok(Self::from_inline_data(bytes))
    }

    #[inline(always)]
    fn empty(&self) -> Option<&'static Self> {
        if self.length() == 0 {
            Some(EMPTY)
        } else {
            None
        }
    }

    #[inline(always)]
    fn length(&self) -> usize {
        self.len()
    }

    #[inline]
    fn as_heap_type(&self) -> &Self::HeapType {
        self
    }

    #[inline(always)]
    fn as_inline_ptr(&self) -> *const u8 {
        self.as_ptr()
    }

    #[inline]
    fn to_string_type(&self) -> Self::StringType {
        self.to_vec()
    }

    #[inline(always)]
    fn try_to_str(&self) -> Result<&str, Utf8Error> {
        core::str::from_utf8(self).map_err(|err| Utf8Error::WithData {
            valid_up_to: err.valid_up_to(),
            error_len: err.error_len(),
        })
    }

    #[inline(always)]
    fn to_string_lossy(&self) -> Cow<str> {
        String::from_utf8_lossy(self)
    }
}

impl<'str, const SIZE: usize, const BPAD: usize, const HPAD: usize, HEAP>
    FlexRawStr<'str, SIZE, BPAD, HPAD, HEAP>
{
    /// An empty ("") static constant string
    pub const EMPTY: Self = Self::from_static(EMPTY);
}
