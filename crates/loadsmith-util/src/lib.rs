use std::{
    io::{BufReader, Read},
    path::Path,
};

pub fn hash_reader(mut reader: impl Read) -> std::io::Result<blake3::Hash> {
    let mut hasher = blake3::Hasher::new();
    std::io::copy(&mut reader, &mut hasher)?;

    Ok(hasher.finalize())
}

pub fn hash_file(path: impl AsRef<Path>) -> std::io::Result<blake3::Hash> {
    let mut file = std::fs::File::open(path).map(BufReader::new)?;
    hash_reader(&mut file)
}

pub fn hash_file_to_string(path: impl AsRef<Path>) -> std::io::Result<String> {
    hash_file(path).map(|hash| hash.to_hex().to_string())
}
