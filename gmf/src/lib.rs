#[cfg(not(target_os = "linux"))]
compile_error!("Only linux is supported!");

pub mod server;
