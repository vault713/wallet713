# Building wallet713

## Requirements
wallet713 has the [same requirements](https://github.com/mimblewimble/grin/blob/master/doc/build.md#requirements) as Grin and also requires **a fully synced Grin node to be running in order to be operational**.

## Installation

```
$ git clone https://github.com/vault713/wallet713
$ cd wallet713
$ cargo build --release
```
And then to run:
```
$ cd target/release
$ ./wallet713
```