use std::io::{BufReader, BufWriter, Error as IOError, Read, Seek, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;
use zip::result::ZipError;

const MAGIC_NUMBER: &[u8] = b"COREPKG";
const HEADER_SIZE: usize = 0x25;

#[derive(Error, Debug)]
pub enum PackError {
    #[error("IO: {0}")]
    IO(#[from] IOError),

    #[error("Zip: {0}")]
    Zip(#[from] ZipError),

    #[error("given path `{0}` not a valid folder")]
    NotAFolder(PathBuf),
    #[error("can't get filename of `{0}`")]
    CantGetFilename(PathBuf),
    #[error("`{0}` file missing")]
    MissingDatapackJson(PathBuf),
}

pub fn pack_folder(path: &Path) -> Result<(), PackError> {
    let path = path.to_path_buf();
    if !path.is_dir() {
        return Err(PackError::NotAFolder(path));
    }
    let Some(file_name) = path.file_name() else {
        return Err(PackError::CantGetFilename(path));
    };

    let mut file_path = path.clone();
    file_path.push("datapack.json");
    let datapack_json = std::fs::read(file_path)?;

    let mut file_path = path.clone();
    file_path.push("loc");
    file_path.push("text.csv");
    let lang_file = if file_path.exists() {
        std::fs::read(file_path)?
    } else {
        Vec::new()
    };


    // TODO specify output file via pack options?
    let mut file_path = path.clone();
    file_path.push("bin");
    if !file_path.exists() {
        std::fs::create_dir(&file_path)?;
    }
    file_path.push(file_name);
    file_path.set_extension("corepackage");

    let file = std::fs::File::create(file_path)?;
    let mut writer = BufWriter::new(file);

    writer.write_all(MAGIC_NUMBER)?;
    // TODO lang
    let datapack_json_offset = datapack_json.len() + HEADER_SIZE;
    let lang_file_offset = datapack_json_offset + lang_file.len();

    writer.write_fmt(format_args!(
        "{:#010x}{:#010x}{:#010x}",
        HEADER_SIZE, datapack_json_offset, lang_file_offset
    ))?;
    writer.write_all(&datapack_json)?;
    writer.write_all(&lang_file)?;

    // TODO Use decorator over write to override seek & file position instead of `zip_buf`
    let mut zip_buf = Vec::new();
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_buf));

    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        // TODO compression level from option
        .compression_level(Some(0));

    let dir = walkdir::WalkDir::new(&path).into_iter().flatten();

    let mut buf = Vec::new();

    const SKIP_EXTENSIONS: &[&str] = &["corepackage"];

    for entry in dir {
        let entry_path = entry.path();

        if let Some(Some(ext)) = entry_path.extension().map(|s| s.to_str()) {
            let skip = SKIP_EXTENSIONS.iter().any(|skip_ext| *skip_ext == ext);
            if skip {
                continue;
            }
        }

        let entry_name = entry_path
            .strip_prefix(&path)
            .map_err(|_| PackError::CantGetFilename(entry_path.to_path_buf()))?
            .as_os_str()
            .to_str()
            .ok_or_else(|| PackError::CantGetFilename(entry_path.to_path_buf()))?;

        if entry_name.is_empty() {
            continue;
        }

        if entry_path.is_file() {
            zip.start_file(entry_name, options)?;

            let mut file = std::fs::File::open(entry_path)?;
            buf.clear();
            file.read_to_end(&mut buf)?;
            zip.write_all(&buf)?;
        } else {
            zip.add_directory(entry_name, options)?;
        }
    }
    zip.finish()?;
    std::mem::drop(zip);

    writer.write_all(&zip_buf)?;

    Ok(())
}

#[derive(Error, Debug)]
pub enum UnpackError {
    #[error("IO: {0}")]
    IO(#[from] IOError),
    #[error("Zip: {0}")]
    Zip(#[from] ZipError),

    #[error("missing magic number")]
    MissingMagicNumber,
    #[error("invalid contest pointer offsets")]
    IvalidOffsets,
}

pub fn unpack_folder(mod_file: &Path, out_path: &Path) -> Result<(), UnpackError> {
    let file = std::fs::File::open(mod_file)?;
    let mut reader = BufReader::new(file);

    let mut header = [0; HEADER_SIZE];
    reader.read_exact(&mut header)?;

    if !header.starts_with(MAGIC_NUMBER) {
        return Err(UnpackError::MissingMagicNumber);
    };
    const DATA_PTR_BEG: usize = HEADER_SIZE - 8;
    const DATA_PTR_END: usize = HEADER_SIZE;

    let data_ptr_bytes = &header[DATA_PTR_BEG..DATA_PTR_END];
    let data_ptr_str =
        std::str::from_utf8(data_ptr_bytes).map_err(|_| UnpackError::IvalidOffsets)?;
    let data_ptr =
        usize::from_str_radix(data_ptr_str, 16).map_err(|_| UnpackError::IvalidOffsets)?;

    // TODO instead of reading full file use seek decorator with offset
    reader.seek(std::io::SeekFrom::Start(data_ptr as u64))?;
    let mut content = Vec::new();
    reader.read_to_end(&mut content)?;
    let reader = std::io::Cursor::new(content.as_slice());

    let mut archive = zip::ZipArchive::new(reader)?;

    archive.extract(out_path)?;

    Ok(())
}
