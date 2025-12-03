use crate::config::UpdateSet;

pub trait StorageBackend {
    fn list_updatesets(&mut self) -> anyhow::Result<Vec<String>>;
    fn get_updateset(&mut self, id: &str) -> anyhow::Result<UpdateSet>;
    fn update_updateset(&mut self, id: String, updateset: UpdateSet) -> anyhow::Result<()>;
}
