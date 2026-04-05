# Learn by Building

Deep-dive courses into systems programming concepts, built from first principles in Rust.

**[Read the book →](https://its-saeed.github.io/learn-by-building/)**

## Courses

| Course | Lessons | What you'll build |
|--------|---------|-------------------|
| [**TLS**](https://its-saeed.github.io/learn-by-building/tls/) | 13 lessons | Hashing, encryption, signatures, key exchange, certificates → encrypted echo server |
| [**Async Rust**](https://its-saeed.github.io/learn-by-building/async/) | 29 lessons + 4 projects | Futures, wakers, executors, reactors → your own runtime, chat server, load tester |

## Running exercises locally

```sh
git clone https://github.com/its-saeed/learn-by-building.git
cd learn-by-building

# TLS exercises
cargo run -p tls --bin 1-hash -- --file-path Cargo.toml
cargo run -p tls --bin 7-echo-server

# Async exercises
cargo run -p async-lessons --bin 1-futures -- all
cargo run -p async-lessons --bin 16-tokio-architecture -- multi-thread
```

## Prerequisites

- Rust fundamentals (ownership, traits, generics)
- Basic networking (TCP/UDP)
