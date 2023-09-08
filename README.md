# rust-spy

Linux-only CLI to dump all threads from a running process.

If you want to spy on your Rust app, make sure you compile
it with debug information enabled (even in release mode)
so you get all symbols converted to files/lines/columns.

Depends on the `libwd` system lib. Install it on Debuntu with:

```
apt-get install libwd-dev
```

Installation:

```
cargo install rust-spy
```

Usage:

```
rust-spy [pid]
```

If you get a permission denied error, run as root.
