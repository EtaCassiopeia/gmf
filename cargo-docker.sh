#!/bin/bash

# Name of your Docker container
CONTAINER_NAME=gmf-rust-app

# Cargo registry directory on your host machine
CARGO_REGISTRY_DIR="${HOME}/.cargo/registry"

# Create the cargo registry directory on your host machine if it doesn't exist
mkdir -p ${CARGO_REGISTRY_DIR}

# Check if the Docker container is running
if ! docker ps | grep -q $CONTAINER_NAME; then
    # If not, check if the container exists but is stopped
    if docker ps -a | grep -q $CONTAINER_NAME; then
        # If it exists, start the container
        docker start $CONTAINER_NAME
    else
        # If it doesn't exist, create and start the container
        docker run --rm \
            --ulimit memlock=-1:-1 \
            -p 50051:50051 \
            -d --name $CONTAINER_NAME \
            -v "$(pwd)":/usr/src/app \
            -v "${CARGO_REGISTRY_DIR}:/usr/local/cargo/registry" \
            -w /usr/src/app \
            rust-protoc:nightly-bookworm \
            sleep infinity
    fi
fi

# Run the specified cargo command inside the running Docker container
docker exec -it -e RUST_BACKTRACE=1 -e RUST_LOG=info $CONTAINER_NAME cargo $@
