/*
#[async_trait::async_trait]
pub trait Archive {
    type Error;

    fn drive(&mut self) -> Result<(), Self::Error>;

    fn shutdown(self) -> Result<(), Self::Error>;

    fn boxed_shutdown(self: Box<Self>) -> Result<(), Self::Error>;
}

impl<Block, B> Archive for ArchiveSystem<Block, B>
where
    Block: BlockT,
{
    type Error = ArchiveError;

    fn drive(&mut self) -> Result<(), Self::Error> {
        self.start_tx.send(())?;
        Ok(())
    }

    fn shutdown(self) -> Result<(), Self::Error> {
        self.kill_tx.send(())?;
        self.handle.join()?;
        Ok(())
    }

    fn boxed_shutdown(self: Box<Self>) -> Result<(), Self::Error> {
        self.kill_tx.send(())?;
        self.handle.join()?;
        Ok(())
    }
}
*/
