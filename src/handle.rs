use crate::error::Error;
use crate::event::JobEvent;
use async_stream::stream;
use futures::Stream;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::{mpsc, Mutex};

#[cfg(windows)]
use windows_sys;

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
    /// - On Windows, it sends a `CTRL_C_EVENT`.
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
            return match signal::kill(Pid::from_raw(pid as i32), Signal::SIGINT) {
                Ok(()) => Ok(()),
                Err(e) => Err(Error::ControlFailed {
                    action: "cancel",
                    source: io::Error::new(
                        io::ErrorKind::Unsupported,
                        format!("Failed with errno: {e}"),
                    ),
                }),
            };
        }

        #[cfg(windows)]
        {
            const CTRL_C_EVENT: u32 = 0;
            // Sending CTRL_C_EVENT to the process group ID (which is the same as the PID
            // when CREATE_NEW_PROCESS_GROUP is used) is the equivalent of pressing Ctrl+C.
            let result = unsafe {
                windows_sys::Win32::System::Console::GenerateConsoleCtrlEvent(CTRL_C_EVENT, pid)
            };

            if result == 0 {
                // A non-zero value indicates success.
                return Err(Error::ControlFailed {
                    action: "cancel",
                    source: io::Error::last_os_error(),
                });
            } else {
                return Ok(());
            }
        }

        #[cfg(not(any(unix, windows)))]
        {
            // Fallback for unsupported platforms
            Err(Error::ControlFailed {
                action: "cancel",
                source: io::Error::new(io::ErrorKind::Unsupported, "Cancel is not supported on this platform"),
            })
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
    /// # use handbrake::{HandBrake, JobEvent, InputSource, OutputDestination};
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