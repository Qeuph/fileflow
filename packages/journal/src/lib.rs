use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use blake3::Hasher;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum JournalError {
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Hashing error: {0}")]
    Hashing(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub session_id: String,
    pub prev_hash: [u8; 32],
    pub operation_type: String,
    pub pre_state_hash: [u8; 32],
    pub post_state_hash: [u8; 32],
    pub transform_metadata: serde_json::Value,
    pub signature: Vec<u8>,
    pub timestamp: DateTime<Utc>,
}

impl Block {
    pub fn new(
        session_id: String,
        prev_hash: [u8; 32],
        operation_type: String,
        pre_state_hash: [u8; 32],
        post_state_hash: [u8; 32],
        transform_metadata: serde_json::Value,
    ) -> Self {
        Self {
            session_id,
            prev_hash,
            operation_type,
            pre_state_hash,
            post_state_hash,
            transform_metadata,
            signature: Vec::new(),
            timestamp: Utc::now(),
        }
    }

    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(self.session_id.as_bytes());
        hasher.update(&self.prev_hash);
        hasher.update(self.operation_type.as_bytes());
        hasher.update(&self.pre_state_hash);
        hasher.update(&self.post_state_hash);
        
        let metadata_bytes = serde_json::to_vec(&self.transform_metadata).unwrap_or_default();
        hasher.update(&metadata_bytes);
        
        *hasher.finalize().as_bytes()
    }
}

pub struct MerkleTree {
    pub root: [u8; 32],
}

impl MerkleTree {
    pub fn from_hashes(hashes: Vec<[u8; 32]>) -> Self {
        if hashes.is_empty() {
            return Self { root: [0u8; 32] };
        }
        let mut sorted_hashes = hashes;
        sorted_hashes.sort();
        let root = Self::compute_root(sorted_hashes);
        Self { root }
    }

    fn compute_root(hashes: Vec<[u8; 32]>) -> [u8; 32] {
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
                hasher.update(&chunk[0]); // Pad with self if odd
            }
            next_level.push(*hasher.finalize().as_bytes());
        }

        Self::compute_root(next_level)
    }
}

pub trait JournalStore {
    fn append_block(&mut self, block: Block) -> Result<[u8; 32], JournalError>;
    fn get_block(&self, hash: [u8; 32]) -> Result<Option<Block>, JournalError>;
    fn get_latest_hash(&self) -> Result<[u8; 32], JournalError>;
    fn get_block_by_step(&self, session_id: &str, step: usize) -> Result<Option<Block>, JournalError>;
}

pub struct SqliteStore {
    connection: sqlite::Connection,
}

impl SqliteStore {
    pub fn new(path: &str) -> Result<Self, JournalError> {
        let connection = sqlite::open(path)
            .map_err(|e| JournalError::Serialization(e.to_string()))?;
        
        connection.execute(
            "CREATE TABLE IF NOT EXISTS journal (
                hash BLOB PRIMARY KEY,
                session_id TEXT NOT NULL,
                prev_hash BLOB,
                data BLOB NOT NULL,
                timestamp TEXT NOT NULL
            )"
        ).map_err(|e| JournalError::Serialization(e.to_string()))?;

        Ok(Self { connection })
    }
}

impl JournalStore for SqliteStore {
    fn append_block(&mut self, block: Block) -> Result<[u8; 32], JournalError> {
        let hash = block.hash();
        let data = serde_json::to_vec(&block)
            .map_err(|e| JournalError::Serialization(e.to_string()))?;
        
        let mut statement = self.connection.prepare(
            "INSERT INTO journal (hash, session_id, prev_hash, data, timestamp) VALUES (?, ?, ?, ?, ?)"
        ).map_err(|e| JournalError::Serialization(e.to_string()))?;

        statement.bind((1, &hash[..])).unwrap();
        statement.bind((2, block.session_id.as_str())).unwrap();
        statement.bind((3, &block.prev_hash[..])).unwrap();
        statement.bind((4, &data[..])).unwrap();
        statement.bind((5, block.timestamp.to_rfc3339().as_str())).unwrap();

        statement.next().map_err(|e| JournalError::Serialization(e.to_string()))?;

        Ok(hash)
    }

    fn get_block(&self, hash: [u8; 32]) -> Result<Option<Block>, JournalError> {
        let mut statement = self.connection.prepare(
            "SELECT data FROM journal WHERE hash = ?"
        ).map_err(|e| JournalError::Serialization(e.to_string()))?;

        statement.bind((1, &hash[..])).unwrap();

        if let Ok(sqlite::State::Row) = statement.next() {
            let data: Vec<u8> = statement.read(0).unwrap();
            let block: Block = serde_json::from_slice(&data)
                .map_err(|e| JournalError::Serialization(e.to_string()))?;
            Ok(Some(block))
        } else {
            Ok(None)
        }
    }

    fn get_latest_hash(&self) -> Result<[u8; 32], JournalError> {
        let mut statement = self.connection.prepare(
            "SELECT hash FROM journal ORDER BY timestamp DESC LIMIT 1"
        ).map_err(|e| JournalError::Serialization(e.to_string()))?;

        if let Ok(sqlite::State::Row) = statement.next() {
            let hash: Vec<u8> = statement.read(0).unwrap();
            let mut res = [0u8; 32];
            res.copy_from_slice(&hash);
            Ok(res)
        } else {
            Ok([0u8; 32])
        }
    }

    fn get_block_by_step(&self, session_id: &str, step: usize) -> Result<Option<Block>, JournalError> {
        let mut statement = self.connection.prepare(
            "SELECT data FROM journal WHERE session_id = ? ORDER BY timestamp ASC LIMIT 1 OFFSET ?"
        ).map_err(|e| JournalError::Serialization(e.to_string()))?;

        statement.bind((1, session_id)).unwrap();
        statement.bind((2, step as i64)).unwrap();

        if let Ok(sqlite::State::Row) = statement.next() {
            let data: Vec<u8> = statement.read(0).unwrap();
            let block: Block = serde_json::from_slice(&data)
                .map_err(|e| JournalError::Serialization(e.to_string()))?;
            Ok(Some(block))
        } else {
            Ok(None)
        }
    }
}
