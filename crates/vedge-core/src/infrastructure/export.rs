//! Export infrastructure (slice 5.3): the in-memory tar [`archive`] and the
//! sealed [`envelope`]. Both operate on bytes in RAM — the plaintext tar never
//! touches disk (Decision ②).

pub mod archive;
pub mod csv;
pub mod envelope;
