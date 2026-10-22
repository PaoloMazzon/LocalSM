This is a toy project for a really rudimentary document/NoSQL database.

![Coverage](assets/coverage.svg)

## Details

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