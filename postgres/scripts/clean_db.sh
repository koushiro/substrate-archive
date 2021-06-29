#!/usr/bin/env bash

psql -d polkadot-archive -c "
drop trigger if exists new_block_trigger on block;
drop function if exists insert_new_block_fn;

drop table if exists _sqlx_migrations;
drop table if exists cild_storage;
drop table if exists main_storage;
drop table if exists block;
drop table if exists best_block;
drop table if exists finalized_block;
drop table if exists metadata;
"
