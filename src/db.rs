//! Database module for persistent storage and incremental scanning

use rusqlite::{params, Connection, Result as SqliteResult};
use std::collections::HashMap;
use std::path::Path;

use crate::models::{FileStatus, ScannedFile};

/// File record stored in database (minimal for fast comparison)
#[derive(Debug, Clone)]
pub struct FileRecord {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub mtime: i64,
    pub hash: Option<String>,
    pub status: String,
}

/// Deleted file record
#[derive(Debug, Clone)]
pub struct DeletedFileRecord {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub hash: Option<String>,
    pub deleted_at: i64,
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
        db.migrate_schema()?;
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
                is_partial_hash INTEGER DEFAULT 0,
                status TEXT DEFAULT 'new',
                old_path TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_files_mtime ON files(mtime);
            CREATE INDEX IF NOT EXISTS idx_files_size ON files(size);
            CREATE INDEX IF NOT EXISTS idx_files_hash ON files(hash);
            CREATE INDEX IF NOT EXISTS idx_files_status ON files(status);

            CREATE TABLE IF NOT EXISTS deleted_files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL,
                name TEXT NOT NULL,
                size INTEGER NOT NULL,
                mtime INTEGER NOT NULL,
                ctime INTEGER NOT NULL,
                extension TEXT NOT NULL,
                media_type TEXT NOT NULL,
                hash TEXT,
                deleted_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_deleted_files_hash ON deleted_files(hash);
            CREATE INDEX IF NOT EXISTS idx_deleted_files_deleted_at ON deleted_files(deleted_at);
            ",
        )?;
        Ok(())
    }

    /// Migrate schema for existing databases
    fn migrate_schema(&self) -> SqliteResult<()> {
        // Check if status column exists
        let has_status: bool = self
            .conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM pragma_table_info('files') WHERE name='status'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !has_status {
            self.conn
                .execute("ALTER TABLE files ADD COLUMN status TEXT DEFAULT 'unchanged'", [])?;
            self.conn
                .execute("ALTER TABLE files ADD COLUMN old_path TEXT", [])?;
        }

        Ok(())
    }

    /// Load all file records as HashMap for fast lookup
    pub fn load_file_index(&self) -> SqliteResult<HashMap<String, FileRecord>> {
        let mut stmt = self
            .conn
            .prepare("SELECT path, name, size, mtime, hash, status FROM files")?;

        let rows = stmt.query_map([], |row| {
            Ok(FileRecord {
                path: row.get(0)?,
                name: row.get(1)?,
                size: row.get::<_, i64>(2)? as u64,
                mtime: row.get(3)?,
                hash: row.get(4)?,
                status: row.get::<_, String>(5).unwrap_or_else(|_| "unchanged".to_string()),
            })
        })?;

        let mut index = HashMap::new();
        for row in rows {
            let record = row?;
            index.insert(record.path.clone(), record);
        }
        Ok(index)
    }

    /// Load hash index for move detection
    pub fn load_hash_index(&self) -> SqliteResult<HashMap<String, FileRecord>> {
        let mut stmt = self
            .conn
            .prepare("SELECT path, name, size, mtime, hash, status FROM files WHERE hash IS NOT NULL")?;

        let rows = stmt.query_map([], |row| {
            Ok(FileRecord {
                path: row.get(0)?,
                name: row.get(1)?,
                size: row.get::<_, i64>(2)? as u64,
                mtime: row.get(3)?,
                hash: row.get(4)?,
                status: row.get::<_, String>(5).unwrap_or_else(|_| "unchanged".to_string()),
            })
        })?;

        let mut index = HashMap::new();
        for row in rows {
            let record = row?;
            if let Some(ref hash) = record.hash {
                index.insert(hash.clone(), record);
            }
        }
        Ok(index)
    }

    /// Reset all file statuses to 'unchanged' before incremental scan
    pub fn reset_statuses(&mut self) -> SqliteResult<()> {
        self.conn.execute(
            "UPDATE files SET status = 'unchanged', old_path = NULL WHERE status IN ('new', 'modified', 'moved')",
            [],
        )?;
        Ok(())
    }

    /// Batch insert/update files with status
    pub fn upsert_files(&mut self, files: &[ScannedFile]) -> SqliteResult<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO files 
                 (path, name, size, mtime, ctime, extension, media_type, hash, is_partial_hash, status, old_path)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            )?;

            for file in files {
                // Normalize path separators for cross-platform consistency
                let path_str = file
                    .path
                    .as_ref()
                    .map(|p| p.to_string_lossy().replace('\\', "/"))
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
                    file.status.as_str(),
                    file.old_path,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Move files to deleted_files table and remove from files table
    pub fn move_to_deleted(&mut self, paths: &[String]) -> SqliteResult<()> {
        if paths.is_empty() {
            return Ok(());
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let tx = self.conn.transaction()?;
        {
            // Insert into deleted_files
            let mut insert_stmt = tx.prepare(
                "INSERT INTO deleted_files (path, name, size, mtime, ctime, extension, media_type, hash, deleted_at)
                 SELECT path, name, size, mtime, ctime, extension, media_type, hash, ?1
                 FROM files WHERE path = ?2",
            )?;

            // Delete from files
            let mut delete_stmt = tx.prepare("DELETE FROM files WHERE path = ?1")?;

            for path in paths {
                insert_stmt.execute(params![now, path])?;
                delete_stmt.execute(params![path])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Delete files by paths (without moving to deleted_files)
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

    /// Get files by status
    pub fn get_files_by_status(&self, status: FileStatus) -> SqliteResult<Vec<FileRecord>> {
        let mut stmt = self
            .conn
            .prepare("SELECT path, name, size, mtime, hash, status FROM files WHERE status = ?1")?;

        let rows = stmt.query_map([status.as_str()], |row| {
            Ok(FileRecord {
                path: row.get(0)?,
                name: row.get(1)?,
                size: row.get::<_, i64>(2)? as u64,
                mtime: row.get(3)?,
                hash: row.get(4)?,
                status: row.get(5)?,
            })
        })?;

        let mut files = Vec::new();
        for row in rows {
            files.push(row?);
        }
        Ok(files)
    }

    /// Get recently deleted files
    pub fn get_deleted_files(&self, since_timestamp: Option<i64>) -> SqliteResult<Vec<DeletedFileRecord>> {
        let mut files = Vec::new();

        if let Some(ts) = since_timestamp {
            let mut stmt = self.conn.prepare(
                "SELECT path, name, size, hash, deleted_at FROM deleted_files WHERE deleted_at >= ?1 ORDER BY deleted_at DESC"
            )?;
            let rows = stmt.query_map([ts], |row| {
                Ok(DeletedFileRecord {
                    path: row.get(0)?,
                    name: row.get(1)?,
                    size: row.get::<_, i64>(2)? as u64,
                    hash: row.get(3)?,
                    deleted_at: row.get(4)?,
                })
            })?;
            for row in rows {
                files.push(row?);
            }
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT path, name, size, hash, deleted_at FROM deleted_files ORDER BY deleted_at DESC"
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(DeletedFileRecord {
                    path: row.get(0)?,
                    name: row.get(1)?,
                    size: row.get::<_, i64>(2)? as u64,
                    hash: row.get(3)?,
                    deleted_at: row.get(4)?,
                })
            })?;
            for row in rows {
                files.push(row?);
            }
        }

        Ok(files)
    }

    /// Get file count
    pub fn file_count(&self) -> SqliteResult<u64> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))?;
        Ok(count as u64)
    }

    /// Get status counts
    pub fn get_status_counts(&self) -> SqliteResult<HashMap<String, u64>> {
        let mut stmt = self
            .conn
            .prepare("SELECT status, COUNT(*) FROM files GROUP BY status")?;

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64))
        })?;

        let mut counts = HashMap::new();
        for row in rows {
            let (status, count) = row?;
            counts.insert(status, count);
        }
        Ok(counts)
    }

    /// Clear all records from deleted_files table
    pub fn clear_deleted_files(&mut self) -> SqliteResult<u64> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM deleted_files", [], |row| row.get(0))?;
        self.conn.execute("DELETE FROM deleted_files", [])?;
        Ok(count as u64)
    }

    /// Get deleted files count
    pub fn deleted_files_count(&self) -> SqliteResult<u64> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM deleted_files", [], |row| row.get(0))?;
        Ok(count as u64)
    }
}
