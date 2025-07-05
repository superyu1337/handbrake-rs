use crate::error::Error;
use crate::event::JobEvent;
use async_stream::stream;
use futures::Stream;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::{mpsc, Mutex};

/// A handle to a running `HandBrakeCLI` job.
///
/// This struct provides two key functionalities:
/// 1.  An async stream of `JobEvent`s parsed from the process's output.
/// 2.  Control methods (`cancel`, `kill`) to manage the underlying process.
#[derive(Debug)]
pub struct JobHandle {
    /// The handle to the child process, shared for control operations.
    pub(crate) child: Arc<Mutex<Child>>,
    /// The receiver for job events from the background parsing task.
    pub(crate) event_rx: mpsc::Receiver<JobEvent>,
}

impl JobHandle {
    /// Attempts to gracefully shut down the `HandBrakeCLI` process.
    ///
    /// This is the preferred method for stopping a job.
    /// - On Unix, it sends a `SIGINT` signal.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if the control signal could not be sent, for example if the
    /// process has already terminated.
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

    /// Forcefully terminates the `HandBrakeCLI` process immediately.
    ///
    /// This should be used as a last resort, as it may leave orphaned files or
    /// result in a corrupt output file.
    /// - On Unix, it sends a `SIGKILL` signal.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if the process could not be killed, for example if it has
    /// already terminated.
    pub async fn kill(&self) -> Result<(), Error> {
        let mut child = self.child.lock().await;
        child.kill().await.map_err(|e| Error::ControlFailed {
            action: "kill",
            source: e,
        })
    }

    /// Returns an async stream of `JobEvent`s from the running job.
    ///
    /// This is the primary way to monitor the state of an encoding job.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use handbrake_rs::{HandBrake, JobEvent, InputSource, OutputDestination};
    /// # use futures::StreamExt;
    /// # use std::path::PathBuf;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let hb = HandBrake::new().await?;
    /// # let mut job_handle = hb.job(InputSource::File(PathBuf::from("")),
    ///                               OutputDestination::File(PathBuf::from(""))).start()?;
    /// while let Some(event) = job_handle.events().next().await {
    ///     match event {
    ///         JobEvent::Progress(p) => println!("Progress: {}%", p.percentage),
    ///         JobEvent::Done(_) => break,
    ///         _ => {}
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn events(&mut self) -> Pin<Box<impl Stream<Item = JobEvent> + '_>> {
        let s = stream! {
            while let Some(event) = self.event_rx.recv().await {
                yield event;
            }
        };
        Box::pin(s)
    }
}
