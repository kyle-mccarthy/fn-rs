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

#[derive(Debug)]
pub struct Task {
    pub script: String,
    pub stdin: Vec<u8>,
    pub stdout: Option<Vec<u8>>,
    pub stderr: Option<Vec<u8>>,
    pub error: Option<Error>,
    //    pub content_type: String,
}

impl Task {
    pub fn new(script: String, stdin: Vec<u8>) -> Task {
        Task {
            script,
            stdin,
            stdout: None,
            stderr: None,
            error: None,
            //            content_type: String::from("text/plain"),
        }
    }
}

fn exec_script(task: &Task) -> Result<Output, Error> {
    let mut cmd = Command::new(task.script.as_str());

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

pub fn handle(script: &str, incoming: &str) -> Task {
    let mut task = Task::new(script.to_string(), incoming.as_bytes().to_vec());

    let output = exec_script(&task);

    make_response(&mut task, output);

    task
}
