use crate::config::FunctionConfig;
use snafu::{Backtrace, OptionExt, ResultExt, Snafu};
use std::io::Write;
use std::process::{Command, Output, Stdio};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("IO Error {}", source))]
    IOError {
        source: std::io::Error,
        context: &'static str,
        backtrace: Backtrace,
    },

    StdinError {
        context: &'static str,
        backtrace: Backtrace,
    },
}

/// The task contains information about what was passed to the function and what the function responded with.
#[derive(Debug)]
pub struct Task<'a> {
    /// The location of the "function" - an executable that will be invoked
    pub config: &'a FunctionConfig,

    /// Data passed to the function
    pub stdin: Vec<u8>,

    /// Data returned by the function
    pub stdout: Option<Vec<u8>>,

    /// Information about an error that occurred during the functions runtime
    pub stderr: Option<Vec<u8>>,

    /// Error that occurred while invoking the function
    pub error: Option<Error>,
}

impl<'a> Task<'a> {
    pub fn new(config: &'a FunctionConfig, stdin: Vec<u8>) -> Task {
        Task {
            config,
            stdin,
            stdout: None,
            stderr: None,
            error: None,
        }
    }
}

fn exec_script(task: &Task) -> Result<Output, Error> {
    // accept either an executable OR a lang + script
    let mut cmd = match &task.config.cmd {
        Some(lang) => {
            let mut cmd = Command::new(lang);
            cmd.arg(task.config.handler.as_str());
            cmd
        }
        _ => Command::new(task.config.handler.as_str()),
    };

    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context(IOError {
            context: "Failed to spawn child function",
        })?;

    {
        let stdin = child.stdin.as_mut().context(StdinError {
            context: "Failed to open stdio",
        })?;

        stdin.write_all(&task.stdin).context(IOError {
            context: "Failed to write data to child",
        })?;
    }

    child.wait_with_output().context(IOError {
        context: "Failed while waiting for output",
    })
}

fn make_response(task: &mut Task, raw: Result<Output, Error>) {
    match raw {
        Ok(out) => {
            task.stderr = Some(out.stderr);
            task.stdout = Some(out.stdout);
        }
        Err(err) => task.error = Some(err),
    }
}

/// Returns a task which contains information about the task including the output
///
/// # Arguments
/// * `config` - Configuration defining the behavior of the function
/// * `incoming` - The incoming data passed to the executable - serialized FunctionPayload
///
/// If the stdout can be deserialized to the FunctionResponse (passed in via the FunctionPayload)
/// we will attempt to set headers and send the body. This allows for custom content types to be sent.
///
pub fn handle<'a>(config: &'a FunctionConfig, incoming: &str) -> Task<'a> {
    let mut task = Task::new(config, incoming.as_bytes().to_vec());

    let output = exec_script(&task);

    make_response(&mut task, output);

    // stdout and err will always at least be an empty array, if it is an empty array covert it to None

    if let Some(stdout) = &task.stdout {
        if stdout.len() == 0 {
            task.stdout = None;
        }
    }

    if let Some(stderr) = &task.stderr {
        if stderr.len() == 0 {
            task.stderr = None;
        }
    }

    dbg!(&task);

    task
}
