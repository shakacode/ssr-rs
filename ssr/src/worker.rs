use std::{fmt, io, net::SocketAddr, path::PathBuf, process::Stdio};

use tokio::{
    net::TcpStream,
    process::{Child, Command},
    time,
};
use uuid::Uuid;

use crate::{error::InitializationError, JsWorkerLog};

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub(crate) struct Port(u16);

impl Port {
    pub fn new(port: u16) -> Self {
        Self(port)
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }

    pub fn to_socket_addr(&self) -> SocketAddr {
        format!("127.0.0.1:{}", self.0)
            .parse::<SocketAddr>()
            .expect("Unable to build socket address for js worker")
    }
}

impl From<SocketAddr> for Port {
    fn from(addr: SocketAddr) -> Port {
        Port(addr.port())
    }
}

struct Process;

impl Process {
    #[cfg(unix)]
    pub const SHELL: &'static str = "/bin/sh";

    #[cfg(windows)]
    pub const SHELL: &'static str = "cmd";

    #[cfg(unix)]
    pub fn cmd(cmd: &str) -> Vec<&str> {
        vec!["-c", &cmd]
    }

    #[cfg(windows)]
    pub fn cmd(cmd: &str) -> Vec<&str> {
        vec!["/c", &cmd]
    }

    pub fn spawn(
        port: &Port,
        js_worker: &PathBuf,
        js_worker_log: &JsWorkerLog,
        global_js_renderer: &Option<PathBuf>,
    ) -> Result<Child, io::Error> {
        let mut cmd = Command::new(Process::SHELL);

        cmd.args(Process::cmd(&format!(
            "node {}",
            js_worker.as_path().display()
        )));
        cmd.env("PORT", port.to_string());
        cmd.env("LOG", js_worker_log.to_str());

        if let Some(global_renderer) = global_js_renderer {
            cmd.env(
                "GLOBAL_RENDERER",
                global_renderer.as_path().display().to_string(),
            );
        }

        cmd.stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
    }
}

pub(crate) struct Worker {
    addr: SocketAddr,
    process: Child,
}

impl Worker {
    pub async fn new(
        port: &Port,
        js_worker: &PathBuf,
        js_worker_log: &JsWorkerLog,
        global_js_renderer: &Option<PathBuf>,
    ) -> Result<Self, InitializationError> {
        let process = Process::spawn(port, js_worker, js_worker_log, global_js_renderer)?;

        Ok(Self {
            addr: port.to_socket_addr(),
            process,
        })
    }

    pub fn display(&self) -> String {
        format!(
            "[RS] Worker [id: {} port: {}]",
            self.process.id(),
            self.addr.port()
        )
    }

    pub fn display_with_request_id(&self, request_id: &Uuid) -> String {
        format!(
            "[RS] Worker [id: {} port: {} request: {}]",
            self.process.id(),
            self.addr.port(),
            request_id
        )
    }

    pub async fn connect(&self) -> Result<TcpStream, io::Error> {
        let max_attempts = 5;
        let mut attempt = 1;
        loop {
            match attempt {
                1 => trace!("{worker}: Connecting to the js worker", worker = self),
                _ => {
                    let delay = attempt * 3;
                    trace!(
                        "{worker}: Trying to reconnect to the js worker. Attempt: {attempt}. Delay: {delay}ms",
                        worker = self,
                        attempt = attempt,
                        delay = delay
                    );
                    time::delay_for(std::time::Duration::from_millis(delay)).await
                }
            }
            match TcpStream::connect(self.addr).await {
                Ok(stream) => {
                    trace!("{worker}: Connected to the js worker", worker = self);
                    return Ok(stream);
                }
                Err(err) => match err.kind() {
                    io::ErrorKind::ConnectionRefused => {
                        if attempt == max_attempts {
                            trace!(
                                "{worker}: Failed to connect to the js worker. Exiting.",
                                worker = self
                            );
                            return Err(err);
                        }
                        trace!(
                            "{worker}: Failed to connect to the js worker. Retrying.",
                            worker = self
                        );
                        attempt += 1;
                    }
                    _ => {
                        trace!(
                            "{worker}: Failed to connected to the js worker due to unexpected error: {err}",
                            worker = self,
                            err = err
                        );
                        return Err(err);
                    }
                },
            };
        }
    }
}

impl fmt::Display for Worker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}
