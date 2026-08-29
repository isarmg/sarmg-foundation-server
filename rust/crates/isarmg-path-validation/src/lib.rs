use std::path::{Path, PathBuf};

pub struct Root {
    root: PathBuf,
}

impl Root {
    pub fn new(root: impl AsRef<Path>) -> anyhow::Result<Self> {
        let root = root.as_ref().to_path_buf();
        anyhow::ensure!(
            root.is_dir(),
            "storage root is not a directory: {}",
            root.display()
        );
        Ok(Self { root })
    }

    pub fn resolve(&self, relative: &str) -> anyhow::Result<PathBuf> {
        anyhow::ensure!(
            !relative.is_empty()
                && !relative.starts_with('/')
                && !Path::new(relative).components().any(|component| {
                    matches!(
                        component,
                        std::path::Component::ParentDir | std::path::Component::RootDir
                    )
                }),
            "only relative, non-escaping storage paths are allowed"
        );
        Ok(self.root.join(relative))
    }
}
