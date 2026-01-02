// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! SQLite-backed persistent database for CherrySight 2.0

use crate::index::symbol::{Symbol, SymbolId, SymbolKind, Visibility, SymbolFlags};
use crate::util::{FileId, InternedString, Span, Interner};
use anyhow::{Context, Result};
use dashmap::DashMap;
use parking_lot::RwLock;
use rusqlite::{Connection, OptionalExtension, params};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use xxhash_rust::xxh3::xxh3_128;

/// SQLite database manager with connection pooling and caching
pub struct DatabaseManager {
    /// Main database connection
    conn: Arc<RwLock<Connection>>,

    /// Hot cache: Recently accessed symbols (DashMap for lock-free reads)
    symbol_cache: Arc<DashMap<SymbolId, Arc<Symbol>>>,

    /// File ID to URI mapping cache
    file_uri_cache: Arc<DashMap<FileId, String>>,
    uri_file_cache: Arc<DashMap<String, FileId>>,

    /// String interner
    interner: Arc<RwLock<Interner>>,

    /// Database file path
    db_path: PathBuf,

    /// Next symbol ID counter (for new symbols)
    next_symbol_id: Arc<RwLock<i64>>,

    /// Next file ID counter
    next_file_id: Arc<RwLock<i64>>,
}

impl DatabaseManager {
    /// Create a new database manager
    pub fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        let db_path = db_path.as_ref().to_path_buf();

        // Create parent directory if it doesn't exist
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)
            .context("Failed to open database")?;

        // Configure SQLite for performance
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA cache_size = -64000;  -- 64MB cache
            PRAGMA temp_store = MEMORY;
            PRAGMA mmap_size = 268435456;  -- 256MB mmap
            PRAGMA page_size = 8192;
            PRAGMA foreign_keys = ON;
            "#,
        )?;

        // Initialize schema
        Self::initialize_schema(&conn)?;

        // Get the next available IDs
        let next_symbol_id = Self::get_max_id(&conn, "symbols")? + 1;
        let next_file_id = Self::get_max_id(&conn, "files")? + 1;

        Ok(Self {
            conn: Arc::new(RwLock::new(conn)),
            symbol_cache: Arc::new(DashMap::new()),
            file_uri_cache: Arc::new(DashMap::new()),
            uri_file_cache: Arc::new(DashMap::new()),
            interner: Arc::new(RwLock::new(Interner::new())),
            db_path,
            next_symbol_id: Arc::new(RwLock::new(next_symbol_id)),
            next_file_id: Arc::new(RwLock::new(next_file_id)),
        })
    }

    /// Get the string interner
    pub fn interner(&self) -> Arc<RwLock<Interner>> {
        self.interner.clone()
    }

    /// Initialize database schema
    fn initialize_schema(conn: &Connection) -> Result<()> {
        let schema = include_str!("schema.sql");
        conn.execute_batch(schema)
            .context("Failed to initialize database schema")?;
        Ok(())
    }

    /// Get the maximum ID from a table
    fn get_max_id(conn: &Connection, table: &str) -> Result<i64> {
        let query = format!("SELECT COALESCE(MAX(id), 0) FROM {}", table);
        let max_id: i64 = conn.query_row(&query, [], |row| row.get(0))?;
        Ok(max_id)
    }

    /// Generate a stable symbol ID based on file_id, qualified_name, and kind
    fn generate_stable_id(file_id: FileId, qualified_name: &str, kind: SymbolKind) -> [u8; 16] {
        let mut hasher_input = Vec::new();
        hasher_input.extend_from_slice(&file_id.as_u32().to_le_bytes());
        hasher_input.extend_from_slice(qualified_name.as_bytes());
        hasher_input.push(kind as u8);

        let hash = xxh3_128(&hasher_input);
        hash.to_le_bytes()
    }

    /// Get or create a file ID for a URI
    pub fn get_or_create_file_id(&self, uri: &str, path: &Path) -> Result<FileId> {
        // Check cache first
        if let Some(file_id) = self.uri_file_cache.get(uri) {
            return Ok(*file_id);
        }

        let conn = self.conn.read();

        // Try to get existing file
        let existing: Option<i64> = conn
            .query_row(
                "SELECT id FROM files WHERE uri = ?1",
                params![uri],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(id) = existing {
            let file_id = FileId::new(id as u32);
            self.uri_file_cache.insert(uri.to_string(), file_id);
            self.file_uri_cache.insert(file_id, uri.to_string());
            return Ok(file_id);
        }

        // Create new file entry
        drop(conn);
        let mut conn = self.conn.write();

        let mut next_id = self.next_file_id.write();
        let file_id = FileId::new(*next_id as u32);
        *next_id += 1;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO files (id, uri, path, content_hash, last_modified, last_indexed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                file_id.as_u32() as i64,
                uri,
                path.to_string_lossy().as_ref(),
                &[0u8; 16],  // Empty hash initially
                now,
                now,
            ],
        )?;

        self.uri_file_cache.insert(uri.to_string(), file_id);
        self.file_uri_cache.insert(file_id, uri.to_string());

        Ok(file_id)
    }

    /// Get file ID for a URI
    pub fn get_file_id(&self, uri: &str) -> Option<FileId> {
        self.uri_file_cache.get(uri).map(|r| *r)
    }

    /// Get URI for a file ID
    pub fn get_file_uri(&self, file_id: FileId) -> Option<String> {
        self.file_uri_cache.get(&file_id).map(|r| r.clone())
    }

    /// Update file content hash
    pub fn update_file_hash(&self, file_id: FileId, content: &str) -> Result<()> {
        let hash = xxh3_128(content.as_bytes());
        let hash_bytes = hash.to_le_bytes();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let conn = self.conn.write();
        conn.execute(
            "UPDATE files SET content_hash = ?1, last_modified = ?2 WHERE id = ?3",
            params![&hash_bytes[..], now, file_id.as_u32() as i64],
        )?;

        Ok(())
    }

    /// Check if file has changed based on content hash
    pub fn has_file_changed(&self, file_id: FileId, content: &str) -> Result<bool> {
        let new_hash = xxh3_128(content.as_bytes());
        let new_hash_bytes = new_hash.to_le_bytes();

        let conn = self.conn.read();
        let existing_hash: Option<Vec<u8>> = conn
            .query_row(
                "SELECT content_hash FROM files WHERE id = ?1",
                params![file_id.as_u32() as i64],
                |row| row.get(0),
            )
            .optional()?;

        Ok(existing_hash.map_or(true, |h| h != new_hash_bytes.to_vec()))
    }

    /// Insert or update a symbol
    pub fn upsert_symbol(&self, symbol: &Symbol) -> Result<SymbolId> {
        let conn = self.conn.write();

        let interner = self.interner.read();
        let name_str = interner.resolve(symbol.name);
        let qualified_name_str = symbol.qualified_name
            .map(|s| interner.resolve(s))
            .unwrap_or_default();

        let stable_id = Self::generate_stable_id(
            symbol.file_id,
            &qualified_name_str,
            symbol.kind,
        );

        // Try to find existing symbol by stable_id
        let existing_id: Option<i64> = conn
            .query_row(
                "SELECT id FROM symbols WHERE stable_id = ?1",
                params![&stable_id[..]],
                |row| row.get(0),
            )
            .optional()?;

        let symbol_id = if let Some(id) = existing_id {
            // Update existing symbol
            conn.execute(
                r#"UPDATE symbols SET
                    name = ?1,
                    qualified_name = ?2,
                    kind = ?3,
                    decl_file_id = ?4,
                    decl_span_start = ?5,
                    decl_span_end = ?6,
                    impl_file_id = ?7,
                    impl_span_start = ?8,
                    impl_span_end = ?9,
                    parent_id = ?10,
                    visibility = ?11,
                    is_const = ?12,
                    is_static = ?13,
                    is_virtual = ?14,
                    is_override = ?15,
                    is_final = ?16,
                    is_inline = ?17,
                    is_abstract = ?18,
                    is_template = ?19,
                    is_exported = ?20,
                    doc_comment = ?21
                WHERE id = ?22"#,
                params![
                    name_str,
                    if qualified_name_str.is_empty() { None } else { Some(qualified_name_str.as_str()) },
                    symbol.kind as u8,
                    symbol.file_id.as_u32() as i64,
                    symbol.span.start as i64,
                    symbol.span.end as i64,
                    symbol.implementation_span.as_ref().map(|s| s.file_id.as_u32() as i64),
                    symbol.implementation_span.as_ref().map(|s| s.start as i64),
                    symbol.implementation_span.as_ref().map(|s| s.end as i64),
                    symbol.parent.map(|p| p.as_u32() as i64),
                    symbol.visibility as u8,
                    symbol.flags.is_const as i64,
                    symbol.flags.is_static as i64,
                    symbol.flags.is_virtual as i64,
                    symbol.flags.is_override as i64,
                    symbol.flags.is_final as i64,
                    symbol.flags.is_inline as i64,
                    symbol.flags.is_abstract as i64,
                    symbol.flags.is_template as i64,
                    symbol.flags.is_exported as i64,
                    symbol.doc_comment.as_deref(),
                    id,
                ],
            )?;
            SymbolId::new(id as u32)
        } else {
            // Insert new symbol
            let mut next_id = self.next_symbol_id.write();
            let new_id = *next_id;
            *next_id += 1;
            drop(next_id);

            conn.execute(
                r#"INSERT INTO symbols (
                    id, stable_id, name, qualified_name, kind,
                    decl_file_id, decl_span_start, decl_span_end,
                    impl_file_id, impl_span_start, impl_span_end,
                    parent_id, visibility,
                    is_const, is_static, is_virtual, is_override, is_final,
                    is_inline, is_abstract, is_template, is_exported,
                    doc_comment
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                    ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23
                )"#,
                params![
                    new_id,
                    &stable_id[..],
                    name_str,
                    if qualified_name_str.is_empty() { None } else { Some(qualified_name_str.as_str()) },
                    symbol.kind as u8,
                    symbol.file_id.as_u32() as i64,
                    symbol.span.start as i64,
                    symbol.span.end as i64,
                    symbol.implementation_span.as_ref().map(|s| s.file_id.as_u32() as i64),
                    symbol.implementation_span.as_ref().map(|s| s.start as i64),
                    symbol.implementation_span.as_ref().map(|s| s.end as i64),
                    symbol.parent.map(|p| p.as_u32() as i64),
                    symbol.visibility as u8,
                    symbol.flags.is_const as i64,
                    symbol.flags.is_static as i64,
                    symbol.flags.is_virtual as i64,
                    symbol.flags.is_override as i64,
                    symbol.flags.is_final as i64,
                    symbol.flags.is_inline as i64,
                    symbol.flags.is_abstract as i64,
                    symbol.flags.is_template as i64,
                    symbol.flags.is_exported as i64,
                    symbol.doc_comment.as_deref(),
                ],
            )?;
            SymbolId::new(new_id as u32)
        };

        // Update cache
        self.symbol_cache.insert(symbol_id, Arc::new(symbol.clone()));

        Ok(symbol_id)
    }

    /// Get symbol by ID
    pub fn get_symbol(&self, id: SymbolId) -> Result<Option<Arc<Symbol>>> {
        // Check cache first
        if let Some(symbol) = self.symbol_cache.get(&id) {
            return Ok(Some(symbol.clone()));
        }

        // Query database
        let conn = self.conn.read();
        let symbol = self.load_symbol_from_db(&conn, id)?;

        // Update cache if found
        if let Some(ref s) = symbol {
            self.symbol_cache.insert(id, s.clone());
        }

        Ok(symbol)
    }

    /// Load symbol from database
    fn load_symbol_from_db(&self, conn: &Connection, id: SymbolId) -> Result<Option<Arc<Symbol>>> {
        let interner = self.interner.clone();

        let result: Option<Symbol> = conn
            .query_row(
                r#"SELECT
                    id, name, qualified_name, kind,
                    decl_file_id, decl_span_start, decl_span_end,
                    impl_file_id, impl_span_start, impl_span_end,
                    parent_id, visibility,
                    is_const, is_static, is_virtual, is_override, is_final,
                    is_inline, is_abstract, is_template, is_exported,
                    doc_comment
                FROM symbols WHERE id = ?1"#,
                params![id.as_u32() as i64],
                |row| {
                    let interner_guard = interner.read();
                    let decl_file_id = FileId::new(row.get::<_, i64>(4)? as u32);
                    let decl_span = Span::new(
                        decl_file_id,
                        row.get::<_, i64>(5)? as u32,
                        row.get::<_, i64>(6)? as u32,
                    );

                    let impl_span = match (
                        row.get::<_, Option<i64>>(7)?,
                        row.get::<_, Option<i64>>(8)?,
                        row.get::<_, Option<i64>>(9)?,
                    ) {
                        (Some(file_id), Some(start), Some(end)) => Some(Span::new(
                            FileId::new(file_id as u32),
                            start as u32,
                            end as u32,
                        )),
                        _ => None,
                    };

                    let name_str: String = row.get(1)?;
                    let qualified_name_str: Option<String> = row.get(2)?;

                    drop(interner_guard);

                    Ok(Symbol {
                        id: SymbolId::new(row.get::<_, i64>(0)? as u32),
                        kind: unsafe { std::mem::transmute(row.get::<_, u8>(3)?) },
                        name: interner.write().intern(&name_str),
                        qualified_name: qualified_name_str.as_ref().map(|s| interner.write().intern(s)),
                        span: decl_span,
                        file_id: decl_file_id,
                        parent: row.get::<_, Option<i64>>(10)?.map(|p| SymbolId::new(p as u32)),
                        children: Vec::new(),  // Loaded separately if needed
                        visibility: unsafe { std::mem::transmute(row.get::<_, u8>(11)?) },
                        flags: SymbolFlags {
                            is_const: row.get::<_, i64>(12)? != 0,
                            is_static: row.get::<_, i64>(13)? != 0,
                            is_virtual: row.get::<_, i64>(14)? != 0,
                            is_override: row.get::<_, i64>(15)? != 0,
                            is_final: row.get::<_, i64>(16)? != 0,
                            is_inline: row.get::<_, i64>(17)? != 0,
                            is_abstract: row.get::<_, i64>(18)? != 0,
                            is_template: row.get::<_, i64>(19)? != 0,
                            is_exported: row.get::<_, i64>(20)? != 0,
                        },
                        symbol_type: None,  // TODO: Load from type_info JSON
                        bases: Vec::new(),  // Loaded from inheritance table
                        derived: Vec::new(),
                        overrides: None,  // Loaded from overrides table
                        overridden_by: Vec::new(),
                        doc_comment: row.get::<_, Option<String>>(21)?,
                        implementation_span: impl_span,
                    })
                },
            )
            .optional()?;

        Ok(result.map(Arc::new))
    }

    /// Get all symbols in a file
    pub fn get_file_symbols(&self, file_id: FileId) -> Result<Vec<Arc<Symbol>>> {
        let conn = self.conn.read();

        let mut stmt = conn.prepare(
            "SELECT id FROM symbols WHERE decl_file_id = ?1 OR impl_file_id = ?1",
        )?;

        let symbol_ids: Vec<SymbolId> = stmt
            .query_map(params![file_id.as_u32() as i64], |row| {
                Ok(SymbolId::new(row.get::<_, i64>(0)? as u32))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        drop(stmt);
        drop(conn);

        let mut symbols = Vec::new();
        for id in symbol_ids {
            if let Some(symbol) = self.get_symbol(id)? {
                symbols.push(symbol);
            }
        }

        Ok(symbols)
    }

    /// Remove all symbols from a file (only the declarations, preserve implementations)
    pub fn remove_file_declarations(&self, file_id: FileId) -> Result<()> {
        let conn = self.conn.write();

        // Only remove symbols where BOTH declaration AND implementation are in this file
        // This preserves .cpp implementations when .h file is reparsed
        conn.execute(
            "DELETE FROM symbols WHERE decl_file_id = ?1 AND (impl_file_id IS NULL OR impl_file_id = ?1)",
            params![file_id.as_u32() as i64],
        )?;

        // Clear cache for this file
        self.symbol_cache.retain(|_, symbol| {
            symbol.file_id != file_id || symbol.implementation_span.as_ref().map(|s| s.file_id) != Some(file_id)
        });

        Ok(())
    }

    /// Update symbol implementation span (preserve declaration)
    pub fn update_symbol_implementation(&self, symbol_id: SymbolId, impl_span: Span) -> Result<()> {
        let conn = self.conn.write();

        conn.execute(
            "UPDATE symbols SET impl_file_id = ?1, impl_span_start = ?2, impl_span_end = ?3 WHERE id = ?4",
            params![
                impl_span.file_id.as_u32() as i64,
                impl_span.start as i64,
                impl_span.end as i64,
                symbol_id.as_u32() as i64,
            ],
        )?;

        // Invalidate cache
        self.symbol_cache.remove(&symbol_id);

        Ok(())
    }

    /// Find symbol by qualified name
    pub fn find_symbol_by_qualified_name(&self, qualified_name: &str) -> Result<Option<Arc<Symbol>>> {
        let conn = self.conn.read();

        let id: Option<i64> = conn
            .query_row(
                "SELECT id FROM symbols WHERE qualified_name = ?1 LIMIT 1",
                params![qualified_name],
                |row| row.get(0),
            )
            .optional()?;

        drop(conn);

        if let Some(id) = id {
            self.get_symbol(SymbolId::new(id as u32))
        } else {
            Ok(None)
        }
    }

    /// Search symbols using FTS5
    pub fn search_symbols(&self, query: &str, limit: usize) -> Result<Vec<Arc<Symbol>>> {
        let conn = self.conn.read();

        let mut stmt = conn.prepare(
            "SELECT symbol_id FROM symbols_fts WHERE symbols_fts MATCH ?1 LIMIT ?2",
        )?;

        let symbol_ids: Vec<SymbolId> = stmt
            .query_map(params![query, limit as i64], |row| {
                Ok(SymbolId::new(row.get::<_, i64>(0)? as u32))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        drop(stmt);
        drop(conn);

        let mut symbols = Vec::new();
        for id in symbol_ids {
            if let Some(symbol) = self.get_symbol(id)? {
                symbols.push(symbol);
            }
        }

        Ok(symbols)
    }

    /// Vacuum database to reclaim space
    pub fn vacuum(&self) -> Result<()> {
        let conn = self.conn.write();
        conn.execute_batch("VACUUM; ANALYZE;")?;
        Ok(())
    }

    /// Close database connection
    pub fn close(self) -> Result<()> {
        // Flush WAL
        let conn = Arc::try_unwrap(self.conn)
            .map_err(|_| anyhow::anyhow!("Cannot close database: connections still in use"))?
            .into_inner();

        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        conn.close().map_err(|(_, e)| e.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_database_creation() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let db = DatabaseManager::new(&db_path).unwrap();
        drop(db);

        assert!(db_path.exists());
    }

    #[test]
    fn test_file_management() {
        let dir = tempdir().unwrap();
        let db = DatabaseManager::new(dir.path().join("test.db")).unwrap();

        let uri = "file:///test/file.cpp";
        let path = Path::new("/test/file.cpp");

        let file_id = db.get_or_create_file_id(uri, path).unwrap();
        assert_eq!(db.get_file_id(uri), Some(file_id));
        assert_eq!(db.get_file_uri(file_id), Some(uri.to_string()));
    }

    #[test]
    fn test_content_hash() {
        let dir = tempdir().unwrap();
        let db = DatabaseManager::new(dir.path().join("test.db")).unwrap();

        let uri = "file:///test/file.cpp";
        let path = Path::new("/test/file.cpp");
        let file_id = db.get_or_create_file_id(uri, path).unwrap();

        let content1 = "int main() {}";
        assert!(db.has_file_changed(file_id, content1).unwrap());

        db.update_file_hash(file_id, content1).unwrap();
        assert!(!db.has_file_changed(file_id, content1).unwrap());

        let content2 = "int main() { return 0; }";
        assert!(db.has_file_changed(file_id, content2).unwrap());
    }
}
