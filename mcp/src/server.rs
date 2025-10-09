use std::collections::VecDeque;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::thread;

const NUM_WORKERS: usize = 4;

pub struct OutputSink(mpsc::Sender<String>);

impl OutputSink {
    pub fn send(&mut self, line: String) {
        self.0.send(line.trim().to_owned()).ok();
    }
}

struct WorkQueue<T> {
    queue: Mutex<VecDeque<T>>,
    condvar: Condvar,
    shutdown: AtomicBool,
}

impl<T> WorkQueue<T> {
    fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            condvar: Condvar::new(),
            shutdown: AtomicBool::new(false),
        }
    }

    fn send(&self, item: T) {
        let mut queue = self.queue.lock().unwrap();
        queue.push_back(item);
        self.condvar.notify_one();
    }

    fn recv(&self) -> Option<T> {
        let mut queue = self.queue.lock().unwrap();
        loop {
            if let Some(item) = queue.pop_front() {
                return Some(item);
            }

            if self.shutdown.load(Ordering::Relaxed) {
                return None;
            }

            queue = self.condvar.wait(queue).unwrap();
        }
    }

    fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
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
    let work_queue = Arc::new(WorkQueue::new());
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
