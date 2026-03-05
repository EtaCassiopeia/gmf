use std::net::SocketAddr;

pub struct ServerConfig {
    pub addr: SocketAddr,
    pub max_connections: usize,
    pub num_cores: Option<usize>,
}

impl ServerConfig {
    pub fn effective_cores(&self) -> usize {
        self.num_cores.unwrap_or_else(num_cpus::get_physical)
    }
}
