# Rusty DAWG

A library for building suffix automata for string indexing and searching in Rust.

```
cargo test
cargo build
```

To build the DAG on the Brown corpus:

```
cat ~/Desktop/brown.txt | ./target/debug/rusty-dawg
```