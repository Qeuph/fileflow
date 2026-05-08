use std::path::{Path, PathBuf};
use sqlite::Connection;
use walkdir::WalkDir;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub path: String,
    pub size: i64,
    pub modified: DateTime<Utc>,
    pub mime: Option<String>,
}

pub struct Indexer {
    connection: Connection,
}

impl Indexer {
    pub fn new(db_path: &str) -> anyhow::Result<Self> {
        let connection = sqlite::open(db_path)?;
        connection.execute(
            "CREATE TABLE IF NOT EXISTS files (
                path TEXT PRIMARY KEY,
                size INTEGER NOT NULL,
                modified TEXT NOT NULL,
                mime TEXT
            )"
        )?;
        Ok(Self { connection })
    }

    pub fn index_directory(&self, root: &Path) -> anyhow::Result<()> {
        for entry in WalkDir::new(root) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let metadata = entry.metadata()?;
                let modified: DateTime<Utc> = metadata.modified()?.into();
                let path = entry.path().to_string_lossy().to_string();
                
                let mut statement = self.connection.prepare(
                    "INSERT OR REPLACE INTO files (path, size, modified, mime) VALUES (?, ?, ?, ?)"
                )?;
                statement.bind((1, path.as_str()))?;
                statement.bind((2, metadata.len() as i64))?;
                statement.bind((3, modified.to_rfc3339().as_str()))?;
                statement.bind((4, None::<&str>))?; // TODO: Add mime detection
                statement.next()?;
            }
        }
        Ok(())
    }
}

pub struct QueryEngine {
    connection: Connection,
}

impl QueryEngine {
    pub fn new(db_path: &str) -> anyhow::Result<Self> {
        let connection = sqlite::open(db_path)?;
        Ok(Self { connection })
    }

    pub fn query(&self, sql_filter: &str) -> anyhow::Result<Vec<FileRecord>> {
        let mut query = "SELECT path, size, modified, mime FROM files".to_string();
        if !sql_filter.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(sql_filter);
        }

        let mut records = Vec::new();
        let mut statement = self.connection.prepare(query)?;
        
        while let Ok(sqlite::State::Row) = statement.next() {
            let path: String = statement.read(0)?;
            let size: i64 = statement.read(1)?;
            let modified_str: String = statement.read(2)?;
            let mime: Option<String> = statement.read(3)?;
            
            let modified = DateTime::parse_from_rfc3339(&modified_str)?.with_timezone(&Utc);
            
            records.push(FileRecord {
                path,
                size,
                modified,
                mime,
            });
        }
        
        Ok(records)
    }
}
