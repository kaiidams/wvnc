use crate::error::Error;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use std::thread_local;

pub enum Event {
    Keyboard(bool, u32),
    Pointer(u8, u16, u16),
    Clipboard(),
    FrameBufferUpdate(u8, u16, u16),
}

#[derive(Default)]
struct EventQueue {
    events: VecDeque<Event>,
    waker: Option<Waker>,
}

pub struct EventFuture {}

impl Future for EventFuture {
    type Output = Result<Event, Error>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        EVENT_QUEUE.with_borrow_mut(|queue| match queue.events.pop_front() {
            Some(event) => Poll::Ready(Ok(event)),
            None => {
                let waker = cx.waker().clone();
                queue.waker = Some(waker);
                Poll::Pending
            }
        })
    }
}

thread_local! {
    static EVENT_QUEUE: RefCell<EventQueue> = panic!();
}

pub fn init() {
    EVENT_QUEUE.set(EventQueue::default());
}

pub fn post(event: Event) {
    EVENT_QUEUE.with_borrow_mut(|queue| {
        queue.events.push_back(event);
        if let Some(waker) = &queue.waker {
            waker.wake_by_ref();
        }
    });
}

pub fn get_next() -> EventFuture {
    EventFuture {}
}
