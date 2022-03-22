# WASI Server Examples

## TCP echo server

To build the project:
`cargo build --target wasm32-wasi`

Then:
`wasmtime run --tcplisten 127.0.0.1:9000 --env 'LISTEN_FDS=1' target/wasm32-wasi/debug/wasi-server-tcp.wasm`

Then on another terminal:
`nc 127.0.0.1 9000`

## HTTP server

To build the project:
`cargo build --target wasm32-wasi`

Then:
`wasmtime run --tcplisten 127.0.0.1:9000 --env 'LISTEN_FDS=1' target/wasm32-wasi/debug/wasi-server-http.wasm`

Then on another terminal:
`curl http://127.0.0.1:9000`
