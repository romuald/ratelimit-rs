
pub struct MockTcpStream {
    read_data: Vec<u8>,
    write_data: Vec<u8>,
}
use async_std::io::{Read, Write};
use std::cmp::min;
use std::pin::Pin;
use futures::io::Error;
use futures::task::{Context, Poll};

impl Default for MockTcpStream {
    fn default() -> Self {
        Self { read_data: vec![], write_data: vec![] }
    }
}

impl MockTcpStream {
    pub fn from_rdata(data: String) -> MockTcpStream {
        MockTcpStream { read_data: data.as_bytes().into(), write_data: vec![] }
    }

    // Returns the data that was written, as string, clearing it
    pub fn get_wdata(&mut self) -> String {
        let ret = std::str::from_utf8(&self.write_data).unwrap().to_string();
        self.write_data.clear();
        ret
    }
}

impl Read for MockTcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        _: &mut Context,
        buf: &mut [u8],
    ) -> Poll<Result<usize, Error>> {
        let size: usize = min(self.read_data.len(), buf.len());
        buf[..size].copy_from_slice(&self.read_data[..size]);
        Poll::Ready(Ok(size))
    }
}

impl Write for MockTcpStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _: &mut Context,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        self.write_data = Vec::from(buf);

        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}

impl Unpin for MockTcpStream {}
