-- Add migration script here
CREATE TABLE IF NOT EXISTS proofs (
    keyset_id TEXT NOT NULL,
    amount INTEGER NOT NULL,
    C TEXT NOT NULL,
    secret TEXT NOT NULL,
    time_created TIMESTAMP,
    UNIQUE (secret)
);

CREATE TABLE IF NOT EXISTS keysets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    keyset_id TEXT NOT NULL,
    active BOOL NOT NULL DEFAULT TRUE,
    last_index INTEGER NOT NULL,
    public_keys TEXT NOT NULL CHECK (json_valid(public_keys)),
    UNIQUE (keyset_id)
);

-- Add migration script here
CREATE TABLE seed (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    seed_words TEXT NOT NULL
    -- other columns
);
