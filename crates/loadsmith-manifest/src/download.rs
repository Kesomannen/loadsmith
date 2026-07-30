use std::{fs, future::Future, io::Cursor, path::Path};

use loadsmith_core::FileUrl;
use url::Url;

use crate::{Error, Result};

pub async fn download_and_extract<F, Fut, E>(
    file: FileUrl,
    target: impl AsRef<Path>,
    download: F,
) -> Result<()>
where
    F: FnOnce(Url) -> Fut,
    Fut: Future<Output = std::result::Result<Vec<u8>, E>>,
    E: std::error::Error + Send + Sync + 'static,
{
    let target = target.as_ref();

    let zip_bytes = match file {
        FileUrl::Url(url) => download(url)
            .await
            .map_err(|err| Error::Download(Box::new(err)))?,
        FileUrl::Path(path) => {
            let metadata = path.metadata()?;

            if metadata.is_dir() {
                fs::create_dir_all(target)?;
                loadsmith_util::copy_dir(&path, target)?;
                return Ok(());
            }

            if path.extension().is_none_or(|ext| ext != "zip") {
                let canon = path.canonicalize()?;
                let file_name = canon
                    .file_name()
                    .expect("file name should exist for a file path");

                fs::create_dir_all(target)?;
                fs::copy(&canon, target.join(file_name))?;
                return Ok(());
            }

            fs::read(&path)?
        }
    };

    loadsmith_install::extract(Cursor::new(zip_bytes), target)?;

    Ok(())
}
