#!/bin/bash

# Docker Image name
IMAGE_NAME=rust-protoc

# Docker Image version
IMAGE_VERSION=1.69.0

# Build Docker Image
docker build -t ${IMAGE_NAME}:${IMAGE_VERSION} .

echo "Docker Image ${IMAGE_NAME}:${IMAGE_VERSION} has been built successfully."
