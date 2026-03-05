fn main() {
    tonic_prost_build::configure()
        .compile_protos(&["proto/helloworld/helloworld.proto"], &["proto"])
        .expect("failed to compile protos");
}
