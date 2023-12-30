Orthrus is designed to be a series of modules across various crates that can be plugged together for complex analysis across a wide range of file formats.

Each library should be able to be used independently of the Orthrus application, which should be used exclusively to provide an interface for all supported files, along with supporting "projects" to address an entire directory of files, geared toward modifying a video game's assets in a simplified way.

The `orthrus_core` crate is meant to hold any code that is usable by many different file formats, such as managing time/date, file I/O, and networking.

All functions should be designed to take the bare minimum (if possible a `&[u8]` slice) to do their processing, and use `Box<[u8]>` if any return data is needed. Choosing to convert to `Vec<u8>` should be a conscious decision by the caller as it implies re-allocation.

The `DataCursor`/`DataCursorRef`/`DataCursorMut` types in `orthrus_core` can be used if more complex data handling is needed, but they should be internal so that callers aren't forced to use them.
