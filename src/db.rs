//! Database module for persistent storage and incremental scanning

use rusqlite::{params, Connection, Result as SqliteResult};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::models::ScannedFile;

/// File record stored in database (minimal for fast comparison)
#[derive(Debug, Clone)]
pub struct FileRecord {
    pub path: String,
    pub size: u64,
    pub mtime: i64,
    pub hash: Option<String>,
}

/// Database manager for scan results
pub struct ScanDatabase {
    conn: Connection,
}

impl ScanDatabase {
    /// Open or create database
    pub fn open(path: &Path) -> SqliteResult<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Open in-memory database (for testing)
    pub fn open_memory() -> SqliteResult<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Initialize database schema
    fn init_schema(&self) -> SqliteResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS files (
                path TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                size INTEGER NOT NULL,
                mtime INTEGER NOT NULL,
                ctime INTEGER NOT NULL,
                extension TEXT NOT NULL,
                media_type TEXT NOT NULL,
                hash TEXT,
                is_partial_hash INTEGER DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_files_mtime ON files(mtime);
            CREATE INDEX IF NOT EXISTS idx_files_size ON files(size);
            CREATE INDEX IF NOT EXISTS idx_files_hash ON files(hash);
            ",
        )?;
        Ok(())
    }

    /// Load all file records as HashMap for fast lookup
    /// Key: file path, Value: (size, mtime, hash)
    pub fn load_file_index(&self) -> SqliteResult<HashMap<String, FileRecord>> {
        let mut stmt = self
            .conn
            .prepare("SELECT path, size, mtime, hash FROM files")?;

        let rows = stmt.query_map([], |row| {
            Ok(FileRecord {
                path: row.get(0)?,
                size: row.get::<_, i64>(1)? as u64,
                mtime: row.get(2)?,
                hash: row.get(3)?,
            })
        })?;

        let mut index = HashMap::new();
        for row in rows {
            let record = row?;
            index.insert(record.path.clone(), record);
        }
        Ok(index)
    }

    /// Batch insert/update files
    pub fn upsert_files(&mut self, files: &[ScannedFile]) -> SqliteResult<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO files 
                 (path, name, size, mtime, ctime, extension, media_type, hash, is_partial_hash)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            )?;

            for file in files {
                let path_str = file
                    .path
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();

                stmt.execute(params![
                    path_str,
                    file.name,
                    file.size as i64,
                    file.mtime,
                    file.ctime,
                    file.extension,
                    file.media_type.as_str(),
                    file.hash,
                    file.is_partial_hash as i32,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Delete files by paths
    pub fn delete_files(&mut self, paths: &[String]) -> SqliteResult<()> {
        if paths.is_empty() {
            return Ok(());
        }

        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare("DELETE FROM files WHERE path = ?1")?;
            for path in paths {
                stmt.execute(params![path])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Get file count
    pub fn file_count(&self) -> SqliteResult<u64> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))?;
        Ok(count as u64)
    }
}
