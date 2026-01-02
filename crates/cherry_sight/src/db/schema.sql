-- CherrySight 2.0 Database Schema
-- Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

-- Core metadata table
CREATE TABLE IF NOT EXISTS metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
) STRICT;

-- Files table
CREATE TABLE IF NOT EXISTS files (
    id INTEGER PRIMARY KEY,
    uri TEXT NOT NULL UNIQUE,
    path TEXT NOT NULL,
    content_hash BLOB NOT NULL,  -- xxhash3 of file content
    last_modified INTEGER NOT NULL,  -- Unix timestamp
    last_indexed INTEGER NOT NULL    -- Unix timestamp
) STRICT;

CREATE INDEX IF NOT EXISTS idx_files_uri ON files(uri);
CREATE INDEX IF NOT EXISTS idx_files_path ON files(path);

-- Symbols table (declaration + definition)
CREATE TABLE IF NOT EXISTS symbols (
    id INTEGER PRIMARY KEY,

    -- Identity (stable across reparses using xxhash of file_id + qualified_name + kind)
    stable_id BLOB NOT NULL UNIQUE,  -- 16 bytes xxhash3

    -- Basic info
    name TEXT NOT NULL,
    qualified_name TEXT NOT NULL,
    kind INTEGER NOT NULL,  -- SymbolKind as u8

    -- Declaration location
    decl_file_id INTEGER NOT NULL,
    decl_span_start INTEGER NOT NULL,
    decl_span_end INTEGER NOT NULL,

    -- Implementation/definition location (NULL if same as declaration)
    impl_file_id INTEGER,
    impl_span_start INTEGER,
    impl_span_end INTEGER,

    -- Hierarchy
    parent_id INTEGER,  -- Foreign key to symbols.id

    -- Metadata
    visibility INTEGER NOT NULL DEFAULT 0,  -- Visibility as u8
    is_const INTEGER NOT NULL DEFAULT 0,
    is_static INTEGER NOT NULL DEFAULT 0,
    is_virtual INTEGER NOT NULL DEFAULT 0,
    is_override INTEGER NOT NULL DEFAULT 0,
    is_final INTEGER NOT NULL DEFAULT 0,
    is_inline INTEGER NOT NULL DEFAULT 0,
    is_abstract INTEGER NOT NULL DEFAULT 0,
    is_template INTEGER NOT NULL DEFAULT 0,
    is_exported INTEGER NOT NULL DEFAULT 0,

    -- Type information (serialized as JSON for complex types)
    type_info TEXT,

    -- Documentation
    doc_comment TEXT,

    FOREIGN KEY (decl_file_id) REFERENCES files(id) ON DELETE CASCADE,
    FOREIGN KEY (impl_file_id) REFERENCES files(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_id) REFERENCES symbols(id) ON DELETE CASCADE
) STRICT;

CREATE INDEX IF NOT EXISTS idx_symbols_qualified_name ON symbols(qualified_name);
CREATE INDEX IF NOT EXISTS idx_symbols_name ON symbols(name);
CREATE INDEX IF NOT EXISTS idx_symbols_decl_file ON symbols(decl_file_id);
CREATE INDEX IF NOT EXISTS idx_symbols_impl_file ON symbols(impl_file_id);
CREATE INDEX IF NOT EXISTS idx_symbols_parent ON symbols(parent_id);
CREATE INDEX IF NOT EXISTS idx_symbols_kind ON symbols(kind);
CREATE INDEX IF NOT EXISTS idx_symbols_stable_id ON symbols(stable_id);

-- Full-text search for symbols
CREATE VIRTUAL TABLE IF NOT EXISTS symbols_fts USING fts5(
    symbol_id UNINDEXED,
    name,
    qualified_name,
    content='symbols',
    content_rowid='id'
);

-- Triggers to keep FTS in sync
CREATE TRIGGER IF NOT EXISTS symbols_fts_insert AFTER INSERT ON symbols BEGIN
    INSERT INTO symbols_fts(rowid, symbol_id, name, qualified_name)
    VALUES (new.id, new.id, new.name, new.qualified_name);
END;

CREATE TRIGGER IF NOT EXISTS symbols_fts_delete AFTER DELETE ON symbols BEGIN
    DELETE FROM symbols_fts WHERE rowid = old.id;
END;

CREATE TRIGGER IF NOT EXISTS symbols_fts_update AFTER UPDATE ON symbols BEGIN
    DELETE FROM symbols_fts WHERE rowid = old.id;
    INSERT INTO symbols_fts(rowid, symbol_id, name, qualified_name)
    VALUES (new.id, new.id, new.name, new.qualified_name);
END;

-- Inheritance relationships
CREATE TABLE IF NOT EXISTS inheritance (
    derived_id INTEGER NOT NULL,
    base_id INTEGER NOT NULL,
    access_specifier INTEGER NOT NULL,  -- public=0, protected=1, private=2
    is_virtual INTEGER NOT NULL DEFAULT 0,

    PRIMARY KEY (derived_id, base_id),
    FOREIGN KEY (derived_id) REFERENCES symbols(id) ON DELETE CASCADE,
    FOREIGN KEY (base_id) REFERENCES symbols(id) ON DELETE CASCADE
) STRICT;

CREATE INDEX IF NOT EXISTS idx_inheritance_base ON inheritance(base_id);
CREATE INDEX IF NOT EXISTS idx_inheritance_derived ON inheritance(derived_id);

-- Method override relationships
CREATE TABLE IF NOT EXISTS overrides (
    overriding_id INTEGER NOT NULL,
    overridden_id INTEGER NOT NULL,

    PRIMARY KEY (overriding_id, overridden_id),
    FOREIGN KEY (overriding_id) REFERENCES symbols(id) ON DELETE CASCADE,
    FOREIGN KEY (overridden_id) REFERENCES symbols(id) ON DELETE CASCADE
) STRICT;

CREATE INDEX IF NOT EXISTS idx_overrides_overridden ON overrides(overridden_id);
CREATE INDEX IF NOT EXISTS idx_overrides_overriding ON overrides(overriding_id);

-- References/usages
CREATE TABLE IF NOT EXISTS references (
    id INTEGER PRIMARY KEY,
    symbol_id INTEGER NOT NULL,
    file_id INTEGER NOT NULL,
    span_start INTEGER NOT NULL,
    span_end INTEGER NOT NULL,
    kind INTEGER NOT NULL,  -- ReferenceKind as u8: Declaration=0, Definition=1, Read=2, Write=3, Call=4

    FOREIGN KEY (symbol_id) REFERENCES symbols(id) ON DELETE CASCADE,
    FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE
) STRICT;

CREATE INDEX IF NOT EXISTS idx_references_symbol ON references(symbol_id);
CREATE INDEX IF NOT EXISTS idx_references_file ON references(file_id);
CREATE INDEX IF NOT EXISTS idx_references_kind ON references(kind);

-- Include dependencies
CREATE TABLE IF NOT EXISTS includes (
    id INTEGER PRIMARY KEY,
    source_file_id INTEGER NOT NULL,
    target_file_id INTEGER,  -- NULL if not resolved
    target_path TEXT NOT NULL,  -- The #include path as written
    span_start INTEGER NOT NULL,
    span_end INTEGER NOT NULL,
    is_angled INTEGER NOT NULL,  -- <> vs ""
    is_resolved INTEGER NOT NULL DEFAULT 0,

    FOREIGN KEY (source_file_id) REFERENCES files(id) ON DELETE CASCADE,
    FOREIGN KEY (target_file_id) REFERENCES files(id) ON DELETE CASCADE
) STRICT;

CREATE INDEX IF NOT EXISTS idx_includes_source ON includes(source_file_id);
CREATE INDEX IF NOT EXISTS idx_includes_target ON includes(target_file_id);

-- Template instantiations
CREATE TABLE IF NOT EXISTS template_instantiations (
    id INTEGER PRIMARY KEY,
    template_id INTEGER NOT NULL,  -- The template symbol
    instantiation_id INTEGER NOT NULL,  -- The specific instantiation symbol
    type_args TEXT NOT NULL,  -- JSON array of type arguments

    FOREIGN KEY (template_id) REFERENCES symbols(id) ON DELETE CASCADE,
    FOREIGN KEY (instantiation_id) REFERENCES symbols(id) ON DELETE CASCADE
) STRICT;

CREATE INDEX IF NOT EXISTS idx_template_inst_template ON template_instantiations(template_id);
CREATE INDEX IF NOT EXISTS idx_template_inst_instantiation ON template_instantiations(instantiation_id);

-- UE5-specific metadata
CREATE TABLE IF NOT EXISTS ue5_metadata (
    symbol_id INTEGER PRIMARY KEY,

    -- UE macro specifiers (serialized as JSON)
    uclass_specifiers TEXT,
    uproperty_specifiers TEXT,
    ufunction_specifiers TEXT,
    ustruct_specifiers TEXT,
    uenum_specifiers TEXT,

    -- Blueprint metadata
    is_blueprint_type INTEGER NOT NULL DEFAULT 0,
    is_blueprint_callable INTEGER NOT NULL DEFAULT 0,
    is_blueprint_pure INTEGER NOT NULL DEFAULT 0,
    is_blueprint_implementable_event INTEGER NOT NULL DEFAULT 0,
    is_blueprint_native_event INTEGER NOT NULL DEFAULT 0,

    -- Network replication
    is_replicated INTEGER NOT NULL DEFAULT 0,
    replication_condition TEXT,  -- RepNotify, COND_OwnerOnly, etc.

    -- Category/group for organization
    category TEXT,
    display_name TEXT,
    tooltip TEXT,

    FOREIGN KEY (symbol_id) REFERENCES symbols(id) ON DELETE CASCADE
) STRICT;

-- Performance optimization: content-based caching
-- Stores hashes of file content to detect when reparsing is needed
CREATE TABLE IF NOT EXISTS file_content_cache (
    file_id INTEGER PRIMARY KEY,
    content_hash BLOB NOT NULL,
    symbol_count INTEGER NOT NULL,
    reference_count INTEGER NOT NULL,
    last_parse_time_ms INTEGER NOT NULL,

    FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE
) STRICT;

-- Version tracking
INSERT OR REPLACE INTO metadata (key, value) VALUES ('schema_version', '1');
INSERT OR REPLACE INTO metadata (key, value) VALUES ('created_at', datetime('now'));
