use crate::peer;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::{Error, Read, Write};
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use std::thread_local;

#[derive(Default)]
struct Socket {
    buf: VecDeque<u8>,
    err: Option<Error>,
    waker: Option<Waker>,
    connected: bool,
}

thread_local! {
    static SOCKET: RefCell<Socket> = panic!();
}

pub fn init() {
    SOCKET.set(Socket::default());
}

pub fn socket_onopen(success: bool) {
    SOCKET.with_borrow_mut(|socket| {
        socket.connected = success;
        if let Some(waker) = &socket.waker {
            waker.wake_by_ref();
        }
    });
}

pub fn socket_onreceive(data: &[u8]) {
    SOCKET.with_borrow_mut(|socket| {
        if let Err(err) = socket.buf.write_all(data) {
            socket.err = Some(err);
        }
        if let Some(waker) = &socket.waker {
            waker.wake_by_ref();
        }
    });
}

pub struct ReadFuture<'a> {
    buf: &'a mut [u8],
}

impl<'a> Future for ReadFuture<'a> {
    type Output = Result<usize, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        SOCKET.with_borrow_mut(|socket| {
            if socket.buf.is_empty() {
                let waker = cx.waker();
                socket.waker = Some(waker.clone());
                Poll::Pending
            } else {
                let res = match socket.buf.read(self.get_mut().buf) {
                    Ok(read_bytes) => Ok(read_bytes),
                    Err(err) => Err(err),
                };
                Poll::Ready(res)
            }
        })
    }
}

pub struct ReadExactFuture<'a> {
    buf: &'a mut [u8],
}

impl<'a> Future for ReadExactFuture<'a> {
    type Output = Result<(), Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        SOCKET.with_borrow_mut(|socket| {
            let buf = &mut *self.get_mut().buf;
            if buf.len() > socket.buf.len() {
                let waker = cx.waker();
                socket.waker = Some(waker.clone());
                Poll::Pending
            } else {
                if let Err(err) = socket.buf.read_exact(buf) {
                    Poll::Ready(Err(err))
                } else {
                    Poll::Ready(Ok(()))
                }
            }
        })
    }
}

pub struct AsyncReader {
    _marker: PhantomData<Vec<()>>,
}

impl AsyncReader {
    pub fn read<'a>(&mut self, buf: &'a mut [u8]) -> ReadFuture<'a> {
        ReadFuture { buf }
    }

    pub fn read_exact<'a>(&mut self, buf: &'a mut [u8]) -> ReadExactFuture<'a> {
        ReadExactFuture { buf }
    }
}

pub struct WriteAllFuture<'a> {
    buf: &'a [u8],
}

impl<'a> Future for WriteAllFuture<'a> {
    type Output = Result<(), Error>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Self::Output> {
        let buf = self.buf;
        unsafe { peer::socket_send(buf.as_ptr(), buf.len()) };
        Poll::Ready(Ok(()))
    }
}

pub struct AsyncWriter {
    _marker: PhantomData<Vec<()>>,
}

impl AsyncWriter {
    pub fn write_all<'a>(&mut self, buf: &'a [u8]) -> WriteAllFuture<'a> {
        WriteAllFuture { buf }
    }
}

pub struct ConnectFuture {}

impl Future for ConnectFuture {
    type Output = (AsyncReader, AsyncWriter);

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        SOCKET.with_borrow_mut(|socket| {
            if socket.connected {
                let rd = AsyncReader {
                    _marker: PhantomData {},
                };
                let wr = AsyncWriter {
                    _marker: PhantomData {},
                };
                Poll::Ready((rd, wr))
            } else {
                let waker = cx.waker();
                socket.waker = Some(waker.clone());
                Poll::Pending
            }
        })
    }
}

pub fn connect() -> ConnectFuture {
    unsafe { peer::socket_open() };
    ConnectFuture {}
}
