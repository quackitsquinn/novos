use std::io::{self, Read, Write};

/// A wrapper around a Read + ?Write that copies all data read from it to another Write.
pub struct CopiedReadWrite<T, Rd, Wd = Rd>
where
    T: Read,
    Rd: Write,
    Wd: Write,
{
    pub read_dump: Rd,
    pub write_dump: Wd,
    pub inner: T,
}

impl<T, Rd, Wd> Read for CopiedReadWrite<T, Rd, Wd>
where
    T: Read,
    Rd: Write,
    Wd: Write,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let res = self.inner.read(buf)?;
        if res == 0 {
            return Ok(0);
        }
        self.read_dump.write_all(&buf[..res])?;
        Ok(res)
    }
}

impl<T, Rd, Wd> Write for CopiedReadWrite<T, Rd, Wd>
where
    T: Read + Write,
    Rd: Write,
    Wd: Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_dump.write(buf)?;
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()?;
        self.read_dump.flush()?;
        self.write_dump.flush()?;
        Ok(())
    }
}
