//! Resource mapping module
//!
//! Provides abstractions for managing chart resources (audio and image files).
//! Supports different resource storage strategies through the `ResourceMapping` trait.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::types::{BmpId, WavId};

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

    /// Convert to a `HashMap` format for WAV files.
    ///
    /// This is useful for backward compatibility and for contexts where
    /// a `HashMap` is more convenient.
    fn to_wav_map(&self) -> HashMap<WavId, &Path>;

    /// Convert to a `HashMap` format for BMP files.
    ///
    /// This is useful for backward compatibility and for contexts where
    /// a `HashMap` is more convenient.
    fn to_bmp_map(&self) -> HashMap<BmpId, &Path>;
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

    fn to_wav_map(&self) -> HashMap<WavId, &Path> {
        self.wav_paths
            .iter()
            .map(|(id, path)| (*id, path.as_path()))
            .collect()
    }

    fn to_bmp_map(&self) -> HashMap<BmpId, &Path> {
        self.bmp_paths
            .iter()
            .map(|(id, path)| (*id, path.as_path()))
            .collect()
    }
}

/// Name-based resource mapping implementation.
///
/// This implementation stores mappings from resource names to IDs.
/// It's commonly used for BMSON format charts where resources are identified by filenames.
///
/// Note: This implementation returns virtual paths (the name itself as a Path).
/// In actual usage, these paths should be resolved based on the chart file location.
#[derive(Debug, Clone)]
pub struct NameBasedResourceMapping {
    /// Audio filename to ID mapping
    audio_name_to_id: HashMap<String, WavId>,

    /// Image filename to ID mapping
    bmp_name_to_id: HashMap<String, BmpId>,
}

impl NameBasedResourceMapping {
    /// Create a new `NameBasedResourceMapping`.
    #[must_use]
    pub const fn new(
        audio_name_to_id: HashMap<String, WavId>,
        bmp_name_to_id: HashMap<String, BmpId>,
    ) -> Self {
        Self {
            audio_name_to_id,
            bmp_name_to_id,
        }
    }

    /// Create a new empty `NameBasedResourceMapping`.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            audio_name_to_id: HashMap::new(),
            bmp_name_to_id: HashMap::new(),
        }
    }

    /// Insert an audio file mapping.
    pub fn insert_audio(&mut self, name: String, id: WavId) {
        self.audio_name_to_id.insert(name, id);
    }

    /// Insert a BMP file mapping.
    pub fn insert_bmp(&mut self, name: String, id: BmpId) {
        self.bmp_name_to_id.insert(name, id);
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
        // Find the name corresponding to this ID and return it as a virtual path
        for (name, wav_id) in &self.audio_name_to_id {
            if *wav_id == id {
                return Some(Path::new(name));
            }
        }
        None
    }

    fn get_bmp_path(&self, id: BmpId) -> Option<&Path> {
        // Find the name corresponding to this ID and return it as a virtual path
        for (name, bmp_id) in &self.bmp_name_to_id {
            if *bmp_id == id {
                return Some(Path::new(name));
            }
        }
        None
    }

    fn to_wav_map(&self) -> HashMap<WavId, &Path> {
        self.audio_name_to_id
            .iter()
            .map(|(name, id)| (*id, Path::new(name)))
            .collect()
    }

    fn to_bmp_map(&self) -> HashMap<BmpId, &Path> {
        self.bmp_name_to_id
            .iter()
            .map(|(name, id)| (*id, Path::new(name)))
            .collect()
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

        // Test to_wav_map
        let wav_hashmap = mapping.to_wav_map();
        assert_eq!(wav_hashmap.len(), 2);
        assert_eq!(
            wav_hashmap.get(&WavId::new(1)),
            Some(&PathBuf::from("audio1.wav").as_path())
        );

        // Test to_bmp_map
        let bmp_hashmap = mapping.to_bmp_map();
        assert_eq!(bmp_hashmap.len(), 2);
        assert_eq!(
            bmp_hashmap.get(&BmpId::new(1)),
            Some(&PathBuf::from("image1.bmp").as_path())
        );
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

        // Test to_wav_map
        let wav_hashmap = mapping.to_wav_map();
        assert_eq!(wav_hashmap.len(), 2);
        assert_eq!(
            wav_hashmap.get(&WavId::new(0)),
            Some(&Path::new("song1.wav"))
        );

        // Test to_bmp_map
        let bmp_hashmap = mapping.to_bmp_map();
        assert_eq!(bmp_hashmap.len(), 2);
        assert_eq!(bmp_hashmap.get(&BmpId::new(0)), Some(&Path::new("bg1.png")));
    }
}
