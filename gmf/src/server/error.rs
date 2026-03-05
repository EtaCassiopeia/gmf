use std::io;
use std::net::SocketAddr;

#[derive(Debug, thiserror::Error)]
pub enum GmfError {
    #[error("bind failed on {addr}")]
    Bind { addr: SocketAddr, source: io::Error },

    #[error("executor spawn failed on CPU {cpu}")]
    SpawnExecutor { cpu: usize, source: io::Error },

    #[error("connection error")]
    Connection(#[source] io::Error),

    #[error("hyper error")]
    Hyper(#[from] hyper::Error),

    #[error("IO error")]
    Io(#[from] io::Error),
}
