use crate::{
    backends::{interface::StorageBackend, local::LocalBackend},
    config::BackendConfig,
};

pub fn init_backend(backend: Option<BackendConfig>) -> anyhow::Result<Box<dyn StorageBackend>> {
    match backend {
        None => Ok(Box::new(LocalBackend::new(".".into()))),
        Some(BackendConfig::Local { path }) => {
            Ok(Box::new(LocalBackend::new(path.unwrap_or_default())))
        }
        Some(BackendConfig::Github { repo: _ }) => {
            panic!("Not implemented yet");
        }
    }
}
