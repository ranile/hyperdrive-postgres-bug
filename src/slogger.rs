//! Socket Logger
//!
//! Intercepts all reads/writes from a [`worker::Socket`] and logs them

use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};
use std::future::Ready;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_postgres::tls::{ChannelBinding, TlsConnect, TlsStream};
use worker::postgres_tls::{PassthroughTls, PassthroughTlsError};
use worker::Socket;

pub struct LoggingSocket<S> {
    inner: S,
}

impl<S> LoggingSocket<S> {
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
}

fn dump(dir: &str, bytes: &[u8]) {
    let hex: String = bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ");
    let ascii: String = bytes
        .iter()
        .map(|&b| {
            if (0x20..0x7f).contains(&b) {
                b as char
            } else {
                '.'
            }
        })
        .collect();
    tracing::trace!(target: "wire", dir, len = bytes.len(), %hex, %ascii);
}

impl<S: AsyncRead + Unpin> AsyncRead for LoggingSocket<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let before = buf.filled().len();
        let res = Pin::new(&mut self.inner).poll_read(cx, buf);
        match &res {
            Poll::Ready(Ok(())) => {
                let new = &buf.filled()[before..];
                if new.is_empty() {
                    tracing::trace!(target: "wire", dir = "recv", "EOF");
                } else {
                    dump("recv", new);
                }
            }
            Poll::Ready(Err(e)) => tracing::trace!(target: "wire", dir = "recv", error = %e),
            Poll::Pending => {}
        }
        res
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for LoggingSocket<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let res = Pin::new(&mut self.inner).poll_write(cx, buf);
        match &res {
            Poll::Ready(Ok(n)) => dump("send", &buf[..*n]),
            Poll::Ready(Err(e)) => tracing::trace!(target: "wire", dir = "send", error = %e),
            Poll::Pending => {}
        }
        res
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let res = Pin::new(&mut self.inner).poll_flush(cx);
        if res.is_ready() {
            tracing::trace!(target: "wire", dir = "send", "flush");
        }
        res
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let res = Pin::new(&mut self.inner).poll_shutdown(cx);
        if res.is_ready() {
            tracing::trace!(target: "wire", dir = "send", "shutdown");
        }
        res
    }
}

impl TlsConnect<LoggingSocket<Socket>> for PassthroughTls {
    type Stream = LoggingSocket<Socket>;
    type Error = PassthroughTlsError;
    type Future = Ready<Result<LoggingSocket<Socket>, PassthroughTlsError>>;

    fn connect(self, s: Self::Stream) -> Self::Future {
        let tls = s.inner.start_tls();
        let socket = LoggingSocket { inner: tls };
        core::future::ready(Ok(socket))
    }
}

impl TlsStream for LoggingSocket<Socket> {
    fn channel_binding(&self) -> ChannelBinding {
        ChannelBinding::none()
    }
}
