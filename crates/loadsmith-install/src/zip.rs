//! Abstractions over zip files.
//!
//! This module contains the [`Zip`] and [`ZipFile`] traits, as well as implementations for the [zip] crate.
//! This is mainly used to allow for mocking zip files in tests, and to allow for different zip implementations.
//!
//! The traits are currently sealed, meaning external crates cannot implement them.

use std::io::Read;

use camino::Utf8PathBuf;

use crate::Result;

/// An abstraction over zip archives.
///
/// See the [module level documentation](self) for more information.
pub trait Zip {
    type File<'a>: ZipFile
    where
        Self: 'a;

    fn len(&self, _: private::Token) -> usize;
    fn by_index<'a>(&'a mut self, index: usize, _: private::Token) -> Result<Self::File<'a>>;
}

/// An abstraction over a file in a zip archive.
///
/// See the [module level documentation](self) for more information.
pub trait ZipFile: Read {
    fn is_dir(&self, _: private::Token) -> bool;
    fn path(&self, _: private::Token) -> Result<Utf8PathBuf>;
    #[allow(unused)]
    fn unix_mode(&self, _: private::Token) -> Option<u32> {
        None
    }
}

pub(crate) mod private {
    #[derive(Clone, Copy)]
    pub struct Token;
}

#[cfg(test)]
pub(crate) mod mock {
    use std::io::Read;

    use camino::Utf8PathBuf;

    use super::{Zip, ZipFile, private};
    use crate::{Error, Result};

    #[derive(Debug, Default)]
    pub struct MockZip<'a> {
        files: Vec<MockZipFile<'a>>,
    }

    #[derive(Debug)]
    pub struct MockZipFile<'a> {
        path: Utf8PathBuf,
        contents: &'a [u8],
        is_dir: bool,
        // unix_mode: Option<u32>,
    }

    pub struct MockZipFileReader<'a> {
        file: &'a MockZipFile<'a>,
        position: usize,
    }

    impl<'a> MockZip<'a> {
        pub fn add_file(mut self, file: MockZipFile<'a>) -> Self {
            self.files.push(file);
            self
        }

        pub fn with_file(self, path: impl Into<Utf8PathBuf>, contents: &'a [u8]) -> Self {
            self.add_file(MockZipFile::new(path.into(), contents))
        }

        pub fn with_empty_file(self, path: impl Into<Utf8PathBuf>) -> Self {
            self.with_file(path, &[])
        }

        // pub fn with_dir(mut self, path: impl Into<Utf8PathBuf>) -> Self {
        //     self.files.push(MockZipFile::new_dir(path.into()));
        //     self
        // }
    }

    impl<'a> MockZipFile<'a> {
        pub fn new(path: Utf8PathBuf, contents: &'a [u8]) -> Self {
            Self {
                path,
                contents,
                is_dir: false,
                // unix_mode: None,
            }
        }

        // pub fn new_dir(path: Utf8PathBuf) -> Self {
        //     Self {
        //         path,
        //         contents: &[],
        //         is_dir: true,
        //         unix_mode: None,
        //     }
        // }

        // pub fn with_unix_mode(mut self, mode: u32) -> Self {
        //     self.unix_mode = Some(mode);
        //     self
        // }
    }

    impl<'a> Zip for MockZip<'a> {
        type File<'b>
            = MockZipFileReader<'b>
        where
            Self: 'b;

        fn len(&self, _: private::Token) -> usize {
            self.files.len()
        }

        fn by_index<'b>(&'b mut self, index: usize, _: private::Token) -> Result<Self::File<'b>> {
            let Some(file) = self.files.get(index) else {
                return Err(Error::ZipFileOutOfBounds {
                    index,
                    len: self.files.len(),
                });
            };

            Ok(MockZipFileReader { file, position: 0 })
        }
    }

    impl Read for MockZipFileReader<'_> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let mut remaining = &self.file.contents[self.position..];
            let n = remaining.read(buf)?;
            self.position += n;
            Ok(n)
        }
    }

    impl<'a> ZipFile for MockZipFileReader<'a> {
        fn is_dir(&self, _: private::Token) -> bool {
            self.file.is_dir
        }

        fn path(&self, _: private::Token) -> Result<Utf8PathBuf> {
            Ok(self.file.path.clone())
        }

        // fn unix_mode(&self, _: private::Token) -> Option<u32> {
        //     self.file.unix_mode
        // }
    }
}

/// [`Zip`] and [`ZipFile`] implementations for the [zip] crate.
mod zip_crate {
    use camino::Utf8PathBuf;
    use std::io::{Read, Seek};

    use super::{Zip, ZipFile, private};
    use crate::{Error, Result};

    impl<R: Seek + Read> Zip for zip::ZipArchive<R> {
        type File<'a>
            = zip::read::ZipFile<'a, R>
        where
            R: 'a;

        fn len(&self, _: private::Token) -> usize {
            self.len()
        }

        fn by_index<'a>(&'a mut self, index: usize, _: private::Token) -> Result<Self::File<'a>> {
            Ok(self.by_index(index)?)
        }
    }

    impl<'a, R: Seek + Read> ZipFile for zip::read::ZipFile<'a, R> {
        fn is_dir(&self, _: private::Token) -> bool {
            self.is_dir()
        }

        fn path(&self, _: private::Token) -> Result<Utf8PathBuf> {
            let path = self
                .enclosed_name()
                .ok_or_else(|| Error::InvalidZipPath(self.name().to_string()))?;

            let utf8_path = Utf8PathBuf::try_from(path)?;
            Ok(utf8_path)
        }

        fn unix_mode(&self, _: private::Token) -> Option<u32> {
            self.unix_mode()
        }
    }
}
