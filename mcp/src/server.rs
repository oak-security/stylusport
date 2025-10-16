use std::collections::VecDeque;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread;

const NUM_WORKERS: usize = 4;

pub struct OutputSink(mpsc::Sender<String>);

impl OutputSink {
    pub fn send(&mut self, line: String) {
        self.0.send(line.trim().to_owned()).ok();
    }
}

pub struct WorkQueue<T> {
    inner: Mutex<Inner<T>>,
    condvar: Condvar,
    capacity: usize,
}

struct Inner<T> {
    queue: VecDeque<T>,
    shutdown: bool,
}

impl<T> WorkQueue<T> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "capacity must be > 0");

        Self {
            inner: Mutex::new(Inner {
                queue: VecDeque::with_capacity(capacity),
                shutdown: false,
            }),
            condvar: Condvar::new(),
            capacity,
        }
    }

    /// Block until there is capacity or shutdown occurs.
    /// If shutdown has been requested, this is a no-op.
    pub fn send(&self, item: T) {
        let mut inner = self.inner.lock().unwrap();

        while inner.queue.len() >= self.capacity && !inner.shutdown {
            inner = self.condvar.wait(inner).unwrap();
        }

        if inner.shutdown {
            return;
        }

        inner.queue.push_back(item);

        self.condvar.notify_one();
    }

    /// Pops an item, blocking if empty. After shutdown and drain, returns None.
    pub fn recv(&self) -> Option<T> {
        let mut inner = self.inner.lock().unwrap();

        loop {
            if let Some(item) = inner.queue.pop_front() {
                // Space available for a blocked sender.
                self.condvar.notify_one();
                return Some(item);
            }

            if inner.shutdown {
                return None;
            }

            inner = self.condvar.wait(inner).unwrap();
        }
    }

    /// Request shutdown: unblocks all waiters. Remaining items can still be received.
    pub fn shutdown(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.shutdown = true;
        self.condvar.notify_all();
    }
}

pub fn start<Msg>(
    parse_fn: impl Fn(&str) -> Option<Vec<Msg>>,
    work_fn: impl Fn(Msg, &mut OutputSink) + Send + Sync + 'static,
) -> io::Result<()>
where
    Msg: Send + 'static,
{
    let work_queue = Arc::new(WorkQueue::new(NUM_WORKERS * 2));
    let (result_tx, result_rx) = mpsc::channel();
    let work_fn = Arc::new(work_fn);

    let mut worker_handles = vec![];

    for _ in 0..NUM_WORKERS {
        let work_queue = Arc::clone(&work_queue);
        let result_tx = result_tx.clone();
        let work_fn = Arc::clone(&work_fn);

        let handle = thread::spawn(move || {
            while let Some(line) = work_queue.recv() {
                work_fn(line, &mut OutputSink(result_tx.clone()));
            }
        });
        worker_handles.push(handle);
    }

    drop(result_tx);

    let result_handle = {
        let mut writer = BufWriter::new(io::stdout());
        thread::spawn(move || -> io::Result<()> {
            while let Ok(result) = result_rx.recv() {
                writeln!(writer, "{result}")?;
                writer.flush()?;
            }
            Ok(())
        })
    };

    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());

    let mut line = String::new();
    while reader.read_line(&mut line)? > 0 {
        if let Some(msgs) = parse_fn(line.trim()) {
            for msg in msgs {
                work_queue.send(msg);
            }
        }
        line.clear();
    }

    work_queue.shutdown();
    for handle in worker_handles {
        let _ = handle.join();
    }
    let _ = result_handle.join();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{mpsc, Arc, Barrier};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn basic_send_recv() {
        let wq = WorkQueue::new(10);
        wq.send(42);
        assert_eq!(wq.recv(), Some(42));
    }

    #[test]
    fn fifo_ordering() {
        let wq = WorkQueue::new(8);
        for i in 0..5 {
            wq.send(i);
        }
        for i in 0..5 {
            assert_eq!(wq.recv(), Some(i));
        }
    }

    #[test]
    fn recv_waits_until_item_available() {
        let wq = Arc::new(WorkQueue::new(4));
        let start = Arc::new(Barrier::new(2));

        let wq_c = wq.clone();
        let start_c = start.clone();
        let h = thread::spawn(move || {
            // Ensure both threads are ready, then attempt to recv (will block until send happens).
            start_c.wait();
            wq_c.recv()
        });

        // Let the receiver start and block.on recv
        start.wait();

        // check receiver unblocks on send
        wq.send(99);
        assert_eq!(h.join().unwrap(), Some(99));
    }

    #[test]
    fn capacity_one_blocks_sender_until_recv() {
        let wq = Arc::new(WorkQueue::new(1));
        wq.send(1); // fill

        let (done_tx, done_rx) = mpsc::channel();
        let sync = Arc::new(Barrier::new(2));

        // Sender that should block trying to enqueue '2' until we recv the '1'.
        let wq_s = wq.clone();
        let sync_s = sync.clone();
        let h = thread::spawn(move || {
            sync_s.wait(); // start together
            wq_s.send(2); // should block until space
            let _ = done_tx.send(());
        });

        // Release the sender to attempt send(2).
        sync.wait();

        // Before making space, the sender should not have completed.
        assert!(done_rx.try_recv().is_err(), "sender should be blocked");

        // Make space; this should unblock the sender.
        assert_eq!(wq.recv(), Some(1));

        // Now the sender should complete; fail fast if it doesn't.
        match done_rx.recv_timeout(Duration::from_secs(1)) {
            Ok(()) => {}
            Err(mpsc::RecvTimeoutError::Timeout) => panic!("sender never unblocked within 1s"),
            Err(e) => panic!("recv failed: {e:?}"),
        }

        assert_eq!(wq.recv(), Some(2));
        h.join().unwrap();
    }

    #[test]
    fn shutdown_unblocks_recv_and_is_idempotent() {
        let wq = Arc::new(WorkQueue::<u32>::new(4));
        let start = Arc::new(Barrier::new(2));

        let wq_c = wq.clone();
        let start_c = start.clone();
        let recv_handle = thread::spawn(move || {
            start_c.wait();
            wq_c.recv()
        });

        // Ensure receiver is ready to block.
        start.wait();

        // Trigger shutdown.
        wq.shutdown();

        assert_eq!(recv_handle.join().unwrap(), None);
        assert_eq!(wq.recv(), None);
    }

    #[test]
    fn shutdown_drains_remaining() {
        let wq = WorkQueue::new(10);
        wq.send(1);
        wq.send(2);
        wq.shutdown();

        assert_eq!(wq.recv(), Some(1));
        assert_eq!(wq.recv(), Some(2));
        assert_eq!(wq.recv(), None);
    }
}
