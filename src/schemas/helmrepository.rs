use crate::schemas::shared::K8sBase;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelmRepository {
    #[serde(flatten)]
    pub k8s_base: K8sBase,
    pub spec: HelmRepositorySpec,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelmRepositorySpec {
    pub url: String,
}
