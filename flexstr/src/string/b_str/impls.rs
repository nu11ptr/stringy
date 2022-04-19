#![cfg(feature = "bstr")]

use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::sync::Arc;

use bstr::BStr;
use paste::paste;

use crate::inner::FlexStrInner;
use crate::string::Str;
use crate::traits::private;
use crate::traits::private::FlexStrCoreInner;
use crate::{define_flex_types, FlexStrCore, FlexStrCoreRef, Storage};

pub(crate) const RAW_EMPTY: &[u8] = b"";

define_flex_types!("BStr", BStr, [u8]);

macro_rules! impl_body {
    () => {
        /// Creates a wrapped static string literal from a raw byte slice.
        #[inline]
        pub fn from_static_raw(s: &'static [u8]) -> Self {
            // There are no `const fn` functions in BStr to do this so we use trait
            Self(FlexStrInner::from_static(BStr::from_inline_data(s)))
        }
    };
}

// *** FlexBStr ***

impl<const SIZE: usize, const BPAD: usize, const HPAD: usize, HEAP>
    FlexBStr<SIZE, BPAD, HPAD, HEAP>
{
    impl_body!();
}

impl<const SIZE: usize, const BPAD: usize, const HPAD: usize, HEAP>
    FlexStrCore<'static, SIZE, BPAD, HPAD, HEAP, BStr> for FlexBStr<SIZE, BPAD, HPAD, HEAP>
where
    HEAP: Storage<BStr>,
{
    #[inline(always)]
    fn as_str_type(&self) -> &BStr {
        self.inner().as_str_type()
    }
}

// *** FlexBStrRef ***

impl<'str, const SIZE: usize, const BPAD: usize, const HPAD: usize, HEAP>
    FlexBStrRef<'str, SIZE, BPAD, HPAD, HEAP>
{
    impl_body!();
}

impl<'str, const SIZE: usize, const BPAD: usize, const HPAD: usize, HEAP>
    FlexStrCore<'str, SIZE, BPAD, HPAD, HEAP, BStr> for FlexBStrRef<'str, SIZE, BPAD, HPAD, HEAP>
where
    HEAP: Storage<BStr>,
{
    #[inline(always)]
    fn as_str_type(&self) -> &BStr {
        self.inner().as_str_type()
    }
}