//! Immutable binary payloads (pictures) shared between document copies.
//!
//! Documents are cloned on every frame, for undo history, recovery snapshots and render
//! primitives. With `Blob` those clones copy a pointer instead of megabytes of image data, and
//! the content fingerprint used for texture caching is computed once per payload.

use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Default)]
struct Inner {
    bytes: Vec<u8>,
    fingerprint: OnceLock<u64>,
}

#[derive(Clone, Default)]
pub struct Blob(Arc<Inner>);

impl Blob {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(Arc::new(Inner {
            bytes,
            fingerprint: OnceLock::new(),
        }))
    }
    pub fn as_slice(&self) -> &[u8] {
        &self.0.bytes
    }
    /// Content hash, computed on first use and shared by every clone.
    pub fn fingerprint(&self) -> u64 {
        *self.0.fingerprint.get_or_init(|| {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            self.0.bytes.hash(&mut hasher);
            hasher.finish()
        })
    }
    /// Takes the bytes out, copying only when another document still shares them.
    pub fn into_vec(self) -> Vec<u8> {
        match Arc::try_unwrap(self.0) {
            Ok(inner) => inner.bytes,
            Err(shared) => shared.bytes.clone(),
        }
    }
}

impl Deref for Blob {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.0.bytes
    }
}

impl AsRef<[u8]> for Blob {
    fn as_ref(&self) -> &[u8] {
        &self.0.bytes
    }
}

impl From<Vec<u8>> for Blob {
    fn from(bytes: Vec<u8>) -> Self {
        Self::new(bytes)
    }
}

impl From<&[u8]> for Blob {
    fn from(bytes: &[u8]) -> Self {
        Self::new(bytes.to_vec())
    }
}

impl PartialEq for Blob {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0.bytes == other.0.bytes
    }
}

impl Eq for Blob {}

impl PartialEq<Vec<u8>> for Blob {
    fn eq(&self, other: &Vec<u8>) -> bool {
        self.0.bytes == *other
    }
}

impl PartialEq<Blob> for Vec<u8> {
    fn eq(&self, other: &Blob) -> bool {
        *self == other.0.bytes
    }
}

impl fmt::Debug for Blob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Blob({} bytes)", self.0.bytes.len())
    }
}

/// Same wire format as the `Vec<u8>` it replaces, so saved files and recovery data stay readable.
impl Serialize for Blob {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.bytes.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Blob {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Vec::<u8>::deserialize(deserializer).map(Self::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clones_share_bytes_and_keep_the_vec_wire_format() -> Result<(), String> {
        let blob = Blob::from(vec![1, 2, 3]);
        let copy = blob.clone();
        assert!(std::ptr::eq(blob.as_slice(), copy.as_slice()));
        assert_eq!(blob.fingerprint(), copy.fingerprint());
        assert_eq!(blob, Blob::from(vec![1, 2, 3]));
        assert_ne!(blob, Blob::from(vec![1, 2]));
        assert_eq!(blob.len(), 3);
        let json = serde_json::to_string(&blob).map_err(|e| e.to_string())?;
        assert_eq!(
            json,
            serde_json::to_string(&vec![1u8, 2, 3]).map_err(|e| e.to_string())?
        );
        let back: Blob = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        assert_eq!(back, blob);
        drop(copy);
        assert_eq!(blob.into_vec(), vec![1, 2, 3]);
        Ok(())
    }
}
