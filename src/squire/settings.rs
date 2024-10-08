use std::net::ToSocketAddrs;
use std::thread;

/// Represents the configuration parameters for SysMonk.
pub struct Config {
    /// Username for login.
    pub username: String,
    /// Password for login.
    pub password: String,

    /// Debug flag to enable debug level logging.
    pub debug: bool,
    /// Boolean flag to enable UTC timezone in logging. Defaults to local timezone.
    pub utc_logging: bool,
    /// Host IP address for the API.
    pub host: String,
    /// Port number for hosting the application.
    pub port: u16,
    /// Duration of a session in seconds.
    pub session_duration: i64,

    /// Number of worker threads to spin up the server.
    pub workers: usize,
    /// Maximum number of concurrent connections.
    pub max_connections: usize,
    /// List of websites (supports regex) to add to CORS configuration.
    pub websites: Vec<String>,
    /// List of services to monitor.
    pub services: Vec<String>,
    /// List of processes to monitor.
    pub processes: Vec<String>,
}

/// Returns the default value for debug flag.
pub fn default_debug() -> bool { false }

/// Returns the default value for UTC logging.
pub fn default_utc_logging() -> bool { false }

/// Returns the default host based on the local machine's IP address.
pub fn default_host() -> String {
    let hostname = "localhost";
    match (hostname, 0).to_socket_addrs() {
        Ok(mut addrs) => {
            if let Some(addr) = addrs.find(|a| a.is_ipv4()) {
                return addr.ip().to_string();
            }
        }
        Err(err) => {
            log::error!("Error resolving hostname: {}", err);
        }
    }
    "localhost".to_string()
}

/// Returns the default port (8000)
pub fn default_port() -> u16 { 8000 }

/// Returns the default session duration (900 seconds)
pub fn default_session_duration() -> i64 { 900 }

/// Returns the default number of worker threads (half of logical cores)
pub fn default_workers() -> usize {
    let logical_cores = thread::available_parallelism();
    match logical_cores {
        Ok(cores) => cores.get() / 2,
        Err(err) => {
            log::error!("{}", err);
            3
        }
    }
}

/// Returns the default maximum number of concurrent connections (3)
pub fn default_max_connections() -> usize { 3 }

/// Returns an empty vec
pub fn default_vec() -> Vec<String> { Vec::new() }
