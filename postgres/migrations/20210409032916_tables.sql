CREATE TABLE IF NOT EXISTS metadata (
    spec_version integer CHECK (spec_version >= 0) NOT NULL,

    block_num integer CHECK (block_num >= 0) NOT NULL,
    block_hash bytea NOT NULL,

    meta bytea NOT NULL,

    PRIMARY KEY (spec_version)
);

CREATE TABLE IF NOT EXISTS block (
    spec_version integer NOT NULL REFERENCES metadata (spec_version),

    block_num integer CHECK (block_num >= 0) NOT NULL,
    block_hash bytea NOT NULL,
    parent_hash bytea NOT NULL,
    state_root bytea NOT NULL,
    extrinsics_root bytea NOT NULL,
    digest bytea NOT NULL,
    extrinsics bytea[] NOT NULL,

    justifications bytea[],

    PRIMARY KEY (block_num)
) PARTITION BY RANGE (block_num);

-- Need to add a new child table when block_num is larger.
CREATE TABLE IF NOT EXISTS block_0 PARTITION OF block FOR VALUES FROM (0) TO (1000000);
CREATE TABLE IF NOT EXISTS block_1 PARTITION OF block FOR VALUES FROM (1000000) TO (2000000);
CREATE TABLE IF NOT EXISTS block_2 PARTITION OF block FOR VALUES FROM (2000000) TO (3000000);
CREATE TABLE IF NOT EXISTS block_3 PARTITION OF block FOR VALUES FROM (3000000) TO (4000000);
CREATE TABLE IF NOT EXISTS block_4 PARTITION OF block FOR VALUES FROM (4000000) TO (5000000);
CREATE TABLE IF NOT EXISTS block_5 PARTITION OF block FOR VALUES FROM (5000000) TO (6000000);
CREATE TABLE IF NOT EXISTS block_6 PARTITION OF block FOR VALUES FROM (6000000) TO (7000000);
CREATE TABLE IF NOT EXISTS block_7 PARTITION OF block FOR VALUES FROM (7000000) TO (8000000);
CREATE TABLE IF NOT EXISTS block_8 PARTITION OF block FOR VALUES FROM (8000000) TO (9000000);
CREATE TABLE IF NOT EXISTS block_9 PARTITION OF block FOR VALUES FROM (9000000) TO (10000000);

CREATE TABLE IF NOT EXISTS main_storage (
    block_num integer CHECK (block_num >= 0) NOT NULL REFERENCES block (block_num),
    block_hash bytea NOT NULL,

    prefix bytea NOT NULL,
    key bytea NOT NULL,
    data bytea,

    PRIMARY KEY (block_num, key)
) PARTITION BY RANGE (block_num);

-- Need to add a new child table when block_num is larger.
CREATE TABLE IF NOT EXISTS main_storage_0 PARTITION OF main_storage FOR VALUES FROM (0) TO (1000000);
CREATE TABLE IF NOT EXISTS main_storage_1 PARTITION OF main_storage FOR VALUES FROM (1000000) TO (2000000);
CREATE TABLE IF NOT EXISTS main_storage_2 PARTITION OF main_storage FOR VALUES FROM (2000000) TO (3000000);
CREATE TABLE IF NOT EXISTS main_storage_3 PARTITION OF main_storage FOR VALUES FROM (3000000) TO (4000000);
CREATE TABLE IF NOT EXISTS main_storage_4 PARTITION OF main_storage FOR VALUES FROM (4000000) TO (5000000);
CREATE TABLE IF NOT EXISTS main_storage_5 PARTITION OF main_storage FOR VALUES FROM (5000000) TO (6000000);
CREATE TABLE IF NOT EXISTS main_storage_6 PARTITION OF main_storage FOR VALUES FROM (6000000) TO (7000000);
CREATE TABLE IF NOT EXISTS main_storage_7 PARTITION OF main_storage FOR VALUES FROM (7000000) TO (8000000);
CREATE TABLE IF NOT EXISTS main_storage_8 PARTITION OF main_storage FOR VALUES FROM (8000000) TO (9000000);
CREATE TABLE IF NOT EXISTS main_storage_9 PARTITION OF main_storage FOR VALUES FROM (9000000) TO (10000000);

-- CREATE TABLE IF NOT EXISTS child_storage (
--     block_num integer CHECK (block_num >= 0) NOT NULL REFERENCES block (block_num),
--     block_hash bytea NOT NULL,
--
--     prefix_key bytea NOT NULL,
--     key bytea NOT NULL,
--     data bytea,
--
--     PRIMARY KEY (block_num, prefix_key, key)
-- ) PARTITION BY RANGE (block_num);

-- Need to add a new child table when block_num is larger.
-- CREATE TABLE IF NOT EXISTS child_storage_0 PARTITION OF child_storage FOR VALUES FROM (0) TO (1000000);
-- CREATE TABLE IF NOT EXISTS child_storage_1 PARTITION OF child_storage FOR VALUES FROM (1000000) TO (2000000);
-- CREATE TABLE IF NOT EXISTS child_storage_2 PARTITION OF child_storage FOR VALUES FROM (2000000) TO (3000000);
-- CREATE TABLE IF NOT EXISTS child_storage_3 PARTITION OF child_storage FOR VALUES FROM (3000000) TO (4000000);
-- CREATE TABLE IF NOT EXISTS child_storage_4 PARTITION OF child_storage FOR VALUES FROM (4000000) TO (5000000);
-- CREATE TABLE IF NOT EXISTS child_storage_5 PARTITION OF child_storage FOR VALUES FROM (5000000) TO (6000000);
-- CREATE TABLE IF NOT EXISTS child_storage_6 PARTITION OF child_storage FOR VALUES FROM (6000000) TO (7000000);
-- CREATE TABLE IF NOT EXISTS child_storage_7 PARTITION OF child_storage FOR VALUES FROM (7000000) TO (8000000);
-- CREATE TABLE IF NOT EXISTS child_storage_8 PARTITION OF child_storage FOR VALUES FROM (8000000) TO (9000000);
-- CREATE TABLE IF NOT EXISTS child_storage_9 PARTITION OF child_storage FOR VALUES FROM (9000000) TO (10000000);

CREATE TABLE IF NOT EXISTS best_block (
    only_one boolean PRIMARY KEY DEFAULT TRUE,

    block_num integer CHECK (block_num >= 0) NOT NULL,
    block_hash bytea NOT NULL,

    CONSTRAINT only_one_row CHECK (only_one)
);

CREATE TABLE IF NOT EXISTS finalized_block (
    only_one boolean PRIMARY KEY DEFAULT TRUE,

    block_num integer CHECK (block_num >= 0) NOT NULL,
    block_hash bytea NOT NULL,

    CONSTRAINT only_one_row CHECK (only_one)
);
