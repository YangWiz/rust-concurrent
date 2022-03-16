//! Thread pool that joins all thread when dropped.

#![allow(clippy::mutex_atomic)]

// NOTE: Crossbeam channels are MPMC, which means that you don't need to wrap the receiver in
// Arc<Mutex<..>>. Just clone the receiver and give it to each worker thread.
use crossbeam_channel::{unbounded, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Job(Box<dyn FnOnce() + Send + 'static>);

#[derive(Debug)]
struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Drop for Worker {
    /// When dropped, the thread's `JoinHandle` must be `join`ed.  If the worker panics, then this
    /// function should panic too.  NOTE: that the thread is detached if not `join`ed explicitly.
    fn drop(&mut self) {
        let _thread = self
            .thread
            .take()
            .expect("failed to take the JoinHandle!")
            .join().expect("panic!");
    }
}

/// Internal data structure for tracking the current job status. This is shared by the worker
/// closures via `Arc` so that the workers can report to the pool that it started/finished a job.
#[derive(Debug, Default)]
struct ThreadPoolInner {
    job_count: Mutex<usize>,
    empty_condvar: Condvar,
}

impl ThreadPoolInner {
    /// Increment the job count.
    fn start_job(&self) {
        let mut job_counter = self.job_count.lock().unwrap();
        *job_counter += 1;
    }

    /// Decrement the job count.
    fn finish_job(&self) {
        let mut job_counter = self.job_count.lock().unwrap();
        *job_counter -= 1;
        self.empty_condvar.notify_one();
    }

    /// Wait until the job count becomes 0.
    ///
    /// NOTE: We can optimize this function by adding another field to `ThreadPoolInner`, but let's
    /// not care about that in this homework.
    fn wait_empty(&self) {
        let mut job_counter = self.job_count.lock().unwrap();

        while *job_counter != 0 {
            job_counter = self.empty_condvar.wait(job_counter).unwrap();
        }
    }
}

/// Thread pool.
#[derive(Debug)]
pub struct ThreadPool {
    workers: Vec<Worker>,
    job_sender: Option<Sender<Job>>,
    pool_inner: Arc<ThreadPoolInner>,
}

impl ThreadPool {
    /// Create a new ThreadPool with `size` threads. Panics if the size is 0.
    pub fn new(size: usize) -> Self {
        assert!(size > 0);
        let (sx, rx) = crossbeam_channel::unbounded::<Job>();
        let mut workers = Vec::new();
        for i in 0..size {
            // crossbeam channel is mpmc, so clone directly is ok.
            let rx = rx.clone();
            let handler = thread::spawn(move || {
                loop {
                    // blocked! drop sender.
                    // crossbeam-channel can detect close when all sender or all receiver are closed.
                    // close the sender.
                    match rx.recv() {
                        Ok(task) => {
                            task.0()
                        },
                        Err(_) => {
                            break;
                        }
                    };
                }
            });

            let worker = Worker {
                id: i,
                thread: Some(handler),
            };
            workers.push(worker);
        }

        let pool_inner = ThreadPoolInner {
            job_count: Mutex::new(0),
            empty_condvar: Condvar::new(),
        };

        ThreadPool {
            workers,
            job_sender: Some(sx),
            pool_inner: Arc::new(pool_inner),
        }
    }

    /// Execute a new job in the thread pool.
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let sender = self
            .job_sender
            .as_ref()
            .unwrap()
            .clone();
       
        // Arc can be sent to another thread.
        let status= self.pool_inner.clone();

        let job =  Job(Box::new(move || {
            // much more cleaner than before, wrap all the tasks in a single closure.
            status.start_job();
            f();
            status.finish_job();
        }));

        sender.send(job).unwrap();
    }

    /// Block the current thread until all jobs in the pool have been executed.  NOTE: This method
    /// has nothing to do with `JoinHandle::join`.
    pub fn join(&self) {
        // wait empty maybe.
        // wait_empty: return until the number of tasks become 0 => all tasks have been executed.
        self.pool_inner.clone().wait_empty();
    }
}

impl Drop for ThreadPool {
    /// When dropped, all worker threads' `JoinHandle` must be `join`ed. If the thread panicked,
    /// then this function should panic too.
    fn drop(&mut self) {
        // wait until all the tasks have been finished.
        self.join();

        // drop the sender => drop the recv => drop the worker => drop everything. 
        drop(self
            .job_sender
            .take()
            .unwrap());
    }
}
