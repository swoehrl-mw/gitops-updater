use crate::schemas::shared::K8sBase;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OciRepository {
    #[serde(flatten)]
    pub k8s_base: K8sBase,
    pub spec: OciRepositorySpec,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OciRepositorySpec {
    pub url: String,
    #[serde(rename = "ref")]
    pub reference: Ref,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ref {
    pub tag: String,
}
