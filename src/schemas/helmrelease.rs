use crate::schemas::shared::K8sBase;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelmRelease {
    #[serde(flatten)]
    pub k8s_base: K8sBase,
    pub spec: HelmReleaseSpec,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelmReleaseSpec {
    pub chart: Option<HelmReleaseChart>,
    pub chart_ref: Option<SourceRef>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelmReleaseChart {
    pub spec: HelmReleaseChartSpec,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelmReleaseChartSpec {
    pub chart: String,
    pub version: Option<String>,
    pub source_ref: SourceRef,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceRef {
    pub kind: String,
    pub name: String,
    pub namespace: Option<String>,
}
