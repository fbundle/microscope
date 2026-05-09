use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

pub fn non_empty(filename: &str) -> bool {
    if let Ok(meta) = fs::metadata(filename) {
        meta.is_file() && meta.len() > 0
    } else {
        false
    }
}

fn write_file<I>(filename: &Path, iter: I) -> io::Result<()>
where
    I: Iterator<Item = Vec<char>>,
{
    let file = fs::File::create(filename)?;
    let mut writer = BufWriter::new(file);
    for line in iter {
        let s: String = line.into_iter().collect();
        writeln!(writer, "{}", s)?;
    }
    writer.flush()?;
    Ok(())
}

fn copy_file(src: &Path, dst: &Path) -> io::Result<()> {
    fs::copy(src, dst)?;
    Ok(())
}

fn move_file(dst: &Path, src: &Path) -> io::Result<()> {
    let dst_mode = if let Ok(meta) = fs::metadata(dst) {
        Some(meta.permissions())
    } else {
        None
    };

    if fs::rename(src, dst).is_err() {
        copy_file(src, dst)?;
        fs::remove_file(src)?;
    }

    if let Some(perms) = dst_mode {
        let _ = fs::set_permissions(dst, perms);
    }
    Ok(())
}

pub fn safe_write_file<I>(filename: &str, iter: I) -> io::Result<()>
where
    I: Iterator<Item = Vec<char>>,
{
    let abs_path = PathBuf::from(filename)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(filename));

    let cfg = crate::config::load();
    let tmp_filename = cfg.tmp_dir.join(
        abs_path
            .to_string_lossy()
            .trim_start_matches('/')
            .to_string(),
    );

    if let Some(parent) = tmp_filename.parent() {
        fs::create_dir_all(parent)?;
    }

    write_file(&tmp_filename, iter)?;
    move_file(Path::new(filename), &tmp_filename)?;
    Ok(())
}
