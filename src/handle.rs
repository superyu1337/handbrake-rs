use crate::error::Error;
use crate::event::JobEvent;
use async_stream::stream;
use futures::Stream;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::{Mutex, mpsc};

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
        let child = self.child.lock().await;
        let pid = child.id().ok_or(Error::ControlFailed {
            action: "cancel",
            source: io::Error::new(io::ErrorKind::NotFound, "Process already exited"),
        })?;

        #[cfg(unix)]
        {
            use nix::sys::signal::{self, Signal};
            use nix::unistd::Pid;
            match signal::kill(Pid::from_raw(pid as i32), Signal::SIGINT) {
                Ok(()) => Ok(()),
                Err(e) => Err(Error::ControlFailed {
                    action: "cancel",
                    source: io::Error::new(
                        io::ErrorKind::Unsupported,
                        format!("Failed with errno: {e}"),
                    ),
                }),
            }
        }
    }

    /// Forcefully terminates the HandBrake process immediately.
    /// (Sends SIGKILL on Unix, TerminateProcess on Windows).
    pub async fn kill(&self) -> Result<(), Error> {
        let mut child = self.child.lock().await;
        child.kill().await.map_err(|e| Error::ControlFailed {
            action: "kill",
            source: e,
        })
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
