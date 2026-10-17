// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bound SDK stdio ingress without replacing its protocol handling.

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll, ready};
use tokio::io::{AsyncRead, ReadBuf};

const MAX_LINE_BYTES: usize = 16 * 1024 * 1024;

pub(super) struct BoundedLines<R> {
    reader: R,
    line_bytes: usize,
}

impl<R> BoundedLines<R> {
    pub(super) fn new(reader: R) -> Self {
        Self {
            reader,
            line_bytes: 0,
        }
    }

    fn charge(&mut self, bytes: &[u8]) -> io::Result<()> {
        for byte in bytes {
            self.line_bytes = if *byte == b'\n' {
                0
            } else {
                self.line_bytes.saturating_add(1)
            };
            if self.line_bytes > MAX_LINE_BYTES {
                return Err(line_limit());
            }
        }
        Ok(())
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for BoundedLines<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.line_bytes > MAX_LINE_BYTES {
            return Poll::Ready(Err(line_limit()));
        }
        if output.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        let mut bytes = [0_u8; 8192];
        let capacity = bytes.len().min(output.remaining());
        let mut chunk = ReadBuf::new(&mut bytes[..capacity]);
        ready!(Pin::new(&mut self.reader).poll_read(cx, &mut chunk))?;
        self.charge(chunk.filled())?;
        output.put_slice(chunk.filled());
        Poll::Ready(Ok(()))
    }
}

fn line_limit() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "stdio line exceeds the 16 MiB limit",
    )
}
