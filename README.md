# rust-spy

Linux-only CLI to dump all threads from a running process.

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
