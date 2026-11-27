// Path-loading the example runs its constructor and presentation codec tests in client CI.
// Binary-only entry points and I/O helpers are unused by this integration target.
#[allow(dead_code)]
#[path = "../examples/threejs_host/main.rs"]
mod threejs_host;
