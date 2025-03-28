use std::io::{Read, Write};

pub trait ReadFrom {
    type Error: std::error::Error;

    fn read_ty<T>(&mut self) -> Result<T, Self::Error>
    where
        T: bytemuck::Pod;
}

impl<G> ReadFrom for G
where
    G: Read,
{
    type Error = std::io::Error;

    fn read_ty<T>(&mut self) -> Result<T, Self::Error>
    where
        T: bytemuck::Pod,
    {
        let mut data = vec![0; std::mem::size_of::<T>()];
        self.read_exact(&mut data)?;
        Ok(*bytemuck::from_bytes(&data))
    }
}
