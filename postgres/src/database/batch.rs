//! A method of dynamic queries with SQLx
//! Taken from this Gist by @mehcode (Github): https://gist.github.com/mehcode/c476922be0290a4f8502d18701cc8c74
//! This is sort of temporary until SQLx develops their dynamic query builder: https://github.com/launchbadge/sqlx/issues/291
//! and `Quaint` switches to SQLx as a backend: https://github.com/prisma/quaint/issues/138

use sqlx::{
    postgres::{PgArguments, PgConnection, Postgres},
    prelude::*,
    Arguments, Error as SqlxError,
};

const CHUNK_MAX: usize = 30_000;

pub struct Chunk {
    query: String,
    pub arguments: PgArguments,
    // FIXME: Would be nice if PgArguments exposed the # of args as `.len()`
    pub args_len: usize,
}

impl Chunk {
    fn new(sql: &str) -> Self {
        let mut query = String::with_capacity(1024 * 8);
        query.push_str(sql);
        Self {
            query,
            arguments: PgArguments::default(),
            args_len: 0,
        }
    }

    pub fn append(&mut self, sql: &str) {
        self.query.push_str(sql);
    }

    pub fn bind<'a, T: 'a>(&mut self, value: T) -> Result<(), SqlxError>
    where
        T: Encode<'a, Postgres> + Type<Postgres> + Send,
    {
        self.arguments.add(value);
        self.args_len += 1;
        self.query.push_str(&format!("${}", self.args_len));
        Ok(())
    }

    async fn execute(self, conn: &mut PgConnection) -> Result<u64, SqlxError> {
        Ok(sqlx::query_with(&*self.query, self.arguments)
            .execute(conn)
            .await?
            .rows_affected())
    }
}

type WithFunc = Box<dyn Fn(&mut Chunk) -> Result<(), SqlxError> + Send>;

pub struct Batch {
    name: &'static str,
    leading: String,
    trailing: String,
    with: Option<WithFunc>,
    chunks: Vec<Chunk>,
    index: usize,
    len: usize,
}

impl Batch {
    pub fn new(name: &'static str, leading: &str, trailing: &str) -> Self {
        Self {
            name,
            leading: leading.to_owned(),
            trailing: trailing.to_owned(),
            chunks: vec![Chunk::new(leading)],
            with: None,
            index: 0,
            len: 0,
        }
    }

    pub fn new_with(
        name: &'static str,
        leading: &str,
        trailing: &str,
        with: impl Fn(&mut Chunk) -> Result<(), SqlxError> + Send + 'static,
    ) -> Result<Self, SqlxError> {
        let mut chunk = Chunk::new(leading);
        with(&mut chunk)?;
        Ok(Self {
            name,
            leading: leading.to_owned(),
            trailing: trailing.to_owned(),
            with: Some(Box::new(with)),
            chunks: vec![chunk],
            index: 0,
            len: 0,
        })
    }

    // ensure there is enough room for N more arguments
    pub fn reserve(&mut self, arguments: usize) -> Result<(), SqlxError> {
        self.len += 1;

        if self.chunks[self.index].args_len + arguments > CHUNK_MAX {
            let mut chunk = Chunk::new(&self.leading);

            if let Some(with) = &self.with {
                with(&mut chunk)?;
            }

            self.chunks.push(chunk);
            self.index += 1;
        }

        Ok(())
    }

    pub fn append(&mut self, sql: &str) {
        self.chunks[self.index].append(sql);
    }

    pub fn bind<'a, T: 'a>(&mut self, value: T) -> Result<(), SqlxError>
    where
        T: Encode<'a, Postgres> + Type<Postgres> + Send,
    {
        self.chunks[self.index].bind(value)
    }

    pub async fn execute(self, conn: &mut PgConnection) -> Result<u64, SqlxError> {
        let mut rows_affected = 0;
        if self.len > 0 {
            for mut chunk in self.chunks {
                chunk.append(&self.trailing);
                rows_affected += chunk.execute(&mut *conn).await?;
            }
        }
        Ok(rows_affected)
    }

    // TODO: Better name?
    pub fn current_num_arguments(&self) -> usize {
        self.chunks[self.index].args_len
    }
}
