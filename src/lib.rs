#![no_std]
#![warn(missing_docs)]

//! A flexible, simple to use, immutable, clone-efficient `String` replacement for Rust
//!
//! ```
//! use flexstr::{flex_fmt, FlexStr, IntoFlexStr, ToCase, ToFlexStr};
//!
//! // Use an `into` function to wrap a literal, no allocation or copying
//! let static_str = "This will not allocate or copy".into_flex_str();
//! assert!(static_str.is_static());
//!
//! // Strings up to 22 bytes (on 64-bit) will be inlined automatically
//! // (demo only, use `into` for literals as above)
//! let inline_str = "inlined".to_flex_str();
//! assert!(inline_str.is_inlined());
//!
//! // When a string is too long to be wrapped/inlined, it will heap allocate
//! // (demo only, use `into` for literals as above)
//! let rc_str = "This is too long to be inlined".to_flex_str();
//! assert!(rc_str.is_heap());
//!
//! // You can efficiently create a new `FlexStr` (without creating a `String`)
//! // This is equivalent to the stdlib `format!` macro
//! let inline_str2 = flex_fmt!("in{}", "lined");
//! assert!(inline_str2.is_inlined());
//! assert_eq!(inline_str, inline_str2);
//!
//! // We can upper/lowercase strings without converting to a `String` first
//! // This doesn't heap allocate since inlined
//! let inline_str3: FlexStr = "INLINED".to_ascii_lower();
//! assert!(inline_str3.is_inlined());
//! assert_eq!(inline_str, inline_str3);
//!
//! // Concatenation doesn't even copy if we can fit it in the inline string
//! let inline_str4 = inline_str3 + "!!!";
//! assert!(inline_str4.is_inlined());
//! assert_eq!(inline_str4, "inlined!!!");
//!
//! // Clone is almost free, and never allocates
//! // (at most it is a ref count increment for heap allocated strings)
//! let static_str2 = static_str.clone();
//! assert!(static_str2.is_static());
//!
//! // Regardless of storage type, these all operate seamlessly together
//! // and choose storage as required
//! let heap_str2 = static_str2 + &inline_str;
//! assert!(heap_str2.is_heap());
//! assert_eq!(heap_str2, "This will not allocate or copyinlined");  
//! ```

extern crate alloc;

mod builder;
mod inline;
mod traits;

pub use traits::*;

use alloc::rc::Rc;
use alloc::string::String;
use alloc::sync::Arc;
use core::borrow::Borrow;
use core::cmp::Ordering;
use core::convert::Infallible;
use core::fmt;
use core::fmt::{Arguments, Debug, Display, Formatter, Write};
use core::hash::{Hash, Hasher};
#[cfg(feature = "serde")]
use core::marker::PhantomData;
use core::ops::{
    Add, Deref, Index, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive,
};
use core::str::FromStr;

use crate::inline::InlineFlexStr;
#[cfg(feature = "serde")]
use serde::de::{Error, Visitor};
#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone)]
enum FlexStrInner<T> {
    /// A wrapped string literal
    Static(&'static str),
    /// An inlined string
    Inlined(inline::InlineFlexStr),
    /// A reference count wrapped `str`
    Heap(T),
}

/// A flexible string type that transparently wraps a string literal, inline string, or an `Rc<str>`
#[derive(Clone)]
pub struct FlexStr<T = Rc<str>>(FlexStrInner<T>);

/// A flexible string type that transparently wraps a string literal, inline string, or an `Arc<str>`
pub type AFlexStr = FlexStr<Arc<str>>;

impl<T> FlexStr<T>
where
    T: Deref<Target = str>,
{
    /// Creates a wrapped static string literal
    #[inline]
    pub fn from_static(s: &'static str) -> FlexStr<T> {
        FlexStr(FlexStrInner::Static(s))
    }

    /// Attempts to create an inlined string. Returns new inline string on success or original source
    /// string as `Err` if it will not fit.
    #[inline]
    pub fn try_inline(s: &str) -> Result<FlexStr<T>, &str> {
        match InlineFlexStr::try_new(s) {
            Ok(s) => Ok(FlexStr(FlexStrInner::Inlined(s))),
            Err(s) => Err(s),
        }
    }

    /// Force the creation of a heap allocated string. Unlike into functions, this will not attempt
    /// to inline first even if the string is a candidate for inlining.
    #[inline]
    pub fn heap(s: &str) -> FlexStr<T>
    where
        T: for<'a> From<&'a str>,
    {
        FlexStr(FlexStrInner::Heap(s.into()))
    }

    /// Returns true if this `FlexStr` is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        str::is_empty(self)
    }

    /// Returns the length of this `FlexStr` in bytes (not chars/graphemes)
    #[inline]
    pub fn len(&self) -> usize {
        str::len(self)
    }

    /// Extracts a string slice containing the entire `FlexStr`
    #[inline]
    pub fn as_str(&self) -> &str {
        &**self
    }

    /// Returns true if this is a wrapped string literal (`&'static str`)
    #[inline]
    pub fn is_static(&self) -> bool {
        matches!(self.0, FlexStrInner::Static(_))
    }

    /// Returns true if this is an inlined string
    #[inline]
    pub fn is_inlined(&self) -> bool {
        matches!(self.0, FlexStrInner::Inlined(_))
    }

    /// Returns true if this is a wrapped string using heap storage
    #[inline]
    pub fn is_heap(&self) -> bool {
        matches!(self.0, FlexStrInner::Heap(_))
    }
}

// *** Deref / Debug / Display ***

impl<T> Deref for FlexStr<T>
where
    T: Deref<Target = str>,
{
    type Target = str;

    /// ```
    /// use flexstr::IntoFlexStr;
    ///
    /// let a = "test";
    /// let b = a.into_flex_str();
    /// assert_eq!(&*b, a);
    /// ```
    #[inline]
    fn deref(&self) -> &Self::Target {
        match &self.0 {
            FlexStrInner::Static(str) => str,
            FlexStrInner::Inlined(ss) => ss,
            FlexStrInner::Heap(rc) => rc,
        }
    }
}

impl<T> Debug for FlexStr<T>
where
    T: Deref<Target = str>,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        <str as Debug>::fmt(self, f)
    }
}

impl<T> Display for FlexStr<T>
where
    T: Deref<Target = str>,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        <str as Display>::fmt(self, f)
    }
}

// *** Hash, PartialEq, Eq ***

impl<T> Hash for FlexStr<T>
where
    T: Deref<Target = str>,
{
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        str::hash(self, state)
    }
}

impl<T, T2> PartialEq<FlexStr<T2>> for FlexStr<T>
where
    T: Deref<Target = str>,
    T2: Deref<Target = str>,
{
    /// ```
    /// use flexstr::{AFlexStr, FlexStr, ToFlex};
    ///
    /// let lit = "inlined";
    /// let s: FlexStr = lit.into();
    /// let s2: AFlexStr = lit.into();
    /// assert_eq!(s, s2);
    /// ```
    #[inline]
    fn eq(&self, other: &FlexStr<T2>) -> bool {
        str::eq(self, &**other)
    }
}

impl<T, T2> PartialEq<FlexStr<T2>> for &FlexStr<T>
where
    T: Deref<Target = str>,
    T2: Deref<Target = str>,
{
    /// ```
    /// use flexstr::{AFlexStr, FlexStr, ToFlex};
    ///
    /// let lit = "inlined";
    /// let s: FlexStr = lit.into();
    /// let s2: AFlexStr = lit.into();
    /// assert_eq!(&s, s2);
    /// ```
    #[inline]
    fn eq(&self, other: &FlexStr<T2>) -> bool {
        str::eq(self, &**other)
    }
}

impl<T> PartialEq<&str> for FlexStr<T>
where
    T: Deref<Target = str>,
{
    /// ```
    /// use flexstr::{FlexStr, ToFlex};
    ///
    /// let lit = "inlined";
    /// let s: FlexStr = lit.to_flex();
    /// assert_eq!(s, lit);
    /// ```
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        str::eq(self, *other)
    }
}

impl<T> PartialEq<str> for FlexStr<T>
where
    T: Deref<Target = str>,
{
    /// ```
    /// use flexstr::{FlexStr, ToFlex};
    ///
    /// let lit = "inlined";
    /// let s: FlexStr = lit.to_flex();
    /// assert_eq!(s, lit);
    /// ```
    #[inline]
    fn eq(&self, other: &str) -> bool {
        str::eq(self, other)
    }
}

impl<T> PartialEq<String> for FlexStr<T>
where
    T: Deref<Target = str>,
{
    /// ```
    /// use flexstr::FlexStr;
    ///
    /// let lit = "inlined";
    /// let s: FlexStr = lit.into();
    /// assert_eq!(s, lit.to_string());
    /// ```
    #[inline]
    fn eq(&self, other: &String) -> bool {
        str::eq(self, other)
    }
}

impl<T> Eq for FlexStr<T> where T: Deref<Target = str> {}

// *** PartialOrd / Ord ***

impl<T> PartialOrd for FlexStr<T>
where
    T: Deref<Target = str>,
{
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        str::partial_cmp(self, other)
    }
}

impl<T> PartialOrd<str> for FlexStr<T>
where
    T: Deref<Target = str>,
{
    #[inline]
    fn partial_cmp(&self, other: &str) -> Option<Ordering> {
        str::partial_cmp(self, other)
    }
}

impl<T> PartialOrd<String> for FlexStr<T>
where
    T: Deref<Target = str>,
{
    #[inline]
    fn partial_cmp(&self, other: &String) -> Option<Ordering> {
        str::partial_cmp(self, other)
    }
}

impl<T> Ord for FlexStr<T>
where
    T: Deref<Target = str>,
{
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        str::cmp(self, other)
    }
}

// *** Index ***

impl<T> Index<Range<usize>> for FlexStr<T>
where
    T: Deref<Target = str>,
{
    type Output = str;

    #[inline]
    fn index(&self, index: Range<usize>) -> &Self::Output {
        str::index(self, index)
    }
}

impl<T> Index<RangeTo<usize>> for FlexStr<T>
where
    T: Deref<Target = str>,
{
    type Output = str;

    #[inline]
    fn index(&self, index: RangeTo<usize>) -> &Self::Output {
        str::index(self, index)
    }
}

impl<T> Index<RangeFrom<usize>> for FlexStr<T>
where
    T: Deref<Target = str>,
{
    type Output = str;

    #[inline]
    fn index(&self, index: RangeFrom<usize>) -> &Self::Output {
        str::index(self, index)
    }
}

impl<T> Index<RangeFull> for FlexStr<T>
where
    T: Deref<Target = str>,
{
    type Output = str;

    #[inline]
    fn index(&self, index: RangeFull) -> &Self::Output {
        str::index(self, index)
    }
}

impl<T> Index<RangeInclusive<usize>> for FlexStr<T>
where
    T: Deref<Target = str>,
{
    type Output = str;

    #[inline]
    fn index(&self, index: RangeInclusive<usize>) -> &Self::Output {
        str::index(self, index)
    }
}

impl<T> Index<RangeToInclusive<usize>> for FlexStr<T>
where
    T: Deref<Target = str>,
{
    type Output = str;

    #[inline]
    fn index(&self, index: RangeToInclusive<usize>) -> &Self::Output {
        str::index(self, index)
    }
}

// *** Add ***

fn concat<T>(s1: &str, s2: &str) -> FlexStr<T>
where
    T: From<String> + for<'a> From<&'a str>,
{
    let mut builder = builder::FlexStrBuilder::with_capacity(s1.len() + s2.len());
    unsafe {
        // SAFETY: write_str always succeeds
        builder.write_str(s1).unwrap_unchecked();
        builder.write_str(s2).unwrap_unchecked();
    }
    builder.into()
}

impl<T> Add<&str> for FlexStr<T>
where
    T: From<String> + for<'a> From<&'a str> + Deref<Target = str>,
{
    type Output = FlexStr<T>;

    /// ```
    /// use flexstr::IntoFlexStr;
    ///
    /// let a = "in".into_flex_str() + "line";
    /// assert!(a.is_inlined());
    /// assert_eq!(a, "inline");
    ///
    /// let a = "in".to_string().into_flex_str() + "line";
    /// assert!(a.is_inlined());
    /// assert_eq!(a, "inline");
    /// ```
    #[inline]
    fn add(self, rhs: &str) -> Self::Output {
        match self.0 {
            FlexStrInner::Static(s) => concat(s, rhs),
            FlexStrInner::Inlined(mut s) => {
                if s.try_concat(rhs) {
                    FlexStr(FlexStrInner::Inlined(s))
                } else {
                    concat(&s, rhs)
                }
            }
            FlexStrInner::Heap(s) => concat(&s, rhs),
        }
    }
}

// *** Misc. standard traits ***

impl<T> AsRef<str> for FlexStr<T>
where
    T: Deref<Target = str>,
{
    #[inline]
    fn as_ref(&self) -> &str {
        self
    }
}

impl<T> Default for FlexStr<T>
where
    T: Deref<Target = str>,
{
    #[inline]
    fn default() -> Self {
        Self::from_static("")
    }
}

impl<T> Borrow<str> for FlexStr<T>
where
    T: Deref<Target = str>,
{
    #[inline]
    fn borrow(&self) -> &str {
        str::borrow(self)
    }
}

impl<T> FromStr for FlexStr<T>
where
    T: for<'a> From<&'a str>,
{
    type Err = Infallible;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.to_flex())
    }
}

// *** From ***

impl<T, T2> From<&FlexStr<T2>> for FlexStr<T>
where
    T2: Clone,
    FlexStr<T>: From<FlexStr<T2>>,
{
    #[inline]
    fn from(s: &FlexStr<T2>) -> Self {
        s.clone().into()
    }
}

impl<T> From<builder::FlexStrBuilder> for FlexStr<T>
where
    T: From<String> + for<'a> From<&'a str>,
{
    #[inline]
    fn from(builder: builder::FlexStrBuilder) -> Self {
        match builder {
            builder::FlexStrBuilder::Small(buffer) => {
                let len: u8 = buffer.len() as u8;
                FlexStr(FlexStrInner::Inlined(inline::InlineFlexStr::from_array(
                    buffer.into_inner(),
                    len,
                )))
            }
            builder::FlexStrBuilder::Regular(buffer) => buffer.to_flex(),
            builder::FlexStrBuilder::Large(s) => s.into(),
        }
    }
}

impl<T> From<String> for FlexStr<T>
where
    T: From<String>,
{
    /// ```
    /// use flexstr::FlexStr;
    ///
    /// let lit = "inlined";
    /// let s: FlexStr = lit.to_string().into();
    /// assert!(s.is_inlined());
    /// assert_eq!(&s, lit);
    ///
    /// let lit = "This is too long too be inlined!";
    /// let s: FlexStr = lit.to_string().into();
    /// assert!(s.is_heap());
    /// assert_eq!(&s, lit);
    /// ```
    #[inline]
    fn from(s: String) -> Self {
        FlexStr(match s.try_into() {
            Ok(s) => FlexStrInner::Inlined(s),
            Err(s) => FlexStrInner::Heap(s.into()),
        })
    }
}

impl<T> From<&String> for FlexStr<T>
where
    T: for<'a> From<&'a str>,
{
    /// ```
    /// use flexstr::FlexStr;
    ///
    /// let lit = "inlined";
    /// let s: FlexStr = (&lit.to_string()).into();
    /// assert!(s.is_inlined());
    /// assert_eq!(&s, lit);
    ///
    /// let lit = "This is too long too be inlined!";
    /// let s: FlexStr = (&lit.to_string()).into();
    /// assert!(s.is_heap());
    /// assert_eq!(&s, lit);
    /// ```
    #[inline]
    fn from(s: &String) -> Self {
        s.to_flex()
    }
}

impl<T> From<&'static str> for FlexStr<T>
where
    T: Deref<Target = str>,
{
    /// ```
    /// use flexstr::FlexStr;
    ///
    /// let lit = "static";
    /// let s: FlexStr  = lit.into();
    /// assert!(s.is_static());
    /// assert_eq!(&s, lit);
    /// ```
    #[inline]
    fn from(s: &'static str) -> Self {
        Self::from_static(s)
    }
}

impl<T> From<char> for FlexStr<T>
where
    T: From<String> + for<'a> From<&'a str> + Deref<Target = str>,
{
    /// ```
    /// use flexstr::FlexStr;
    ///
    /// let s: FlexStr  = 't'.into();
    /// assert!(s.is_inlined());
    /// assert_eq!(&s, "t");
    /// ```
    #[inline]
    fn from(ch: char) -> Self {
        // SAFETY: Regardless of architecture, 4 bytes will always fit in an inline string
        unsafe { Self::try_inline(ch.encode_utf8(&mut [0; 4])).unwrap_unchecked() }
    }
}

// *** FromIterator ***

fn from_iter_str<I, T, U>(iter: I) -> FlexStr<T>
where
    I: IntoIterator<Item = U>,
    T: From<String> + for<'b> From<&'b str>,
    U: AsRef<str>,
{
    let iter = iter.into_iter();

    // Since `IntoIterator` consumes, we cannot loop over it twice to find lengths of strings
    // for a good capacity # without cloning it (which might be expensive)
    let mut builder = builder::FlexStrBuilder::new();
    for s in iter {
        // SAFETY: Always succeeds
        unsafe {
            builder.write_str(s.as_ref()).unwrap_unchecked();
        }
    }
    builder.into()
}

fn from_iter_char<I, F, T, U>(iter: I, f: F) -> FlexStr<T>
where
    I: IntoIterator<Item = U>,
    F: Fn(U) -> char,
    T: From<String> + for<'b> From<&'b str>,
{
    let iter = iter.into_iter();
    let (lower, _) = iter.size_hint();

    let mut builder = builder::FlexStrBuilder::with_capacity(lower);
    for ch in iter {
        // SAFETY: Always succeeds
        unsafe {
            builder.write_char(f(ch)).unwrap_unchecked();
        }
    }
    builder.into()
}

impl<T, T2> FromIterator<FlexStr<T2>> for FlexStr<T>
where
    T: From<String> + for<'b> From<&'b str>,
    T2: Deref<Target = str>,
{
    /// ```
    /// use flexstr::{FlexStr};
    ///
    /// let v: Vec<FlexStr> = vec!["best".into(), "test".into()];
    /// let s: FlexStr = v.into_iter().map(|s| if s == "best" { "test".into() } else { s }).collect();
    /// assert!(s.is_inlined());
    /// assert_eq!(s, "testtest");
    /// ```
    #[inline]
    fn from_iter<I: IntoIterator<Item = FlexStr<T2>>>(iter: I) -> Self {
        from_iter_str(iter)
    }
}

impl<'a, T, T2> FromIterator<&'a FlexStr<T2>> for FlexStr<T>
where
    T: From<String> + for<'b> From<&'b str>,
    T2: Deref<Target = str> + 'a,
{
    /// ```
    /// use flexstr::{FlexStr};
    ///
    /// let v: Vec<FlexStr> = vec!["best".into(), "test".into()];
    /// let s: FlexStr = v.iter().filter(|s| *s == "best").collect();
    /// assert!(s.is_inlined());
    /// assert_eq!(s, "best");
    /// ```
    #[inline]
    fn from_iter<I: IntoIterator<Item = &'a FlexStr<T2>>>(iter: I) -> Self {
        from_iter_str(iter)
    }
}

impl<T> FromIterator<String> for FlexStr<T>
where
    T: From<String> + for<'b> From<&'b str>,
{
    /// ```
    /// use flexstr::{FlexStr};
    ///
    /// let v = vec!["best".to_string(), "test".to_string()];
    /// let s: FlexStr = v.into_iter().map(|s| if s == "best" { "test".into() } else { s }).collect();
    /// assert!(s.is_inlined());
    /// assert_eq!(s, "testtest");
    /// ```
    #[inline]
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        from_iter_str(iter)
    }
}

impl<'a, T> FromIterator<&'a str> for FlexStr<T>
where
    T: From<String> + for<'b> From<&'b str>,
{
    /// ```
    /// use flexstr::{FlexStr};
    ///
    /// let v = vec!["best", "test"];
    /// let s: FlexStr = v.into_iter().map(|s| if s == "best" { "test" } else { s }).collect();
    /// assert!(s.is_inlined());
    /// assert_eq!(s, "testtest");
    /// ```
    #[inline]
    fn from_iter<I: IntoIterator<Item = &'a str>>(iter: I) -> Self {
        from_iter_str(iter)
    }
}

impl<T> FromIterator<char> for FlexStr<T>
where
    T: From<String> + for<'b> From<&'b str>,
{
    /// ```
    /// use flexstr::{FlexStr};
    ///
    /// let v = "besttest";
    /// let s: FlexStr = v.chars().map(|c| if c == 'b' { 't' } else { c }).collect();
    /// assert!(s.is_inlined());
    /// assert_eq!(s, "testtest");
    /// ```
    #[inline]
    fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Self {
        from_iter_char(iter, |ch| ch)
    }
}

impl<'a, T> FromIterator<&'a char> for FlexStr<T>
where
    T: From<String> + for<'b> From<&'b str>,
{
    /// ```
    /// use flexstr::{FlexStr};
    ///
    /// let v = vec!['b', 'e', 's', 't', 't', 'e', 's', 't'];
    /// let s: FlexStr = v.iter().filter(|&ch| *ch != 'b').collect();
    /// assert!(s.is_inlined());
    /// assert_eq!(s, "esttest");
    /// ```
    #[inline]
    fn from_iter<I: IntoIterator<Item = &'a char>>(iter: I) -> Self {
        from_iter_char(iter, |ch| *ch)
    }
}

// *** Optional serialization support ***

#[cfg(feature = "serde")]
impl<T> Serialize for FlexStr<T>
where
    T: Deref<Target = str>,
{
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self)
    }
}

// Uses *const T because we don't want it to actually own a `T`
#[cfg(feature = "serde")]
struct FlexStrVisitor<T>(PhantomData<*const T>);

#[cfg(feature = "serde")]
impl<'de, T> Visitor<'de> for FlexStrVisitor<T>
where
    T: From<String> + for<'a> From<&'a str>,
{
    type Value = FlexStr<T>;

    #[inline]
    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("a string")
    }

    #[inline]
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(v.to_flex())
    }

    #[inline]
    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(v.into())
    }
}

#[cfg(feature = "serde")]
impl<'de, T> Deserialize<'de> for FlexStr<T>
where
    T: From<String> + for<'a> From<&'a str>,
{
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(FlexStrVisitor(PhantomData))
    }
}

/// `FlexStr` equivalent to `format` function from stdlib. Efficiently creates a native `FlexStr`
pub fn flex_fmt<T>(args: Arguments<'_>) -> FlexStr<T>
where
    T: From<String> + for<'a> From<&'a str>,
{
    // NOTE: We have a disadvantage to `String` because we cannot call `estimated_capacity()` on args
    // As such, we cannot assume a given needed capacity - we start with a stack allocated buffer
    // and only promote to a heap buffer if a write won't fit
    let mut builder = builder::FlexStrBuilder::new();
    builder
        .write_fmt(args)
        .expect("a formatting trait implementation returned an error");
    builder.into_flex()
}

/// `FlexStr` equivalent to `format!` macro from stdlib. Efficiently creates a native `FlexStr`
/// ```
/// use flexstr::flex_fmt;
///
/// let a = flex_fmt!("Is {}", "inlined");
/// assert!(a.is_inlined());
/// assert_eq!(a, "Is inlined")
/// ```
#[macro_export]
macro_rules! flex_fmt {
    ($($arg:tt)*) => {{
        let s: flexstr::FlexStr = flexstr::flex_fmt(format_args!($($arg)*));
        s
    }}
}

/// `AFlexStr` equivalent to `format!` macro from stdlib. Efficiently creates a native `AFlexStr`
/// ```
/// use flexstr::a_flex_fmt;
///
/// let a = a_flex_fmt!("Is {}", "inlined");
/// assert!(a.is_inlined());
/// assert_eq!(a, "Is inlined")
/// ```

#[macro_export]
macro_rules! a_flex_fmt {
    ($($arg:tt)*) => {{
        let s: flexstr::AFlexStr = flexstr::flex_fmt(format_args!($($arg)*));
        s
    }}
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "serde")]
    #[test]
    fn serialization() {
        use crate::{AFlexStr, FlexStr};
        use alloc::string::ToString;
        use serde_json::json;

        #[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
        struct Test {
            a: FlexStr,
            b: AFlexStr,
            c: FlexStr,
        }

        let a = "test";
        let b = "testing";
        let c = "testing testing testing testing testing testing testing testing testing";

        // Create our struct and values and verify storage
        let test = Test {
            a: a.into(),
            b: b.to_string().into(),
            c: c.to_string().into(),
        };
        assert!(test.a.is_static());
        assert!(test.b.is_inlined());
        assert!(test.c.is_heap());

        // Serialize and ensure our JSON value actually matches
        let val = serde_json::to_value(test.clone()).unwrap();
        assert_eq!(json!({"a": a, "b": b, "c": c}), val);

        // Deserialize and validate storage and contents
        let test2: Test = serde_json::from_value(val).unwrap();
        assert!(test2.a.is_inlined());
        assert!(test2.b.is_inlined());
        assert!(test2.c.is_heap());

        assert_eq!(&test, &test2);
    }
}
