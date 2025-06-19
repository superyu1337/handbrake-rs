use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::sync::{Mutex, OnceLock};
use std::thread;

/// Global mock registry
static MOCK_REGISTRY: OnceLock<Mutex<HashMap<thread::ThreadId, HashMap<CommandKey, MockResult>>>> =
    OnceLock::new();

/// Mock result that will be returned by the command
#[derive(Debug, Clone)]
pub struct MockResult {
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl MockResult {
    pub fn success() -> Self {
        Self {
            exit_code: 0,
            stdout: Vec::new(),
            stderr: Vec::new(),
        }
    }

    pub fn failure(exit_code: i32) -> Self {
        Self {
            exit_code,
            stdout: Vec::new(),
            stderr: Vec::new(),
        }
    }

    pub fn with_stdout<T: Into<Vec<u8>>>(mut self, stdout: T) -> Self {
        self.stdout = stdout.into();
        self
    }

    pub fn with_stderr<T: Into<Vec<u8>>>(mut self, stderr: T) -> Self {
        self.stderr = stderr.into();
        self
    }
}

/// Key used to match commands in the mock registry
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CommandKey {
    program: OsString,
    args: Vec<OsString>,
}

/// Builder for setting up mock expectations
pub struct MockCommandExpect {
    program: OsString,
    args: Vec<OsString>,
}

impl MockCommandExpect {
    fn new(program: OsString) -> Self {
        Self {
            program,
            args: Vec::new(),
        }
    }

    pub fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.args = args
            .into_iter()
            .map(|s| s.as_ref().to_os_string())
            .collect();
        self
    }

    pub fn with_arg<S: AsRef<OsStr>>(mut self, arg: S) -> Self {
        self.args.push(arg.as_ref().to_os_string());
        self
    }

    pub fn returns(self, result: MockResult) {
        let key = CommandKey {
            program: self.program,
            args: self.args,
        };

        let registry = Self::get_global_registry();
        if let Ok(mut registry) = registry.lock() {
            let thread_id = thread::current().id();
            let thread_registry = registry.entry(thread_id).or_insert_with(HashMap::new);
            thread_registry.insert(key, result);
        } else {
            panic!("failed to lock the mutex");
        }
    }

    fn get_global_registry() -> &'static Mutex<HashMap<thread::ThreadId, HashMap<CommandKey, MockResult>>> {
        MOCK_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
    }

    /// Start building a mock expectation
    pub fn when<S: AsRef<OsStr>>(program: S) -> MockCommandExpect {
        MockCommandExpect::new(program.as_ref().to_os_string())
    }

    /// Clear all mock expectations (useful for test cleanup)
    pub fn clear_all_expectations() {
        let registry = Self::get_global_registry();
        if let Ok(mut registry) = registry.try_lock() {
             let thread_id = thread::current().id();
            registry.remove(&thread_id);
        } else {
            // If we can't get the lock, create a new registry
            // This handles the case where the mutex is poisoned
            let _ = MOCK_REGISTRY.set(Mutex::new(HashMap::new()));
        }
    }
}

/// Mock implementation of tokio::process::Command
pub struct MockCommand {
    program: OsString,
    args: Vec<OsString>,
}

impl MockCommand {
    /// Creates a new MockCommand
    pub fn new<S: AsRef<OsStr>>(program: S) -> Self {
        Self {
            program: program.as_ref().to_os_string(),
            args: Vec::new(),
        }
    }

    /// Add an argument
    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.args.push(arg.as_ref().to_os_string());
        self
    }

    /// Add multiple arguments
    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for arg in args {
            self.args.push(arg.as_ref().to_os_string());
        }
        self
    }

    /// Execute command and capture output
    pub async fn output(&mut self) -> std::io::Result<std::process::Output> {
        let key = CommandKey {
            program: self.program.clone(),
            args: self.args.clone(),
        };

        let registry = MockCommandExpect::get_global_registry();
        let mock_result = if let Ok(registry) = registry.lock() {
            let thread_id = thread::current().id();
            registry.get(&thread_id).and_then(|thread_registry| thread_registry.get(&key)).cloned()
        } else {
            // Handle poisoned mutex
            None
        };

        let mock_result = mock_result.unwrap_or_else(|| {
            panic!(
                "No mock result configured for command: {:?} with args: {:?}",
                self.program, self.args
            )
        });

        // Create a dummy ExitStatus - in practice you might need a more sophisticated approach
        let status = if mock_result.exit_code == 0 {
            std::process::Command::new("true").status().unwrap()
        } else {
            std::process::Command::new("false").status().unwrap()
        };

        Ok(std::process::Output {
            status,
            stdout: mock_result.stdout,
            stderr: mock_result.stderr,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_mock_command() {
        // Clear any existing expectations
        MockCommandExpect::clear_all_expectations();

        // Set up expectations
        MockCommandExpect::when("git")
            .with_args(["status", "--porcelain"])
            .returns(MockResult::success().with_stdout(b"M  src/lib.rs"));

        // Use the command
        let mut cmd = MockCommand::new("git");
        cmd.args(["status", "--porcelain"]);

        let output = cmd.output().await.unwrap();
        assert_eq!(output.stdout, b"M  src/lib.rs");
        assert!(output.status.success());
    }

    #[tokio::test]
    async fn test_failure_scenario() {
        // Clear any existing expectations
        MockCommandExpect::clear_all_expectations();

        MockCommandExpect::when("git")
            .with_arg("push")
            .returns(MockResult::failure(1).with_stderr(b"Permission denied"));

        let mut cmd = MockCommand::new("git");
        cmd.arg("push");

        let output = cmd.output().await.unwrap();
        assert_eq!(output.stderr, b"Permission denied");
        assert!(!output.status.success());
    }

    #[tokio::test]
    #[should_panic(expected = "No mock result configured")]
    async fn test_unmatched_command_panics() {
        // Clear any existing expectations
        MockCommandExpect::clear_all_expectations();

        let mut cmd = MockCommand::new("git");
        cmd.arg("unknown-command");

        let _ = cmd.output().await;
    }

    #[tokio::test]
    async fn test_multiple_calls_same_result() {
        // Clear any existing expectations
        MockCommandExpect::clear_all_expectations();

        MockCommandExpect::when("echo")
            .with_arg("hello")
            .returns(MockResult::success().with_stdout(b"hello\n"));

        // First call
        let mut cmd1 = MockCommand::new("echo");
        cmd1.arg("hello");
        let output1 = cmd1.output().await.unwrap();

        // Second call
        let mut cmd2 = MockCommand::new("echo");
        cmd2.arg("hello");
        let output2 = cmd2.output().await.unwrap();

        // Both should return the same result
        assert_eq!(output1.stdout, b"hello\n");
        assert_eq!(output2.stdout, b"hello\n");
        assert!(output1.status.success());
        assert!(output2.status.success());
    }
}
