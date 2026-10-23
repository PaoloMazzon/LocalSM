This is a toy project for a really rudimentary document/NoSQL database. There
are no real plans on implementing something like a querying interface, I'm
focused almost entirely on the storage engine. As such, this would be more
useful as an "on-disk hashmap."

![Coverage](assets/coverage.svg)

# Building/Using/Running
It's pure Rust, you can include it in your project with something like

```sh
cargo add localsm --git https://github.com/PaoloMazzon/LocalSM --branch main
```

## Documentation
[Rust docs are auto-generated into GH pages.](https://paolomazzon.github.io/LocalSM/localsm/index.html)

# Roadmap

 - [x] `BitField` implementation
 - [x] `BloomFilter` implementation
 - [x] In-memory SSTables
 - [ ] Disk-based SSTable searching
   - [x] Sparse key/location lookup table
   - [x] Serializing/deserializing SSTable
   - [x] Encode by blocks, compressing each one
   - [ ] Find keys through walking files
   - [ ] Iterate over all keys in a file
 - [ ] Compaction algorithm
 - [ ] High-level public interface
 - [ ] Cryptographic item destruction