use std::path::{Path, PathBuf};
use std::collections::HashMap;
use anyhow::Result;
use blake3::Hasher;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub size: u64,
    pub hash: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub root: PathBuf,
    pub files: HashMap<PathBuf, FileMetadata>,
}

impl Snapshot {
    pub fn create(root: &Path) -> Result<Self> {
        let mut files = HashMap::new();
        for entry in walkdir::WalkDir::new(root) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let path = entry.path().to_path_buf();
                let relative_path = path.strip_prefix(root)?.to_path_buf();
                let metadata = std::fs::metadata(&path)?;
                let mut hasher = Hasher::new();
                let content = std::fs::read(&path)?;
                hasher.update(&content);
                let hash = *hasher.finalize().as_bytes();

                files.insert(relative_path.clone(), FileMetadata {
                    path: relative_path,
                    size: metadata.len(),
                    hash,
                });
            }
        }
        Ok(Self { root: root.to_path_buf(), files })
    }

    pub fn compute_merkle_root(&self) -> [u8; 32] {
        let mut hashes: Vec<[u8; 32]> = self.files.values()
            .map(|m| m.hash)
            .collect();
        hashes.sort();
        
        if hashes.is_empty() {
            return [0u8; 32];
        }

        Self::compute_root(hashes)
    }

    pub fn compute_root(hashes: Vec<[u8; 32]>) -> [u8; 32] {
        if hashes.len() == 1 {
            return hashes[0];
        }
        let mut next_level = Vec::new();
        for chunk in hashes.chunks(2) {
            let mut hasher = Hasher::new();
            hasher.update(&chunk[0]);
            if chunk.len() == 2 {
                hasher.update(&chunk[1]);
            } else {
                hasher.update(&chunk[0]);
            }
            next_level.push(*hasher.finalize().as_bytes());
        }
        Self::compute_root(next_level)
    }
}

pub struct Sandbox {
    pub base_snapshot: Snapshot,
    pub work_dir: PathBuf,
    pub changes: HashMap<PathBuf, Vec<u8>>, // Simple CoW: relative path -> new content
}

impl Sandbox {
    pub fn new(snapshot: Snapshot, work_dir: PathBuf) -> Self {
        Self {
            base_snapshot: snapshot,
            work_dir,
            changes: HashMap::new(),
        }
    }

    pub fn write_file(&mut self, relative_path: PathBuf, content: Vec<u8>) {
        self.changes.insert(relative_path, content);
    }

    pub fn compute_post_merkle_root(&self) -> [u8; 32] {
        let mut hashes: Vec<[u8; 32]> = Vec::new();
        
        // Use base hashes for unchanged files
        for (rel_path, meta) in &self.base_snapshot.files {
            if !self.changes.contains_key(rel_path) {
                hashes.push(meta.hash);
            }
        }

        // Use new hashes for changed files
        for content in self.changes.values() {
            let mut hasher = Hasher::new();
            hasher.update(content);
            hashes.push(*hasher.finalize().as_bytes());
        }

        hashes.sort();
        
        if hashes.is_empty() {
            return [0u8; 32];
        }

        Snapshot::compute_root(hashes)
    }

    pub fn materialize(&self, target_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(target_dir)?;
        
        // Copy base files
        for (rel_path, _meta) in &self.base_snapshot.files {
            if !self.changes.contains_key(rel_path) {
                let src = self.base_snapshot.root.join(rel_path);
                let dst = target_dir.join(rel_path);
                if let Some(parent) = dst.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(src, dst)?;
            }
        }

        // Apply changes
        for (rel_path, content) in &self.changes {
            let dst = target_dir.join(rel_path);
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(dst, content)?;
        }

        Ok(())
    }
}
