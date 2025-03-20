CREATE TABLE used_proofs (
    amount BIGINT NOT NULL,
    secret TEXT NOT NULL PRIMARY KEY,
    c TEXT NOT NULL,
    keyset_id TEXT NOT NULL
);

CREATE TABLE onchain_mint_quotes (
    id uuid NOT NULL,
    reference text COLLATE pg_catalog."default" NOT NULL,
    fee_total bigint NOT NULL,
	amount bigint NOT NULL,
    expiry bigint NOT NULL,
    state TEXT NOT NULL,
    CONSTRAINT onchain_mint_quotes_pkey PRIMARY KEY (id)
);

CREATE TABLE onchain_melt_quotes (
    id uuid NOT NULL,
	amount bigint NOT NULL,
    address text COLLATE pg_catalog."default" NOT NULL,
    reference text COLLATE pg_catalog."default" NOT NULL,
    fee_total bigint NOT NULL,
    fee_sat_per_vbyte bigint NOT NULL,
    expiry bigint NOT NULL,
    description text,
    state TEXT NOT NULL,
    CONSTRAINT onchain_melt_quotes_pkey PRIMARY KEY (id)
);

-- Keysets Table
CREATE TABLE IF NOT EXISTS keysets (
    id TEXT PRIMARY KEY,
    unit TEXT NOT NULL,
    active BOOL NOT NULL,
    valid_from INTEGER NOT NULL,
    valid_to INTEGER,
    derivation_path TEXT NOT NULL,
    max_order INTEGER NOT NULL,
    input_fee_ppk INTEGER,
    derivation_path_index INTEGER
);

CREATE INDEX IF NOT EXISTS unit_index ON keysets(unit);
CREATE INDEX IF NOT EXISTS active_index ON keysets(active);
