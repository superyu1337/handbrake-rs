use crate::error::Error;
use crate::event::JobEvent;
use async_stream::stream;
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::{mpsc, Mutex};

/// Represents a running HandBrake job, providing access to events and process control.
#[derive(Debug)]
pub struct JobHandle {
    /// The handle to the child process, shared for control operations.
    pub(crate) child: Arc<Mutex<Child>>,
    /// The receiver for job events from the background parsing task.
    pub(crate) event_rx: mpsc::Receiver<JobEvent>,
}

impl JobHandle {
    /// Attempts to gracefully shut down the HandBrake process.
    /// (Sends SIGINT on Unix, CTRL_C_EVENT on Windows).
    pub async fn cancel(&self) -> Result<(), Error> {
        // To be implemented in Chunk 7
        unimplemented!("Graceful cancellation is not yet implemented.");
    }

    /// Forcefully terminates the HandBrake process immediately.
    /// (Sends SIGKILL on Unix, TerminateProcess on Windows).
    pub async fn kill(&mut self) -> Result<(), Error> {
        // To be implemented in Chunk 7
        unimplemented!("Killing the process is not yet implemented.");
    }

    /// Returns an async stream of events from the running job.
    pub fn events(&mut self) -> Pin<Box<impl Stream<Item = JobEvent> + '_>> {
        let s = stream! {
            while let Some(event) = self.event_rx.recv().await {
                yield event;
            }
        };
        Box::pin(s)
    }
}
