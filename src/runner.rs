use std::{
    io,
    io::Write,
    process::Stdio,
    time::{Duration, SystemTime},
};

use crossbeam::thread;
use execute::shell;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RunnerError {
    #[error("Error running command {0}:\n {1}")]
    RuntimeError(String, String),
}

pub struct CommandInfo {
    pub time: Duration,
    pub command: String,
    pub stdout: String,
    pub stderr: String,
}

impl CommandInfo {
    pub fn new(command: String, time: Duration, stdout: String, stderr: String) -> Self {
        Self {
            time,
            command,
            stdout,
            stderr,
        }
    }
}

pub fn run(command: String) -> Result<CommandInfo, RunnerError> {
    let start = SystemTime::now();

    let mut binding = shell(&command);
    binding.stdout(Stdio::piped());
    let cmd = binding.stderr(Stdio::piped());

    let mut child = cmd.spawn().unwrap();
    let mut child_stdout = child.stdout.take().expect("logic error getting stdout");
    let mut child_stderr = child.stderr.take().expect("logic error getting stderr");

    let (stdout, stderr) = thread::scope(|s| {
        let stdout_thread = s.spawn(|_| -> Vec<u8> {
            let stdout = io::stdout();
            let mut stdout = stdout.lock();
            let mut stdout_log = Vec::<u8>::new();
            let mut tee = TeeWriter::new(&mut stdout, &mut stdout_log);
            io::copy(&mut child_stdout, &mut tee).unwrap();
            stdout_log
        });
        let stderr_thread = s.spawn(|_| -> Vec<u8> {
            let stderr = io::stderr();
            let mut stderr = stderr.lock();
            let mut stderr_log = Vec::<u8>::new();
            let mut tee = TeeWriter::new(&mut stderr, &mut stderr_log);

            io::copy(&mut child_stderr, &mut tee).unwrap();
            stderr_log
        });

        let status = child.wait().expect("child wasn't running");

        let stdout_log = stdout_thread.join().expect("stdout thread panicked");
        let stderr_log = stderr_thread.join().expect("stderr thread panicked");

        (stdout_log, stderr_log)
    })
    .expect("stdout/stderr thread panicked");
    let stdout = String::from_utf8(stdout).unwrap();
    let stderr = String::from_utf8(stderr).unwrap();

    let info = CommandInfo::new(
        command.to_owned(),
        start.elapsed().unwrap_or_default(),
        stdout,
        stderr,
    );

    return Ok(info);
}

struct TeeWriter<'a, W0: Write, W1: Write> {
    w0: &'a mut W0,
    w1: &'a mut W1,
}

impl<'a, W0: Write, W1: Write> TeeWriter<'a, W0, W1> {
    fn new(w0: &'a mut W0, w1: &'a mut W1) -> Self {
        Self { w0, w1 }
    }
}

impl<'a, W0: Write, W1: Write> Write for TeeWriter<'a, W0, W1> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // We have to use write_all() otherwise what happens if different
        // amounts are written?
        self.w0.write_all(buf)?;
        self.w1.write_all(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.w0.flush()?;
        self.w1.flush()?;
        Ok(())
    }
}
