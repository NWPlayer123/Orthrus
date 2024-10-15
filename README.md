# Orthrus
Orthrus is a work-in-progress modding toolkit that aims to support a wide array of game file formats, to allow for blazing fast interoperability across systems and game engines. It supports no_std for embedded platforms, and tries to keep dependencies to a minimum.

For more about the structure of the Orthrus project, see the [Architecture](ARCHITECTURE.md) page.

## Current Formats
### ncompress - Nintendo Compression Formats
* Yay0 - used for early first-party engines on N64/GameCube
* Yaz0 - used across various first-party engines on N64, GameCube, Wii, Wii U and Switch
### panda3d - Panda3D Rendering/Game Engine
* Multifile - archive format that supports running as a full application

## Future Plans (Wishlist)
Panda3D BAM, LZ77/Okumura compression, ASH0/ASR0 compression, BFSTM/BWAV, GUI/Rendering

## License

This software is licensed under multiple licenses:

### Non-Commercial Use
For non-commercial use, this software is licensed under either of:

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Commercial Use Restrictions
Any commercial use of this software, including but not limited to:

 * Using the software to provide commercial services
 * Incorporating the software into a commercial product
 * Any use of the software that generates revenue

requires explicit written permission. Please contact me at nikki@aetheria.dev to discuss commercial licensing terms.

Use of this software in a commercial context without explicit permission is strictly prohibited.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.