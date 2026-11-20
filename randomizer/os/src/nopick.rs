use std::path::PathBuf;

pub enum Outcome {
    Chosen(PathBuf),
    Cancelled,
    Failed(String),
}

pub fn pick(_start: &str, _title: &str, _file: bool, _dry: bool) -> Outcome {
    Outcome::Failed("the Windows picker only runs on Windows".into())
}
