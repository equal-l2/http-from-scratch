# HTTP Server from scratch
Toy project to learn how to build server and learn about async ecosystem of Rust

## Plans
1. Build a single-threaded synchronous HTTP/1.1 server
    - [x] Parse a request header
    - [ ] Parse path and read content (with caring about path traversal)
    - [ ] Parse body
    - [ ] Build response and send it (should be easy)
    - [ ] Consider `keep-alive`ing? (maybe)
2. Build a single-threaded async runtime and turn the HTTP server into async
    - [ ] Read [the async book](https://rust-lang.github.io/async-book/)
    - [ ] Totally no idea after that, currently
3. Make the runtime multi-threaded
    - [ ] Steal idea from tokio/async-std/actix?

## Problem
- Can't handle keep-alive yet (Modern browsers assume all server supports it :( )
