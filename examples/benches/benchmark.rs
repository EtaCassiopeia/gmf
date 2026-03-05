use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hello_world::greeter_client::GreeterClient;
use hello_world::HelloRequest;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

pub async fn run_client(host: &'static str) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = GreeterClient::connect(host).await?;

    let request = tonic::Request::new(HelloRequest {
        name: "World".into(),
    });

    let _response = client.say_hello(request).await?;

    Ok(())
}

fn benchmark_run_client(c: &mut Criterion) {
    let mut group = c.benchmark_group("gRPC client");

    group.bench_function("run_client", |b| {
        b.iter(|| {
            let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
            let _ =
                runtime.block_on(async { black_box(run_client("http://0.0.0.0:50051")).await });
        })
    });

    group.finish();
}

criterion_group!(benches, benchmark_run_client);
criterion_main!(benches);
