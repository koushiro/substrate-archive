-- Add migration script here
CREATE TABLE IF NOT EXISTS metadata (
    version integer CHECK (version >= 0) NOT NULL,

    block_num integer CHECK (block_num >= 0) NOT NULL,
    block_hash bytea NOT NULL,

    metadata bytea NOT NULL,

    PRIMARY KEY (version)
);
