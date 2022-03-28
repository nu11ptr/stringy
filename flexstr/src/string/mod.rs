pub(crate) mod b_str;
pub(crate) mod c_str;
pub(crate) mod os_str;
pub(crate) mod raw_str;
pub(crate) mod std_str;

/// Trait used for implementing a custom inner string type ([str], [OsStr](std::ffi::OsStr), [Cstr](std::ffi::CStr), etc.)
pub trait Str {
    /// Regular (typically [Vec]-based) heap allocate string type
    type StringType;
    /// Type held by the underlying heap storage
    type HeapType: ?Sized;
    /// Error returned when a conversion from raw type to representative type fails
    type ConvertError;

    /// Transforms a slice of the inline stored type into the final string type. This can't fail so
    /// it is only called when the data is already vetted to be valid
    fn from_inline_data(bytes: &[u8]) -> &Self;

    /// Transforms a slice of the heap stored type into the final string type. This can't fail so it
    ///is only called when the data is already vetted to be valid
    fn from_heap_data(bytes: &Self::HeapType) -> &Self;

    /// Tries to transform raw data that has not yet been vetted to the final string type. If it is not
    /// possible, a [Self::ConvertError] is returned
    fn try_from_raw_data(bytes: &[u8]) -> Result<&Self, Self::ConvertError>;

    /// If self is_empty return a static empty string. If not supported by this string type, None is returned
    fn empty(&self) -> Option<&'static Self>;

    /// Returns the storage length for this particular string in bytes (not the # of chars)
    fn length(&self) -> usize;

    /// Returns a representation of the storage type
    fn as_heap_type(&self) -> &Self::HeapType;

    /// Returns a representation of the inline type as a pointer
    fn as_inline_ptr(&self) -> *const u8;
}

#[doc(hidden)]
#[macro_export]
macro_rules! define_flex_types {
    ($ident:literal, $type:ty, $heap_type:ty) => {
        use $crate::custom::{PTR_SIZED_PAD, STRING_SIZED_INLINE};

        paste! {
            #[doc = concat!("A flexible string type that transparently wraps a string literal, inline string, or an [`Rc<",
            stringify!($heap_type), ">`](std::rc::Rc)")]
            ///
            /// # Note
            /// Since this is just a type alias for a generic type, full documentation can be found here: [FlexStrBase]

            // *** FlexStr ***
            pub type [<Flex $ident Str>]<'str, const SIZE: usize, const BPAD: usize, const HPAD: usize, HEAP> =
                FlexStrBase<'str, SIZE, BPAD, HPAD, HEAP, $type>;

            // *** FlexStrRef *** (no need to export atm - it is blank)
            type [<Flex $ident StrRef>]<'str, const SIZE: usize, const BPAD: usize, const HPAD: usize, HEAP> =
                FlexStrRefBase<'str, SIZE, BPAD, HPAD, HEAP, $type>;

            /// A flexible base string type that transparently wraps a string literal, inline string, or a custom `HEAP` type.
            ///
            /// It is three machine words in size (3x usize) and can hold 22 bytes of inline string data on 64-bit platforms.
            ///
            /// # Note
            /// Since this is just a type alias for a generic type, full documentation can be found here: [FlexStrBase]
            ///
            /// # Note 2
            /// Custom concrete types need to specify a `HEAP` type with an exact size of two machine words (16 bytes
            /// on 64-bit, and 8 bytes on 32-bit). Any other sized parameter will result in a runtime panic on string
            /// creation.

            // *** FlexStr3USize ***
            pub type [<Flex $ident Str3USize>]<HEAP> =
                [<Flex $ident Str>]<'static, STRING_SIZED_INLINE, PTR_SIZED_PAD, PTR_SIZED_PAD, HEAP>;

            /// A flexible base string type that transparently wraps a string literal, inline string, a custom `HEAP` type, or
            /// a borrowed string (with appropriate lifetime specified).
            ///
            /// It is three machine words in size (3x usize) and can hold 22 bytes of inline string data on 64-bit platforms.
            ///
            /// # Note
            /// Since this is just a type alias for a generic type, full documentation can be found here: [FlexStrBase]
            ///
            /// # Note 2
            /// Custom concrete types need to specify a `HEAP` type with an exact size of two machine words (16 bytes
            /// on 64-bit, and 8 bytes on 32-bit). Any other sized parameter will result in a runtime panic on string
            /// creation.

            // *** FlexStrRef3USize ***
            pub type [<Flex $ident StrRef3USize>]<'str, HEAP> =
                [<Flex $ident StrRef>]<'str, STRING_SIZED_INLINE, PTR_SIZED_PAD, PTR_SIZED_PAD, HEAP>;

            #[doc = concat!("A flexible string type that transparently wraps a string literal, inline string, or an [`Rc<",
            stringify!($heap_type), ">`](std::rc::Rc)")]
            ///
            /// # Note
            /// Since this is just a type alias for a generic type, full documentation can be found here: [FlexStrBase]
            pub type [<Local $ident Str>] = [<Flex $ident Str3USize>]<Rc<$heap_type>>;

            #[doc = concat!("A flexible string type that transparently wraps a string literal, inline string, or an [`Arc<",
            stringify!($heap_type), ">`](std::sync::Arc)")]
            ///
            /// # Note
            /// Since this is just a type alias for a generic type, full documentation can be found here: [FlexStrBase]
            pub type [<Shared $ident Str>] = [<Flex $ident Str3USize>]<Arc<$heap_type>>;

            #[doc = concat!("A flexible string type that transparently wraps a string literal, inline string, an [`Rc<",
            stringify!($heap_type), ">`](std::rc::Rc), or borrowed string (with appropriate lifetime)")]
            ///
            /// # Note
            /// Since this is just a type alias for a generic type, full documentation can be found here: [FlexStrBase]
            pub type [<Local $ident StrRef>]<'str> = [<Flex $ident StrRef3USize>]<'str, Rc<$heap_type>>;

            #[doc = concat!("A flexible string type that transparently wraps a string literal, inline string, an [`Arc<",
            stringify!($heap_type), ">`](std::sync::Arc), or borrowed string (with appropriate lifetime)")]
            ///
            /// # Note
            /// Since this is just a type alias for a generic type, full documentation can be found here: [FlexStrBase]
            pub type [<Shared $ident StrRef>]<'str> = [<Flex $ident StrRef3USize>]<'str, Arc<$heap_type>>;

            #[doc = concat!("A flexible string type that transparently wraps a string literal, inline string, or a [`Box<",
            stringify!($heap_type), ">`](std::boxed::Box)")]
            ///
            /// # Note
            #[doc = concat!("This type is included for convenience for those who need wrapped [`Box<", stringify!($heap_type),
            ">`](std::boxed::Box)")]
            #[doc = "support. Those who do not have this special use case are encouraged to use [Local" $ident "Str] or [Shared"
            $ident "Str] for much better clone performance (without copy or additional allocation)"]
            ///
            /// # Note 2
            /// Since this is just a type alias for a generic type, full documentation can be found here: [FlexStrBase]
            pub type [<Boxed $ident Str>] = [<Flex $ident Str3USize>]<Box<$heap_type>>;

            #[doc = concat!("A flexible string type that transparently wraps a string literal, inline string, an [`Box<",
            stringify!($heap_type), ">`](std::boxed::Box), or borrowed string (with appropriate lifetime)")]
            ///
            /// # Note
            #[doc = concat!("This type is included for convenience for those who need wrapped [`Box<", stringify!($heap_type),
            ">`](std::boxed::Box)")]
            #[doc = "support. Those who do not have this special use case are encouraged to use [Local" $ident "StrRef] or [Shared"
            $ident "StrRef] for much better clone performance (without copy or additional allocation)"]
            ///
            /// # Note 2
            /// Since this is just a type alias for a generic type, full documentation can be found here: [FlexStrBase]
            pub type [<Boxed $ident StrRef>]<'str> = [<Flex $ident StrRef3USize>]<'str, Box<$heap_type>>;
        }
    };
}