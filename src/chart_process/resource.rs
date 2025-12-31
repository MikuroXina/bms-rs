//! Resource mapping module
//!
//! Provides abstractions for managing chart resources (audio and image files).
//! Supports different resource storage strategies through the `ResourceMapping` trait.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// WAV audio file ID wrapper type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WavId(pub usize);

impl AsRef<usize> for WavId {
    fn as_ref(&self) -> &usize {
        &self.0
    }
}

impl WavId {
    /// Create a new `WavId`
    #[must_use]
    pub const fn new(id: usize) -> Self {
        Self(id)
    }

    /// Returns the contained id value.
    #[must_use]
    pub const fn value(self) -> usize {
        self.0
    }
}

impl From<usize> for WavId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<WavId> for usize {
    fn from(id: WavId) -> Self {
        id.0
    }
}

/// BMP/BGA image file ID wrapper type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BmpId(pub usize);

impl AsRef<usize> for BmpId {
    fn as_ref(&self) -> &usize {
        &self.0
    }
}

impl BmpId {
    /// Create a new `BmpId`
    #[must_use]
    pub const fn new(id: usize) -> Self {
        Self(id)
    }

    /// Returns the contained id value.
    #[must_use]
    pub const fn value(self) -> usize {
        self.0
    }
}

impl From<usize> for BmpId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<BmpId> for usize {
    fn from(id: BmpId) -> Self {
        id.0
    }
}

/// Trait for resource mapping, providing a unified interface for accessing chart resources.
///
/// This trait abstracts different resource storage strategies, allowing flexible
/// resource loading from various sources (filesystem, memory, network, etc.).
pub trait ResourceMapping {
    /// Get the audio file path for a given WAV ID.
    ///
    /// Returns `None` if the ID is not found in the mapping.
    fn get_wav_path(&self, id: WavId) -> Option<&Path>;

    /// Get the image file path for a given BMP ID.
    ///
    /// Returns `None` if the ID is not found in the mapping.
    fn get_bmp_path(&self, id: BmpId) -> Option<&Path>;

    /// Iterate over all audio file mappings.
    ///
    /// This is more efficient than collecting into a `HashMap`, as it avoids
    /// intermediate allocations. Use this for processing all audio files.
    fn for_each_wav_path<F>(&self, f: F)
    where
        F: FnMut(WavId, &Path);

    /// Iterate over all BMP file mappings.
    ///
    /// This is more efficient than collecting into a `HashMap`, as it avoids
    /// intermediate allocations. Use this for processing all image files.
    fn for_each_bmp_path<F>(&self, f: F)
    where
        F: FnMut(BmpId, &Path);
}

/// HashMap-based resource mapping implementation.
///
/// This implementation stores direct mappings from resource IDs to file paths.
/// It's commonly used for BMS format charts where resources are identified by numeric IDs.
#[derive(Debug, Clone)]
pub struct HashMapResourceMapping {
    /// WAV file ID to path mapping
    wav_paths: HashMap<WavId, PathBuf>,

    /// BMP file ID to path mapping
    bmp_paths: HashMap<BmpId, PathBuf>,
}

impl HashMapResourceMapping {
    /// Create a new `HashMapResourceMapping`.
    #[must_use]
    pub const fn new(
        wav_paths: HashMap<WavId, PathBuf>,
        bmp_paths: HashMap<BmpId, PathBuf>,
    ) -> Self {
        Self {
            wav_paths,
            bmp_paths,
        }
    }

    /// Create a new empty `HashMapResourceMapping`.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            wav_paths: HashMap::new(),
            bmp_paths: HashMap::new(),
        }
    }

    /// Insert a WAV file mapping.
    pub fn insert_wav(&mut self, id: WavId, path: PathBuf) {
        self.wav_paths.insert(id, path);
    }

    /// Insert a BMP file mapping.
    pub fn insert_bmp(&mut self, id: BmpId, path: PathBuf) {
        self.bmp_paths.insert(id, path);
    }
}

impl ResourceMapping for HashMapResourceMapping {
    fn get_wav_path(&self, id: WavId) -> Option<&Path> {
        self.wav_paths.get(&id).map(std::path::PathBuf::as_path)
    }

    fn get_bmp_path(&self, id: BmpId) -> Option<&Path> {
        self.bmp_paths.get(&id).map(std::path::PathBuf::as_path)
    }

    fn for_each_wav_path<F>(&self, mut f: F)
    where
        F: FnMut(WavId, &Path),
    {
        for (id, path) in &self.wav_paths {
            f(*id, path);
        }
    }

    fn for_each_bmp_path<F>(&self, mut f: F)
    where
        F: FnMut(BmpId, &Path),
    {
        for (id, path) in &self.bmp_paths {
            f(*id, path);
        }
    }
}

/// Name-based resource mapping implementation.
///
/// This implementation stores bidirectional mappings between resource names and IDs.
/// It's commonly used for BMSON format charts where resources are identified by filenames.
///
/// Note: This implementation returns virtual paths (the name itself as a Path).
/// In actual usage, these paths should be resolved based on the chart file location.
#[derive(Debug, Clone)]
pub struct NameBasedResourceMapping {
    /// Audio filename to ID mapping
    audio_name_to_id: HashMap<String, WavId>,

    /// Audio ID to filename mapping (for reverse lookup)
    audio_id_to_name: HashMap<WavId, String>,

    /// Image filename to ID mapping
    bmp_name_to_id: HashMap<String, BmpId>,

    /// Image ID to filename mapping (for reverse lookup)
    bmp_id_to_name: HashMap<BmpId, String>,
}

impl NameBasedResourceMapping {
    /// Create a new `NameBasedResourceMapping`.
    #[must_use]
    pub fn new(
        audio_name_to_id: HashMap<String, WavId>,
        bmp_name_to_id: HashMap<String, BmpId>,
    ) -> Self {
        // Build reverse mappings for O(1) lookup by ID
        let audio_id_to_name: HashMap<WavId, String> = audio_name_to_id
            .iter()
            .map(|(name, id)| (*id, name.clone()))
            .collect();

        let bmp_id_to_name: HashMap<BmpId, String> = bmp_name_to_id
            .iter()
            .map(|(name, id)| (*id, name.clone()))
            .collect();

        Self {
            audio_name_to_id,
            audio_id_to_name,
            bmp_name_to_id,
            bmp_id_to_name,
        }
    }

    /// Create a new empty `NameBasedResourceMapping`.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            audio_name_to_id: HashMap::new(),
            audio_id_to_name: HashMap::new(),
            bmp_name_to_id: HashMap::new(),
            bmp_id_to_name: HashMap::new(),
        }
    }

    /// Insert an audio file mapping.
    pub fn insert_audio(&mut self, name: String, id: WavId) {
        self.audio_name_to_id.insert(name.clone(), id);
        self.audio_id_to_name.insert(id, name);
    }

    /// Insert a BMP file mapping.
    pub fn insert_bmp(&mut self, name: String, id: BmpId) {
        self.bmp_name_to_id.insert(name.clone(), id);
        self.bmp_id_to_name.insert(id, name);
    }

    /// Get WAV ID by filename.
    #[must_use]
    pub fn get_wav_id(&self, name: &str) -> Option<WavId> {
        self.audio_name_to_id.get(name).copied()
    }

    /// Get BMP ID by filename.
    #[must_use]
    pub fn get_bmp_id(&self, name: &str) -> Option<BmpId> {
        self.bmp_name_to_id.get(name).copied()
    }
}

impl ResourceMapping for NameBasedResourceMapping {
    fn get_wav_path(&self, id: WavId) -> Option<&Path> {
        // O(1) reverse lookup using the ID-to-name mapping
        self.audio_id_to_name.get(&id).map(Path::new)
    }

    fn get_bmp_path(&self, id: BmpId) -> Option<&Path> {
        // O(1) reverse lookup using the ID-to-name mapping
        self.bmp_id_to_name.get(&id).map(Path::new)
    }

    fn for_each_wav_path<F>(&self, mut f: F)
    where
        F: FnMut(WavId, &Path),
    {
        for (name, id) in &self.audio_name_to_id {
            f(*id, Path::new(name));
        }
    }

    fn for_each_bmp_path<F>(&self, mut f: F)
    where
        F: FnMut(BmpId, &Path),
    {
        for (name, id) in &self.bmp_name_to_id {
            f(*id, Path::new(name));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_map_resource_mapping() {
        let mut wav_map = HashMap::new();
        wav_map.insert(WavId::new(1), PathBuf::from("audio1.wav"));
        wav_map.insert(WavId::new(2), PathBuf::from("audio2.wav"));

        let mut bmp_map = HashMap::new();
        bmp_map.insert(BmpId::new(1), PathBuf::from("image1.bmp"));
        bmp_map.insert(BmpId::new(2), PathBuf::from("image2.bmp"));

        let mapping = HashMapResourceMapping::new(wav_map, bmp_map);

        // Test get_wav_path
        assert_eq!(
            mapping.get_wav_path(WavId::new(1)),
            Some(Path::new("audio1.wav"))
        );
        assert_eq!(mapping.get_wav_path(WavId::new(999)), None);

        // Test get_bmp_path
        assert_eq!(
            mapping.get_bmp_path(BmpId::new(1)),
            Some(Path::new("image1.bmp"))
        );
        assert_eq!(mapping.get_bmp_path(BmpId::new(999)), None);

        // Test for_each_wav_path
        let mut wav_count = 0;
        mapping.for_each_wav_path(|id, path| {
            wav_count += 1;
            if id == WavId::new(1) {
                assert_eq!(path, Path::new("audio1.wav"));
            }
        });
        assert_eq!(wav_count, 2);

        // Test for_each_bmp_path
        let mut bmp_count = 0;
        mapping.for_each_bmp_path(|id, path| {
            bmp_count += 1;
            if id == BmpId::new(1) {
                assert_eq!(path, Path::new("image1.bmp"));
            }
        });
        assert_eq!(bmp_count, 2);
    }

    #[test]
    fn test_name_based_resource_mapping() {
        let mut audio_map = HashMap::new();
        audio_map.insert("song1.wav".to_string(), WavId::new(0));
        audio_map.insert("song2.wav".to_string(), WavId::new(1));

        let mut bmp_map = HashMap::new();
        bmp_map.insert("bg1.png".to_string(), BmpId::new(0));
        bmp_map.insert("bg2.png".to_string(), BmpId::new(1));

        let mapping = NameBasedResourceMapping::new(audio_map, bmp_map);

        // Test get_wav_path
        assert_eq!(
            mapping.get_wav_path(WavId::new(0)),
            Some(Path::new("song1.wav"))
        );
        assert_eq!(mapping.get_wav_path(WavId::new(999)), None);

        // Test get_bmp_path
        assert_eq!(
            mapping.get_bmp_path(BmpId::new(0)),
            Some(Path::new("bg1.png"))
        );
        assert_eq!(mapping.get_bmp_path(BmpId::new(999)), None);

        // Test get_wav_id
        assert_eq!(mapping.get_wav_id("song1.wav"), Some(WavId::new(0)));
        assert_eq!(mapping.get_wav_id("nonexistent.wav"), None);

        // Test get_bmp_id
        assert_eq!(mapping.get_bmp_id("bg1.png"), Some(BmpId::new(0)));
        assert_eq!(mapping.get_bmp_id("nonexistent.png"), None);

        // Test for_each_wav_path
        let mut wav_count = 0;
        mapping.for_each_wav_path(|id, path| {
            wav_count += 1;
            if id == WavId::new(0) {
                assert_eq!(path, Path::new("song1.wav"));
            }
        });
        assert_eq!(wav_count, 2);

        // Test for_each_bmp_path
        let mut bmp_count = 0;
        mapping.for_each_bmp_path(|id, path| {
            bmp_count += 1;
            if id == BmpId::new(0) {
                assert_eq!(path, Path::new("bg1.png"));
            }
        });
        assert_eq!(bmp_count, 2);
    }
}
