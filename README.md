# How to use putty-rs

## Usage

```bash
cargo run -- --help
```

## Test program with putty as other end of virtual serial device

```bash
socat -d -d pty,raw,echo=0 pty,raw,echo=0 # Create two connected virtual serial devices e.g. /dev/pts/2 and /dev/pts/3
# Connect a programm like putty to /dev/pts/2 or just launch putty-rs twice
cargo run -- --port /dev/pts/2 # run putty-rs and connect it to /dev/pts/3
cargo run -- --port /dev/pts/3 # run putty-rs and connect it to /dev/pts/3
```
