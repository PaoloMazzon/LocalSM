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
   - [x] Find keys through walking files
   - [ ] Iterate over all keys in a file
 - [ ] Higher-level controller that lets you arbitrarily read/write
 - [ ] Compaction algorithm
 - [ ] High-level public interface
 - [ ] Cryptographic item destruction

# Controller/high-level design
From a high-level: The controller stores any number of MemTables in memory.
Once a MemTable hits its memory threshhold, it is marked read-only and from
then until its flushed to disk. Writes attempt to hit any non-read-only
MemTable, and reads first attempt to hit MemTables before starting backwards
into the SSTables on disk.

## MemTables
MemTables are stored in a list and the amount of them in unbounded. In
practice, they exist in three states:

 - Write/read - They aren't yet at threshhold and can take writes and reads
 - Read-only - They have hit thresshold and until they are safely written to
   disk they may only be read from
 - Written out - They are completely done writing to disk, and once there are
   no more readers, will be deleted

 Because writes lock, we can have any number of MemTables in the event of many
 concurrent writes.

## SSTables
A tiered-storage approach is used for SSTables. The first level (L0) is where
all SSTables initially land, and is the first place reads are searched from.
Tiers below L0 are significantly larger than L0, and unlike L0, guaranteed to
be completely separate ranges (ie, one file will have all a-d records, and
another would have all e-h records).
