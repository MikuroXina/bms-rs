use std::fmt::Debug;
use std::str::FromStr;

/// A wrapper that preserves the original string representation of a value.
/// The value is stored as a Result to preserve parsing errors.
pub struct StringValue<T: FromStr>
where
    <T as FromStr>::Err: Debug + Clone + PartialEq + Eq,
{
    raw: String,
    value: Result<T, <T as FromStr>::Err>,
}

impl<T> StringValue<T>
where
    T: FromStr,
    <T as FromStr>::Err: Debug + Clone + PartialEq + Eq,
{
    /// Creates a new `StringValue` from a string.
    /// Parsing is performed immediately and the result is stored.
    pub fn new(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        let value = raw.parse::<T>();
        Self { raw, value }
    }
}

impl<T> StringValue<T>
where
    T: FromStr,
    <T as FromStr>::Err: Debug + Clone + PartialEq + Eq,
{
    /// Returns the original string representation.
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// Returns the parse result.
    pub const fn value(&self) -> &Result<T, <T as FromStr>::Err> {
        &self.value
    }
}

impl<T> StringValue<T>
where
    T: ToString + FromStr,
    <T as FromStr>::Err: Debug + Clone + PartialEq + Eq,
{
    /// Creates a `StringValue` from a value, generating a string representation.
    pub fn from_value(value: T) -> Self {
        Self {
            raw: value.to_string(),
            value: Ok(value),
        }
    }
}

// Implement Debug manually
impl<T: Debug + FromStr> Debug for StringValue<T>
where
    <T as FromStr>::Err: Debug + Clone + PartialEq + Eq,
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
    <T as FromStr>::Err: Debug + Clone + PartialEq + Eq,
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
    <T as FromStr>::Err: Debug + Clone + PartialEq + Eq,
{
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw && self.value == other.value
    }
}

// Implement Eq manually
impl<T: FromStr + PartialEq> Eq for StringValue<T> where
    <T as FromStr>::Err: Debug + Clone + PartialEq + Eq
{
}

impl<T> AsRef<Result<T, <T as FromStr>::Err>> for StringValue<T>
where
    T: FromStr,
    <T as FromStr>::Err: Debug + Clone + PartialEq + Eq,
{
    fn as_ref(&self) -> &Result<T, <T as FromStr>::Err> {
        &self.value
    }
}

impl<T: std::fmt::Display + FromStr> std::fmt::Display for StringValue<T>
where
    <T as FromStr>::Err: Debug + Clone + PartialEq + Eq,
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
    <T as FromStr>::Err: Debug + Clone + PartialEq + Eq,
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
    <T as FromStr>::Err: Debug + Clone + PartialEq + Eq,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Ok(Self::new(raw))
    }
}
