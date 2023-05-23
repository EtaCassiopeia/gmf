use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hello_world::greeter_client::GreeterClient;
use hello_world::HelloRequest;
use tonic::transport::Channel;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

pub async fn run_client(host: &'static str) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the server
    let channel = Channel::from_static(host).connect().await?;

    // Create a new Greeter client
    let mut client = GreeterClient::new(channel);

    // Create a new HelloRequest
    let request = tonic::Request::new(HelloRequest {
        name: "World".into(), // for example
    });

    // Send the request
    let response = client.say_hello(request).await?;

    //println!("RESPONSE={:?}", response);

    Ok(())
}

fn benchmark_run_client(c: &mut Criterion) {
    let mut group = c.benchmark_group("gRPC client");

    group.bench_function("run_client", |b| {
        b.iter(|| {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            // If the function is async, we use block_on here to wait for the result.
            runtime.block_on(async { black_box(run_client("http://0.0.0.0:50051")).await });
        })
    });

    group.finish();
}

criterion_group!(benches, benchmark_run_client);
criterion_main!(benches);
