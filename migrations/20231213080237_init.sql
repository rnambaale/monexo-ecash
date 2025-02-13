CREATE TABLE used_proofs (
    amount BIGINT NOT NULL,
    secret TEXT NOT NULL PRIMARY KEY,
    c TEXT NOT NULL,
    keyset_id TEXT NOT NULL
);

CREATE TABLE pending_invoices (
    key TEXT NOT NULL PRIMARY KEY,
    payment_request TEXT NOT NULL,
    amount BIGINT NOT NULL
);

CREATE TABLE bolt11_mint_quotes (
    id UUID PRIMARY KEY NOT NULL,
    payment_request TEXT NOT NULL,
    expiry BIGINT NOT NULL,
    paid BOOLEAN NOT NULL
);

CREATE TABLE bolt11_melt_quotes (
    id UUID PRIMARY KEY NOT NULL,
    payment_request TEXT NOT NULL,
    expiry BIGINT NOT NULL,
    paid BOOLEAN NOT NULL,
    amount BIGINT NOT NULL,
    fee_reserve BIGINT NOT NULL
);

CREATE TABLE onchain_mint_quotes (
    id uuid NOT NULL,
    reference text COLLATE pg_catalog."default" NOT NULL,
	amount bigint NOT NULL,
    expiry bigint NOT NULL,
    state TEXT NOT NULL,
    CONSTRAINT onchain_mint_quotes_pkey PRIMARY KEY (id)
);

CREATE TABLE onchain_melt_quotes (
    id uuid NOT NULL,
	amount bigint NOT NULL,
    address text COLLATE pg_catalog."default" NOT NULL,
    fee_total bigint NOT NULL,
    fee_sat_per_vbyte bigint NOT NULL,
    expiry bigint NOT NULL,
    description text,
    state TEXT NOT NULL,
    CONSTRAINT onchain_melt_quotes_pkey PRIMARY KEY (id)
);
