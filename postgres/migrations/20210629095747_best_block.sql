CREATE TABLE IF NOT EXISTS best_block (
    only_one boolean PRIMARY KEY DEFAULT TRUE,

    block_num integer CHECK (block_num >= 0) NOT NULL,
    block_hash bytea NOT NULL,

    CONSTRAINT only_one_row CHECK (only_one)
);
