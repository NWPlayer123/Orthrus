# Orthrus
Orthrus is a work-in-progress modding toolkit that aims to support a wide array of game file formats, to allow
for blazing fast interoperability across systems and game engines. Project goals are minimizing dependencies
and allocations (where possible), as well as running as fast as possible.

For more about the structure of the Orthrus project, see the [Architecture](ARCHITECTURE.md) page.

## Current Formats
### ncompress - Nintendo Compression Formats
* Yay0 - used for early first-party engines on N64/GameCube
* Yaz0 - used across various first-party engines on N64, GameCube, Wii, Wii U and Switch
### panda3d - Panda3D Rendering/Game Engine
* Multifile - archive format that supports running as a full application
* BAM/BOO - binary model format used to store an internal scene graph
### godot - Godot Game Engine
* PCK (experimental) - archive format, either standalone or in a self-contained executable
### jsystem  (experimental)- Nintendo JSystem Middleware
* RARC (experimental) - Resource Archive, used for specifying which way to load specific files in-engine
### nintendoware (experimental) - NintendoWare for {Revolution, CTR, Cafe}
* BRSTM (experimental) - Streamed Audio, stored in DSP-(AD)PCM format
* BFSAR (experimental) - Sound Archive, used for metadata related to a game project


## Future Plans (Wishlist)
LZ77/Okumura compression, ASH0/ASR0 compression, BFSTM/BWAV, GUI/Rendering

## License

This software is licensed under the Mozilla Public License 2.0 ([LICENSE-MPL](LICENSE-MPL) or
https://mozilla.org/MPL/2.0/).

Commercial users may contact nikki@aetheria.dev if they wish to discuss alternative licensing arrangements.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this work by
you shall be licensed under the Mozilla Public License 2.0.