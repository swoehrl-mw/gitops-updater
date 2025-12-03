use std::cmp::Ordering;

use anyhow::Context;
use regex::Regex;
use reqwest::{
    Response,
    header::{AUTHORIZATION, HeaderValue, LINK, WWW_AUTHENTICATE},
};
use semver::Version;
use serde::Deserialize;

use crate::sources::auth::auth_from_docker_config;

pub async fn find_newest_version_of_chart(
    repo_url: &str,
    chart_name: &str,
) -> anyhow::Result<Option<String>> {
    let tags = get_tags(&format!("{repo_url}/{chart_name}")).await?;
    let newest = tags
        .into_iter()
        .filter_map(|v| Version::parse(&v).ok())
        .max_by(|a, b| a.cmp(b))
        .map(|v| v.to_string());
    Ok(newest)
}

pub async fn find_newest_version_of_container_image(
    image_name: &str,
    version_regex: &Option<String>,
) -> anyhow::Result<Option<String>> {
    let tags = get_tags(image_name).await?;
    let mut tags = if let Some(regex) = version_regex {
        let re = Regex::new(regex)?;
        tags.into_iter().filter(|t| re.is_match(t)).collect()
    } else {
        tags
    };
    tags.sort_by(|a, b| compare_tags(b, a));
    Ok(tags.into_iter().next())
}

fn try_parse_version(s: &str) -> Option<Version> {
    Version::parse(s)
        .or_else(|_| Version::parse(s.trim_start_matches(['v', 'V'])))
        .ok()
}

fn compare_tags(a: &str, b: &str) -> Ordering {
    // Try semver parsing first (allow optional leading 'v')
    if let (Some(va), Some(vb)) = (try_parse_version(a), try_parse_version(b)) {
        return va.cmp(&vb);
    }

    // Fallback: component-wise comparison by '.' with numeric parts compared numerically
    let a_parts: Vec<&str> = a.split('.').collect();
    let b_parts: Vec<&str> = b.split('.').collect();
    let len = std::cmp::min(a_parts.len(), b_parts.len());

    for i in 0..len {
        let ap = a_parts[i];
        let bp = b_parts[i];
        // try numeric comparison first
        let an = ap.parse::<u64>();
        let bn = bp.parse::<u64>();
        match (an, bn) {
            (Ok(na), Ok(nb)) => {
                if na < nb {
                    return Ordering::Less;
                } else if na > nb {
                    return Ordering::Greater;
                }
            }
            (Ok(_), Err(_)) => return Ordering::Greater,
            (Err(_), Ok(_)) => return Ordering::Less,
            (Err(_), Err(_)) => {
                let ord = ap.cmp(bp);
                if ord != Ordering::Equal {
                    return ord;
                }
            }
        }
    }

    // all compared parts equal; if lengths differ, decide based on remaining parts
    if a_parts.len() == b_parts.len() {
        return Ordering::Equal;
    }

    // check remaining parts in the longer one for non-zero numeric or non-empty string
    if a_parts.len() > b_parts.len() {
        for part in &a_parts[len..] {
            if let Ok(n) = part.parse::<u64>() {
                if n > 0 {
                    return Ordering::Greater;
                }
            } else if !part.is_empty() {
                return Ordering::Greater;
            }
        }
        Ordering::Equal
    } else {
        for part in &b_parts[len..] {
            if let Ok(n) = part.parse::<u64>() {
                if n > 0 {
                    return Ordering::Less;
                }
            } else if !part.is_empty() {
                return Ordering::Less;
            }
        }
        Ordering::Equal
    }
}

async fn get_tags(image_name: &str) -> anyhow::Result<Vec<String>> {
    let image_name = complete_image_name(image_name);
    let mut tag_list = Vec::new();
    let RegistryInfo {
        registry,
        base_url,
        tag_list_url,
    } = calculate_url(&image_name);
    let mut next_url = Some(tag_list_url);
    let auth = auth_from_docker_config()?.get(&registry).cloned();
    let mut client = TagClient::new(auth);

    while let Some(url) = next_url.take() {
        let (mut tags, next) = client.get_tags(&url).await?;
        tag_list.append(&mut tags);
        if let Some(next) = next {
            next_url = Some(format!("{base_url}/{next}"));
        }
    }

    Ok(tag_list)
}

struct TagClient {
    client: reqwest::Client,
    token: Option<String>,
    auth: Option<String>,
}

impl TagClient {
    pub fn new(auth: Option<String>) -> TagClient {
        TagClient {
            client: reqwest::Client::new(),
            token: None,
            auth,
        }
    }
    async fn get_tags(&mut self, url: &str) -> anyhow::Result<(Vec<String>, Option<String>)> {
        let mut response = self.perform_request(url).await?;
        if response.status().as_u16() == 401 {
            if self.token.is_some() {
                panic!("Got 401 even though token was provided");
            }
            self.handle_auth(&response).await?;
            response = self.perform_request(url).await?;
        }
        if response.status().is_success() {
            let link = next_link(&response).await?;
            let tags: TagList = response.json().await?;
            Ok((tags.tags, link))
        } else {
            Err(anyhow::format_err!(
                "Could not get tags from registry: {}",
                response.text().await?.to_owned()
            ))
        }
    }

    async fn perform_request(&mut self, url: &str) -> anyhow::Result<reqwest::Response> {
        let mut request = self.client.get(url);
        if let Some(token) = self.token.as_ref() {
            request = request.header(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {token}"))?,
            );
        }
        Ok(request.send().await?)
    }

    async fn handle_auth(&mut self, response: &Response) -> anyhow::Result<()> {
        let header = response
            .headers()
            .get(WWW_AUTHENTICATE)
            .ok_or(anyhow::format_err!("No www-authenticate header found"))?;
        let bearer = parse_www_authenticate(header.to_str()?)?;
        self.get_token(&bearer.realm, &bearer.service, &bearer.scope)
            .await?;
        Ok(())
    }

    async fn get_token(&mut self, realm: &str, service: &str, scope: &str) -> anyhow::Result<()> {
        let mut request = self
            .client
            .get(format!("{realm}?service={service}&scope={scope}"));
        if let Some(auth) = self.auth.as_ref() {
            request = request.header(AUTHORIZATION, format!("Basic {auth}"));
        }
        let result = request.send().await?;
        let token: Token = result.json().await?;
        self.token = Some(token.token);
        Ok(())
    }
}

async fn next_link(response: &Response) -> anyhow::Result<Option<String>> {
    if let Some(link) = response.headers().get(LINK) {
        let result = parse_link(link.to_str()?)?;
        Ok(Some(result))
    } else {
        Ok(None)
    }
}

struct RegistryInfo {
    pub registry: String,
    pub base_url: String,
    pub tag_list_url: String,
}

fn calculate_url(image_name: &str) -> RegistryInfo {
    if image_name.contains('.') {
        let (host, image) = image_name.split_once('/').unwrap_or_else(|| {
            panic!("Could not split image name into host and image: '{image_name}'")
        });
        RegistryInfo {
            registry: host.to_owned(),
            base_url: format!("https://{host}"),
            tag_list_url: format!("https://{host}/v2/{image}/tags/list"),
        }
    } else if image_name.contains('/') {
        RegistryInfo {
            registry: "index.docker.io".to_owned(),
            base_url: "https://registry-1.docker.io".to_owned(),
            tag_list_url: format!("https://registry-1.docker.io/v2/{image_name}/tags/list"),
        }
    } else {
        RegistryInfo {
            registry: "index.docker.io".to_owned(),
            base_url: "https://registry-1.docker.io".to_owned(),
            tag_list_url: format!("https://registry-1.docker.io/v2/library/{image_name}/tags/list"),
        }
    }
}

#[derive(Deserialize)]
struct Token {
    pub token: String,
}

#[derive(Deserialize)]
struct TagList {
    pub tags: Vec<String>,
}

struct Bearer {
    pub realm: String,
    pub service: String,
    pub scope: String,
}

fn parse_www_authenticate(input: &str) -> anyhow::Result<Bearer> {
    assert!(input.starts_with("Bearer "));
    let re_realm = Regex::new(r#"realm="([\w:\/\.]+)""#).expect("Could not create regex for realm");
    let re_service =
        Regex::new(r#"service="([\w:\/\.]+)""#).expect("Could not create regex for service");
    let re_scope =
        Regex::new(r#"scope="([\w:\/\-_\.*]+)""#).expect("Could not create regex for scope");
    let realm = re_realm
        .captures(input)
        .context(format!(
            "Could not extract realm from authenticate header: '{input}'"
        ))?
        .get(1)
        .context(format!(
            "Could not extract realm from authenticate header: '{input}'"
        ))?
        .as_str()
        .to_owned();
    let service = re_service
        .captures(input)
        .context(format!(
            "Could not extract service from authenticate header: '{input}'"
        ))?
        .get(1)
        .context(format!(
            "Could not extract service from authenticate header: '{input}'"
        ))?
        .as_str()
        .to_owned();
    let scope = re_scope
        .captures(input)
        .context(format!(
            "Could not extract scope from authenticate header: '{input}'"
        ))?
        .get(1)
        .context(format!(
            "Could not extract scope from authenticate header: '{input}'"
        ))?
        .as_str()
        .to_owned();
    Ok(Bearer {
        realm,
        service,
        scope,
    })
}

fn parse_link(input: &str) -> anyhow::Result<String> {
    // example: "</v2/gitlab-org/gitlab-runner/tags/list?last=alpine-v10.7.1&n=100>; rel=\"next\""
    if let Some((left, _)) = input.split_once(';') {
        Ok(left.replace(['<', '>'], ""))
    } else {
        Err(anyhow::format_err!("Could not parse link: {input}"))
    }
}

fn complete_image_name(image: &str) -> String {
    if image.contains(".") {
        // assume FQDN
        image.to_owned()
    } else if image.contains("/") {
        // assume docker hub with namespace
        format!("index.docker.io/{image}")
    } else {
        // assume docker hub library name
        format!("index.docker.io/library/{image}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn compares_dot_numeric() {
        assert_eq!(compare_tags("10.0", "9.0.1"), Ordering::Greater);
        assert_eq!(compare_tags("9.0.1", "10.0"), Ordering::Less);
    }

    #[test]
    fn numeric_vs_non_numeric_component() {
        assert_eq!(compare_tags("1.2", "1.2a"), Ordering::Greater);
        assert_eq!(compare_tags("1.2a", "1.2"), Ordering::Less);
    }

    #[test]
    fn trailing_zero_equality() {
        assert_eq!(compare_tags("1.0", "1"), Ordering::Equal);
    }

    #[test]
    fn extra_nonzero_part() {
        assert_eq!(compare_tags("1.0.1", "1"), Ordering::Greater);
    }

    #[test]
    fn lexicographic_parts() {
        assert_eq!(compare_tags("a.b", "a.c"), Ordering::Less);
    }

    #[test]
    fn equal_tags() {
        assert_eq!(compare_tags("1.2.3", "1.2.3"), Ordering::Equal);
    }

    #[test]
    fn semver_v_prefix_and_semver_compare() {
        // v-prefixed should compare equal to non-prefixed semver
        assert_eq!(compare_tags("v1.2.3", "1.2.3"), Ordering::Equal);
        // semver numeric comparison
        assert_eq!(compare_tags("v2.0.0", "v1.9.9"), Ordering::Greater);
    }

    #[test]
    fn semver_vs_prerelease() {
        // release > prerelease
        assert_eq!(compare_tags("v1.2.3", "1.2.3-beta"), Ordering::Greater);
        assert_eq!(compare_tags("1.2.3-alpha", "1.2.3-beta"), Ordering::Less);
    }
}
