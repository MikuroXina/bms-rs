/// Trait extension utility for [`str`].
pub trait StrExtension {
    /// Returns `true` if `needle` is a prefix of the string regardless of its case.
    fn starts_with_ignore_case(&self, needle: &str) -> bool;

    /// Returns a string slice with the prefix removed regardless of its case.
    fn strip_prefix_ignore_case(&self, prefix: &str) -> Option<&Self>;
}

impl StrExtension for str {
    fn starts_with_ignore_case(&self, needle: &str) -> bool {
        let n = needle.len();
        self.len() >= n && self.is_char_boundary(n) && needle.eq_ignore_ascii_case(&self[..n])
    }

    fn strip_prefix_ignore_case(&self, prefix: &str) -> Option<&Self> {
        self.starts_with_ignore_case(prefix)
            .then(|| &self[prefix.len()..])
            .filter(|s| !s.is_empty())
    }
}
