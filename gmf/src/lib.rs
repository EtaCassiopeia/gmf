#[cfg(not(any(
    feature = "monoio-runtime",
    feature = "glommio-runtime",
    feature = "tokio-runtime"
)))]
compile_error!(
    "Enable at least one runtime feature: monoio-runtime, glommio-runtime, or tokio-runtime"
);

#[cfg(all(feature = "glommio-runtime", not(target_os = "linux")))]
compile_error!("glommio-runtime requires Linux");

pub mod server;
