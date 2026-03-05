pub mod config;
pub mod error;
pub mod gmf_server;
pub mod runtime;

#[cfg(feature = "glommio-runtime")]
mod hyper_io;

#[cfg(feature = "monoio-runtime")]
pub mod monoio_runtime;

#[cfg(feature = "glommio-runtime")]
pub mod glommio_runtime;

#[cfg(feature = "tokio-runtime")]
pub mod tokio_runtime;

// Type aliases for ergonomic usage
#[cfg(feature = "monoio-runtime")]
pub type MonoioServer = gmf_server::GmfServer<monoio_runtime::MonoioRuntime>;

#[cfg(feature = "glommio-runtime")]
pub type GlommioServer = gmf_server::GmfServer<glommio_runtime::GlommioRuntime>;

#[cfg(feature = "tokio-runtime")]
pub type TokioServer = gmf_server::GmfServer<tokio_runtime::TokioRuntime>;
