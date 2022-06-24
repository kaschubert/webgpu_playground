### How to run this example with WebGL backend
NOTE: Currently, WebGL backend is is still experimental, so expect bugs.

# 0. Install prerequisites
cargo install wasm-bindgen-cli https
# 1. cd to the current folder
# 2. Compile wasm module
cargo build --target wasm32-unknown-unknown
# 3. Invoke wasm-bindgen
wasm-bindgen target/wasm32-unknown-unknown/debug/webgpu_tut_lib.wasm --out-dir . --target web --no-typescript
# 4. run http server
http
# 5. Open 127.0.0.1:8000 in browser