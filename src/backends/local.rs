use std::path::PathBuf;

use crate::config::UpdateSet;

use super::interface::StorageBackend;

pub struct LocalBackend {
    path: PathBuf,
}

impl StorageBackend for LocalBackend {
    fn list_updatesets(&mut self) -> anyhow::Result<Vec<String>> {
        let mut result = Vec::new();
        for entry in std::fs::read_dir(&self.path)? {
            let entry = entry?;
            let filename = entry.file_name().into_string().unwrap();
            if entry.path().is_file() && filename.starts_with("updateset-") {
                let id = filename.replace("updateset-", "").replace(".json", "");
                result.push(id);
            }
        }
        Ok(result)
    }

    fn get_updateset(&mut self, id: &str) -> anyhow::Result<UpdateSet> {
        let path = self.path.join(format!("updateset-{id}.json"));
        let updateset_content = std::fs::read_to_string(&path).map_err(|e| {
            eprintln!("Failed to read updateset file: {}", e);
            e
        })?;
        Ok(serde_json::from_str(&updateset_content)?)
    }

    fn update_updateset(&mut self, id: String, updateset: UpdateSet) -> anyhow::Result<()> {
        let path = self.path.join(format!("updateset-{id}.json"));
        let serialized = serde_json::to_string_pretty(&updateset)?;
        std::fs::write(&path, serialized)?;
        Ok(())
    }
}

impl LocalBackend {
    pub fn new(path: PathBuf) -> Self {
        LocalBackend { path }
    }
}
