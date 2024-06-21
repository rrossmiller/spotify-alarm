use std::path;

const CACHE_PATH: &str = ".spotify_cache";
pub fn get_home_path() -> Result<path::PathBuf, ()> {
    let mut home_path: path::PathBuf;
    if let Some(pth) = home::home_dir() {
        home_path = pth;
    } else {
        return Err(());
    }

    home_path.push(CACHE_PATH);
    Ok(home_path)
}
