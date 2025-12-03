use std::collections::HashMap;

use crate::schemas::shared::K8sBase;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Kustomization {
    #[serde(flatten)]
    pub k8s_base: K8sBase,
    pub spec: KustomizationSpec,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KustomizationSpec {
    pub path: String,
    pub post_build: Option<PostBuild>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostBuild {
    pub substitute: Option<HashMap<String, String>>,
}
