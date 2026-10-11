use std::cell::RefCell;
use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, RawWaker, RawWakerVTable, Waker};
use std::thread_local;

fn wake_waker(data: *const ()) {
    RUNTIME.with_borrow(|runtime| {
        log_trace!("wvnc::task::wake_waker {}", data.addr());
        runtime.wake.store(true, Ordering::Release);
    });
}

fn drop_waker(data: *const ()) {
    log_trace!("wvnc::task::drop_waker: {}", data.addr());
}

fn clone_waker(data: *const ()) -> RawWaker {
    let data = data.addr() + 1;
    let data = data as *const ();
    log_trace!("wvnc::task::clone_waker: {}", data.addr());
    RawWaker::new(data, &WAKER_VTABLE)
}

fn new_waker() -> Waker {
    log_trace!("wvnc::task::new_waker: {}", 0);
    unsafe { Waker::new(std::ptr::null(), &WAKER_VTABLE) }
}

const WAKER_VTABLE: RawWakerVTable =
    RawWakerVTable::new(clone_waker, wake_waker, wake_waker, drop_waker);

type Task = Pin<Box<dyn Future<Output = ()> + Send>>;

struct Runtime {
    wake: AtomicBool,
    tasks: Mutex<VecDeque<Task>>,
}

impl Runtime {
    fn new() -> Self {
        Self {
            wake: AtomicBool::new(true),
            tasks: Mutex::new(VecDeque::new()),
        }
    }

    fn spawn<F>(&self, task: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.tasks.lock().unwrap().push_back(Box::pin(task));
    }

    fn wake_and_run(&self) {
        self.wake.store(true, Ordering::Release);
        self.run();
    }

    fn run(&self) {
        log_trace!("run");
        let waker = new_waker();
        let mut cx = Context::from_waker(&waker);
        while self.wake.load(Ordering::Acquire) {
            log_trace!("run: wake=true, count={}", {
                self.tasks.lock().unwrap().len()
            });
            let mut scheduled = Vec::<Task>::new();

            self.wake.store(false, Ordering::Release);
            while let Some(mut task) = { self.tasks.lock().unwrap().pop_front() } {
                log_trace!("run task");
                if task.as_mut().poll(&mut cx).is_pending() {
                    scheduled.push(task);
                    log_trace!("pending");
                } else {
                    self.wake.store(true, Ordering::Release);
                    log_trace!("ready");
                }
            }

            let mut tasks = self.tasks.lock().unwrap();
            while let Some(task) = scheduled.pop() {
                tasks.push_back(task);
            }
        }
    }
}

thread_local! {
    static RUNTIME: RefCell<Runtime> = panic!();
}

pub(super) fn run() {
    RUNTIME.with_borrow(|runtime| {
        runtime.wake_and_run();
    });
}

pub fn init() {
    RUNTIME.set(Runtime::new());
}

pub(super) fn spawn<F>(task: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    RUNTIME.with_borrow(|runtime| {
        runtime.spawn(task);
    });
}
