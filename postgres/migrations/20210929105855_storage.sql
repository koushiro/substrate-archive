-- Add migration script here
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
CREATE TABLE IF NOT EXISTS main_storage_10 PARTITION OF main_storage FOR VALUES FROM (10000000) TO (11000000);
CREATE TABLE IF NOT EXISTS main_storage_11 PARTITION OF main_storage FOR VALUES FROM (11000000) TO (12000000);
CREATE TABLE IF NOT EXISTS main_storage_12 PARTITION OF main_storage FOR VALUES FROM (12000000) TO (13000000);
CREATE TABLE IF NOT EXISTS main_storage_13 PARTITION OF main_storage FOR VALUES FROM (13000000) TO (14000000);
CREATE TABLE IF NOT EXISTS main_storage_14 PARTITION OF main_storage FOR VALUES FROM (14000000) TO (15000000);
CREATE TABLE IF NOT EXISTS main_storage_15 PARTITION OF main_storage FOR VALUES FROM (15000000) TO (16000000);
CREATE TABLE IF NOT EXISTS main_storage_16 PARTITION OF main_storage FOR VALUES FROM (16000000) TO (17000000);
CREATE TABLE IF NOT EXISTS main_storage_17 PARTITION OF main_storage FOR VALUES FROM (17000000) TO (18000000);
CREATE TABLE IF NOT EXISTS main_storage_18 PARTITION OF main_storage FOR VALUES FROM (18000000) TO (19000000);
CREATE TABLE IF NOT EXISTS main_storage_19 PARTITION OF main_storage FOR VALUES FROM (19000000) TO (20000000);

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
