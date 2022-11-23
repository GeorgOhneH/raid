# RAID

## Installation
* Install [rustup](https://www.rust-lang.org/tools/install)
* Switch to nightly `rustup default nightly`

## Run Fuzz
`cargo run --release`

To change the number of data devices, checksum devices or/and the chunk size 
you need to change the following lines in the file `src/fuzz.rs`.
```rust
// src/fuzz.rs
const D: usize = 30; // number of data devices
const C: usize = 3; // number of checksum devices
const X: usize = 2usize.pow(20); // chunk size
```  

## Run Bench
`cargo bench`