#!/bin/bash

set -e
set -x

rustc --version

cargo install --force cbindgen
rm -rf ./include

rustup toolchain install nightly-2022-12-01

echo -------------------------------------
echo - Build library with CMake
echo -------------------------------------
mkdir -p build
cd build
cmake -DCMAKE_BUILD_TYPE=Debug ..
cmake --build . -v
cd ..

echo -------------------------------------
echo - Generate header with cbindgen
echo -------------------------------------
rustup run nightly-2022-12-01 cbindgen \
  --config cbindgen.toml \
  --crate pact_ffi \
  --output include/pact.h
rustup run nightly-2022-12-01 cbindgen \
  --config cbindgen-c++.toml \
  --crate pact_ffi \
  --output include/pact-c++.h

echo -------------------------------------
echo - Make library available for examples
echo -------------------------------------
cd build
cmake --install . --prefix ./install

echo -------------------------------------
echo - Running examples
echo -------------------------------------
cd ..
for i in examples/*; do
  pushd $i
  mkdir -p build
  cd build
  cmake ..
  cmake --build .

  echo "Running example $i"
  if [[ "$OSTYPE" == "msys"* ]]; then
    cp ../../../build/install/lib/*.dll Debug/
    ./Debug/example.exe
  elif [[ "$OSTYPE" == "darwin"* ]]; then
    cp ../../../build/install/lib/*.dylib .
    ./example
  else
    ./example
  fi

  popd
done
