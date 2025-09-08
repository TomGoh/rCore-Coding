#!/bin/bash

set -e

echo "=== Cleaning previous build ==="
make clean

echo "=== Building and running OS ==="
echo "Press Ctrl+A then X to exit QEMU"
make run