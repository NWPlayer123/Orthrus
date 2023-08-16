Orthrus is designed to be a series of modules across various crates that can be plugged together for complex analysis across a wide range of file formats.

The main Orthrus crate should be used exclusively to combine all supported files, along with supporting "projects" to address an entire directory of files, geared toward modifying a video game's assets in a simplified way.

The orthrus_core crate is meant to hold any code that is usable by many different file formats, such as managing time/date, file I/O, error handling, and networking. 
