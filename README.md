# Glommio Messaging Framework (GMF)

![Rust](https://github.com/EtaCassiopeia/gmf/actions/workflows/rust.yml/badge.svg)

The GMF library is a powerful and innovative framework developed for facilitating Remote Procedure Calls (RPCs) in Rust. It harnesses the efficiency and performance capabilities of the Glommio and Tonic libraries.

Our library elegantly blends Glommio's modern cooperative threading model with Tonic's gRPC-based protocol handling, providing an asynchronous, high-performance RPC tool tailored for modern microservices architecture.

GMF enables you to manage your RPCs in an efficient, lightweight, and powerful manner, while staying fully async from end to end. By implementing an executor based on Glommio's cooperative threading model, the library offers low-latency, high-throughput operation, and excellent resource utilization.

Please note that GMF is designed specifically for Linux, leveraging several Linux-specific features to provide its capabilities.

## Harnessing Glommio for High-Performance Networking

[Glommio](https://github.com/DataDog/glommio) is designed to provide high-performance networking capabilities by optimizing how network operations are handled. It uses techniques that revolve around the shared-nothing design, single-threaded execution, and bypassing the traditional network layers. Let's dive a bit into these techniques:

1. **Shared-nothing Design**: Both Glommio is designed around a shared-nothing architecture, which means each CPU core operates independently of others. This architecture minimizes the contention and overhead that can occur with thread synchronization, leading to greater performance and efficiency. Each core has its private memory, and there's no need for locks to access shared data. This approach is particularly beneficial for handling high concurrency levels and achieving high-throughput network services.

2. **Single-threaded Execution**: Each core runs a single thread and handles all the tasks scheduled to it in an asynchronous, non-blocking manner. This model further reduces the overhead associated with context switching between threads and enables efficient execution of numerous concurrent tasks.

3. **Efficient I/O Operations with io_uring**: Glommio is designed to take full advantage of io_uring, a powerful interface introduced in recent versions of the Linux kernel, which allows for highly efficient asynchronous I/O operations.

   Unlike traditional models where system calls are made for each I/O operation, leading to performance overhead due to context switches and interrupts, io_uring allows applications to enqueue and dequeue I/O operations using ring buffers in user space. These operations can be batched, significantly reducing the need for system calls and context switches.

   Glommio leverages io_uring to directly manage and optimize network and file I/O operations, bypassing traditional kernel network layers. This leads to lower latency, higher throughput, and more scalable I/O operations.

   This combination of io_uring and Glommio's asynchronous programming model is at the heart of GMF's high-performance design. With these features, GMF is able to provide superior I/O performance and scalability, making it an excellent choice for networking-intensive applications.

When it comes to handling gRPC services, these techniques allow the efficient processing of numerous RPC calls. With the asynchronous, event-driven model, incoming requests can be processed as they arrive, without blocking threads or needing to context switch. The result is a gRPC service that is not only high-performing but also resource-efficient, enabling better scaling for microservices architectures.

## System Requirements

**IMPORTANT:** This project is designed to work on Linux systems only.
Please ensure you're running a compatible Linux distribution before installing or using this package.

## Getting Started

To use `gmf`, add the following to your `Cargo.toml`:

```toml
[dependencies]
gmf = "1.0.0"
```

Then you can include it in your project:

```rust
use gmf::GmfServer;
use std::net::SocketAddr;
use tower::Service;
```

## Usage

After defining your service (like `MyGreeter`), you can serve it using `GmfServer` in just a few lines of code:

```rust
let greeter: MyGreeter = MyGreeter::default();
let tonic: GreeterServer<MyGreeter> = GreeterServer::new(greeter);

use hyper::service::service_fn;
let gmf = GmfServer::new(
    service_fn(move |req| {
        let mut tonic = tonic.clone();
        tonic.call(req)
    }),
    10240,  // max_connections
);

let addr: SocketAddr = "0.0.0.0:50051".parse().unwrap();
gmf.serve(addr).unwrap_or_else(|e| panic!("failed {e}"));
```

For a full working example, please refer to the [example source code](examples/src/helloworld-gmf/server.rs)

## Setting up Development Environment Using Nix

If the required tools are not installed on your machine, Nix provides an easy and consistent way to set up your development environment. This setup process assumes that you have Nix installed on your machine. If not, you can install it from the [official Nix website](https://nixos.org/download.html).

### 1. Define your Environment

Create a file named `shell.nix` in your project's root directory, and define the packages that you need for your project. For example, your `shell.nix` file might look like this:

```nix
{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = [
    pkgs.rustc
    pkgs.cargo
    pkgs.protobuf
  ];
}
```

This `shell.nix` file tells Nix that you need `rustc`, `cargo`, and `protobuf` for your project. You can add or remove packages based on your project's needs.

### 2. Enter the Nix Shell

You can now enter the development environment by running the following command in your project's root directory:

```bash
nix-shell
```

Nix will download and install the packages that you've defined in `shell.nix`, and then it will drop you into a shell where those packages are available.

### 3. Start Developing

You can now run your build commands as you normally would. The tools that you defined in `shell.nix` will be available in your path.

Please note that the Nix shell only affects your current terminal session. If you open a new terminal, you'll need to run `nix-shell` again to enter the development environment. Also, the packages installed by Nix do not affect your system's global state, and are isolated to the Nix shell.

Remember to always test your application in a environment as close to your production environment as possible to ensure that it works as expected.

## Testing and Running Examples in Non-Linux Environments

This project leverages specific Linux features and hence is not directly compatible with non-Linux environments like MacOS or Windows. However, we provide a way to build, test and run examples using Docker. Docker allows you to create a Linux environment inside a container.

### Prerequisites

- Install [Docker](https://docs.docker.com/get-docker/) on your machine.

#### Building the Docker Image

Before running the project, you will need to build a Docker image that includes the Rust toolchain and `protoc`. We've created a Dockerfile for this purpose and a script to simplify the build process.

##### Step 1: Build the Docker Image

1. Navigate to the project directory:

   ```bash
   cd /path/to/your/project
   ```

2. Make the build script executable:

   ```bash
   chmod +x build_docker_image.sh
   ```

3. Run the build script:

   ```bash
   ./build_docker_image.sh
   ```

   This script builds a Docker image named `rust-protoc:nightly-bookworm` by default. If the build is successful, you will see the following message:

   ```bash
   Docker Image rust-protoc:nightly-bookworm has been built successfully.
   ```

### Step 2: Running the Project with the Docker Image

After building the Docker image, you can use it to compile and run your Rust project.

1. Make sure that the run script is executable:

   ```bash
   chmod +x cargo-docker.sh
   ```

2. Run your Cargo command with the script. For example, to run the `cargo check` command:

   ```bash
   ./cargo-docker.sh check
   ```

   The script automatically handles the Docker container lifecycle, creating or starting the container as needed and running your Cargo command inside it.


##### Running Examples

To run the examples inside the Docker container, you can use the `cargo-docker.sh` script followed by `run --package examples --bin <example_name> --features <feature-name>`. Replace `example_name` and `feature-name` with the name of the example and the required feature you want to run:

```bash
./cargo-docker.sh run --package examples --bin helloworld-gmf-server --features hyper-warp
```

### Testing

To run the tests inside the Docker container, you can use the `cargo-docker.sh` script followed by `test`:

```bash
./cargo-docker.sh test
```

## Using `grpcurl` to Interact with the gRPC Service

`grpcurl` is a command-line tool that lets you interact with gRPC servers. It's like `curl`, but for gRPC!

Here's how you can use `grpcurl` to send requests to the gRPC service defined in this project:

1. **Install `grpcurl`**: If you haven't installed `grpcurl` yet, you can find installation instructions [here](https://github.com/fullstorydev/grpcurl#installation).

2. **Prepare your request**: For example, if you're calling the `SayHello` method of the `Greeter` service, your request might look like this:

    ```json
    {
      "name":"John"
    }
    ```

3. **Call the service**: You can use `grpcurl` to send this request to your running gRPC service:

    ```bash
    grpcurl -plaintext -d '{"name":"John"}' -proto examples/proto/helloworld/helloworld.proto 0.0.0.0:50051 helloworld.Greeter/SayHello
    ```

   In this command:
   - `-plaintext` tells `grpcurl` to use HTTP/2 plaintext instead of TLS.
   - `-d '{"name":"John"}'` is the data to send with your request.
   - `-proto examples/proto/helloworld/helloworld.proto` tells `grpcurl` where to find the protobuf service definition.
   - `0.0.0.0:50051` is the address and port where your gRPC service is running.
   - `helloworld.Greeter/SayHello` is the full name of the method you're calling.

Note: If the gRPC service is running on a Docker container, make sure the Docker container's ports are correctly mapped to the host's ports.

## Use remote development features in your IDE

IntelliJ IDEA supports remote development through a feature called "Remote Development". This feature requires the "Ultimate" edition of IntelliJ IDEA.

Here are the general steps to set up a remote development environment in IntelliJ IDEA:

1. **Configure a Remote SDK**:

   - Go to "File" > "Project Structure" > "SDKs" (on the left pane).
   - Click the "+" button on the top bar > select "Remote" or "Docker".
   - You'll then need to provide the details of your remote environment or Docker.

2. **Configure the Project SDK**:

   - In the same "Project Structure" window, click on "Project" (left pane).
   - Under "Project SDK", select the remote SDK you just configured.
   - Click "Apply".

3. **Configure the Run/Debug Configuration**:

   - Go to "Run" > "Edit Configurations".
   - In the configuration you want to run remotely, select the remote SDK under "Use classpath of module".
   - Click "Apply".

Now, when you run your application, it will run in the remote environment, but you'll still be able to use all the features of IntelliJ IDEA on your local machine.

Please note that the exact steps might vary slightly depending on the version of IntelliJ IDEA you're using and whether you're using Docker or a different type of remote environment.

If you want to use a remote development environment but you're using the "Community" edition of IntelliJ IDEA, one workaround is to use Visual Studio Code with the "Remote - SSH" or "Remote - Containers" extensions, which provide similar capabilities and are free to use.

Lastly, remember that while you can run your application in a remote environment, the source code itself will still need to be available locally if you want to use IntelliJ IDEA's code navigation and other features. If you're currently storing your code only in the Docker container, you might need to change your setup to store the code on your local machine and mount it as a volume in the Docker container.

## Benchmarking

### Using `ghz` to Benchmark the gRPC Service

You can use benchmarking tools like `ghz`, a high-performance gRPC benchmarking tool written in Go.

Firstly, you need to install `ghz`. If you have Go installed, you can use:

```bash
brew install ghz
```

Or you can download pre-compiled binaries from the `ghz` [releases page](https://github.com/bojand/ghz/releases).

Once you have `ghz` installed, you can run the benchmark with the following command:

```bash
ghz --insecure --proto examples/proto/helloworld/helloworld.proto --call helloworld.Greeter.SayHello -d '{"name":"Test"}' -c 200 -n 2000 0.0.0.0:50051
```

In this command:

- `--insecure`: Disable TLS.
- `--proto /path/to/helloworld.proto`: The path to the service's protocol buffer definition.
- `--call helloworld.Greeter.SayHello`: The fully-qualified name of the service method to call.
- `-d '{"name":"Test"}'`: The data to send with your request.
- `-c 200`: The number of concurrent requests to make.
- `-n 2000`: The total number of requests to make.
- `0.0.0.0:50051`: The address and port of the service to benchmark.

You will need to adjust the `--proto`, `--call`, `--d`, `--c`, `--n`, and server address arguments as needed for your service.

Please note, if the server you are testing uses TLS, you need to provide the certificate file to the `ghz` tool using the `--cacert` flag. You would also need to remove the `--insecure` flag.

# Performance Comparison: Tonic vs GMF

We've compared the performance of the Tonic-based GRPC server and client implementation with our enhanced Glommio Messaging Framework (GMF) based implementation. Here is a summary of the results:

| Metric       | Tonic-based Implementation | GMF-based Implementation |
|--------------|----------------------------|--------------------------|
| Requests/sec | 6512.01                    | 12009.70                 |
| Fastest      | 7.71 ms                    | 1.58 ms                  |
| Slowest      | 62.15 ms                   | 45.71 ms                 |
| Average      | 28.00 ms                   | 16.54 ms                 |

*Requests/sec*: Higher is better. This metric represents the number of requests that the server can handle per second.

We found that GMF-based implementation handled approximately 84% more requests per second compared to the Tonic-based implementation.

### Using Criterion.rs to Benchmark the GMF-based Implementation

We use the Criterion.rs library for benchmarking. Here's how you can run the benchmarks for this project:

   ```bash
   ./cargo-docker.sh bench -p examples -- --verbose
   ```

This will run all benchmarks in the project.

The output of the benchmark will be saved to the `target/criterion` directory. You can view a detailed report by opening `target/criterion/report/index.html` in your web browser.
