CREATE TABLE IF NOT EXISTS sessions (
 ordinal INTEGER PRIMARY KEY AUTOINCREMENT,
 id TEXT NOT NULL UNIQUE,
 device TEXT NOT NULL, installation TEXT NOT NULL, source TEXT NOT NULL,
 native TEXT NOT NULL, project TEXT NOT NULL, workspace TEXT NOT NULL,
 source_version TEXT NOT NULL, title TEXT NOT NULL DEFAULT '',
 state TEXT NOT NULL DEFAULT 'unknown', parent_native TEXT,
 sequence INTEGER NOT NULL DEFAULT 0, state_sequence INTEGER NOT NULL DEFAULT 0, observed INTEGER NOT NULL,
 UNIQUE(device,installation,source,native)
);
CREATE TABLE IF NOT EXISTS entries (
 ordinal INTEGER PRIMARY KEY AUTOINCREMENT,
 id TEXT NOT NULL UNIQUE, session TEXT NOT NULL REFERENCES sessions(id),
 native TEXT NOT NULL, kind TEXT NOT NULL, role TEXT NOT NULL DEFAULT 'unknown',
 parent_native TEXT, removed INTEGER NOT NULL DEFAULT 0,
 sequence INTEGER NOT NULL DEFAULT 0, observed INTEGER NOT NULL,
 text TEXT NOT NULL DEFAULT '', content_hash TEXT,
 UNIQUE(session,native,kind)
);
CREATE TABLE IF NOT EXISTS session_workspaces (
 session TEXT NOT NULL REFERENCES sessions(id), workspace TEXT NOT NULL,
 PRIMARY KEY(session,workspace)
);
CREATE TABLE IF NOT EXISTS parts (
 entry TEXT NOT NULL REFERENCES entries(id), native TEXT NOT NULL,
 kind TEXT NOT NULL, text TEXT NOT NULL, removed INTEGER NOT NULL,
 sequence INTEGER NOT NULL,
 PRIMARY KEY(entry,native)
);
CREATE TABLE IF NOT EXISTS captures (
 ordinal INTEGER PRIMARY KEY AUTOINCREMENT, id TEXT NOT NULL UNIQUE,
 delivery TEXT NOT NULL UNIQUE, hash TEXT NOT NULL,
 envelope_hash TEXT NOT NULL, payload_hash TEXT NOT NULL,
 session TEXT NOT NULL REFERENCES sessions(id), entry TEXT REFERENCES entries(id),
 recorded INTEGER NOT NULL, receipt TEXT NOT NULL
);
CREATE VIRTUAL TABLE IF NOT EXISTS search_index USING fts5(entry UNINDEXED, text);
PRAGMA user_version = 1;
