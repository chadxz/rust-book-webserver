use std::thread;
use std::sync::{mpsc, Arc, Mutex};

type Job = Box<dyn FnOnce() + Send + 'static>;

pub enum Message {
    NewJob(Job),
    Terminate,
}

pub struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    pub fn new(id: usize, channel: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        Worker { id, thread: Some(thread::spawn(move || loop {
            match channel.lock().unwrap().recv().unwrap() {
                Message::NewJob(job) => {
                    println!("Worker {} got a job; executing.", id);
                    job();
                },
                Message::Terminate => {
                    println!("Worker {} was told to terminate.", id);
                    break
                }
            }
        }))}
    }
}

pub struct ThreadPool {
    channel: mpsc::Sender<Message>,
    workers: Vec<Worker>
}

#[derive(Debug)]
pub struct PoolCreationError {}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        println!("Sending terminate message to all workers.");

        for _ in &self.workers {
            self.channel.send(Message::Terminate).unwrap();
        }

        println!("Shutting down all workers.");

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            };
        }
    }
}

impl ThreadPool {
    pub fn new(size: usize) -> Result<ThreadPool, PoolCreationError> {
        if size == 0 {
            return Err(PoolCreationError {});
        }

        let (channel, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        Ok(ThreadPool {
            channel,
            workers: (1..=size).map(|i| {
                let channel = Arc::clone(&receiver);
                Worker::new(i, channel)
            }).collect()
        })
    }

    pub fn execute<F>(&self, f: F)
        where F: FnOnce() + Send + 'static
    {
        self.channel.send(Message::NewJob(Box::new(f))).unwrap();
    }
}