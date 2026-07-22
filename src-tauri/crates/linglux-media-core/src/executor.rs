use crate::TaskKind;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

type Job = Box<dyn FnOnce() + Send + 'static>;

#[derive(Clone)]
pub struct TaskExecutor {
    export_sender: Sender<Job>,
    media_sender: Sender<Job>,
}

impl TaskExecutor {
    pub fn new(export_workers: usize, media_workers: usize) -> Self {
        let export_sender = spawn_workers("linglux-export", export_workers.max(1));
        let media_sender = spawn_workers("linglux-media", media_workers.max(1));

        Self {
            export_sender,
            media_sender,
        }
    }

    pub fn submit(
        &self,
        kind: TaskKind,
        job: impl FnOnce() + Send + 'static,
    ) -> Result<(), String> {
        let sender = if kind == TaskKind::Export {
            &self.export_sender
        } else {
            &self.media_sender
        };

        sender
            .send(Box::new(job))
            .map_err(|_| "媒体任务队列已经停止。".to_string())
    }
}

fn spawn_workers(name: &str, count: usize) -> Sender<Job> {
    let (sender, receiver) = mpsc::channel::<Job>();
    let receiver = Arc::new(Mutex::new(receiver));

    for index in 0..count {
        let receiver = receiver.clone();
        let thread_name = format!("{name}-{index}");
        let _ = thread::Builder::new()
            .name(thread_name)
            .spawn(move || loop {
                let job = receiver
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .recv();

                match job {
                    Ok(job) => job(),
                    Err(_) => break,
                }
            });
    }

    sender
}
