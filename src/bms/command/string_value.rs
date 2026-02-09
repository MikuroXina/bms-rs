use std::fmt::Debug;
use std::str::FromStr;

/// Type alias for the error type of a `FromStr` implementation.
type ParseError<T> = <T as FromStr>::Err;

/// A wrapper that preserves the original string representation of a value.
/// The value is stored as a Result to preserve parsing errors.
pub struct StringValue<T: FromStr>
where
    ParseError<T>: Debug + Clone + PartialEq + Eq,
{
    raw: String,
    value: Result<T, ParseError<T>>,
}

impl<T> StringValue<T>
where
    T: FromStr,
    ParseError<T>: Debug + Clone + PartialEq + Eq,
{
    /// Creates a `StringValue` from a value, generating a string representation.
    pub fn from_value(value: T) -> Self
    where
        T: ToString,
    {
        Self {
            raw: value.to_string(),
            value: Ok(value),
        }
    }

    /// Creates a `StringValue` from a raw string.
    #[must_use]
    pub fn new(raw: String) -> Self {
        let value = T::from_str(&raw);
        Self { raw, value }
    }

    /// Returns a reference to the parsed value.
    pub const fn value(&self) -> &Result<T, ParseError<T>> {
        &self.value
    }

    /// Returns a reference to the raw string.
    pub fn raw(&self) -> &str {
        &self.raw
    }
}

// Implement Debug manually
impl<T: Debug + FromStr> Debug for StringValue<T>
where
    ParseError<T>: Debug + Clone + PartialEq + Eq,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StringValue")
            .field("raw", &self.raw)
            .field("value", &self.value)
            .finish()
    }
}

// Implement Clone manually
impl<T: FromStr + Clone> Clone for StringValue<T>
where
    ParseError<T>: Debug + Clone + PartialEq + Eq,
{
    fn clone(&self) -> Self {
        Self {
            raw: self.raw.clone(),
            value: self.value.clone(),
        }
    }
}

// Implement PartialEq manually
impl<T: FromStr + PartialEq> PartialEq for StringValue<T>
where
    ParseError<T>: Debug + Clone + PartialEq + Eq,
{
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw && self.value == other.value
    }
}

// Implement Eq manually
impl<T: FromStr + PartialEq> Eq for StringValue<T> where
    ParseError<T>: Debug + Clone + PartialEq + Eq
{
}

impl<T> AsRef<Result<T, ParseError<T>>> for StringValue<T>
where
    T: FromStr,
    ParseError<T>: Debug + Clone + PartialEq + Eq,
{
    fn as_ref(&self) -> &Result<T, ParseError<T>> {
        &self.value
    }
}

impl<T: std::fmt::Display + FromStr> std::fmt::Display for StringValue<T>
where
    ParseError<T>: Debug + Clone + PartialEq + Eq,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Display the raw string representation
        std::fmt::Display::fmt(&self.raw, f)
    }
}

// Serde support - manually implement
#[cfg(feature = "serde")]
impl<T: FromStr> serde::Serialize for StringValue<T>
where
    ParseError<T>: Debug + Clone + PartialEq + Eq,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.raw.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, T: FromStr> serde::Deserialize<'de> for StringValue<T>
where
    ParseError<T>: Debug + Clone + PartialEq + Eq,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Ok(Self::new(raw))
    }
}
