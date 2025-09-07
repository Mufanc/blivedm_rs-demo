mod database;
pub mod logger;

use directories::ProjectDirs;
use once_cell::sync::Lazy;

pub static PROJECT_DIRS: Lazy<ProjectDirs> =
    Lazy::new(|| ProjectDirs::from("xyz", "mufanc", "boa").expect("failed to get project dirs"));
