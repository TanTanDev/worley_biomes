## Worley biomes
A worley noise implementation, that supports k-nearest sampling + biome classification.

The library comes with a bevy, DebugPlugin that can visualize the worley as a texture + live tweak.

### compilation flag features
"serde", "bevy"

### in-depth my design decisions
This library uses a [further developed version](https://github.com/TanTanDev/bracket-fast-noise/tree/main) of 
[bracket-noise](https://crates.io/crates/bracket-noise).
The reason I don't fork bracket is because bracket-fast-nosie is a sub crate inside a collection of libraries.  
My version implement serialization+deserialization with serde.

My fork implements serde serialization/deserialization. I haven't done performance comparisons with other libraries, it does the job well, and I like the api. 


## Bevy support table

| bevy | worley biomes |
| ---- | ------------------- |
| 0.16 | 0.1.0               |


## License
worley_biomes is free and open source! All code in this repository is dual-licensed under either:

* MIT License ([LICENSE-MIT](docs/LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](docs/LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
