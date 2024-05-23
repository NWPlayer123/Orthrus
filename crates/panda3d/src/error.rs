//! Shared error definitions for the Panda3D engine

/// Error conditions for when working with Multifile archives.
#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum Error {
    /// Thrown when trying to open a file or folder that doesn't exist.
    #[snafu(display("Unable to find file/folder!"))]
    NotFound,
    /// Thrown if reading/writing tries to go out of bounds.
    #[snafu(display("Unexpected End-Of-File!"))]
    EndOfFile,
    /// Thrown when unable to open a file or folder.
    #[snafu(display("No permissions to open file/folder!"))]
    PermissionDenied,
    /// Thrown if the header contains a magic number other than "pmf\0\n\r".
    #[snafu(display("Invalid Magic! Expected {:?}.", Multifile::MAGIC))]
    InvalidMagic,
    /// Thrown if the header version is too new to be supported.
    #[snafu(display(
        "Unknown Multifile Version! Expected >= v{}.",
        Multifile::CURRENT_VERSION
    ))]
    UnknownVersion,
}
