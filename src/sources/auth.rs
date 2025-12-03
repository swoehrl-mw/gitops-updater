use std::collections::HashMap;

pub fn auth_from_docker_config() -> anyhow::Result<HashMap<String, String>> {
    let mut auth = HashMap::new();
    let Some(homedir) = homedir::my_home()? else {
        return Ok(auth);
    };
    let docker_config = homedir.join(".docker").join("config.json");
    if !docker_config.exists() {
        return Ok(auth);
    }
    let auth_data = std::fs::read_to_string(&docker_config)?;
    let config: serde_json::Value = serde_json::from_str(&auth_data)?;
    let Some(auths) = config.get("auths").and_then(|a| a.as_object()) else {
        return Ok(auth);
    };
    for (registry, data) in auths.iter() {
        // Handle https://index.docker.io/v1/ and the like
        let registry = if let Some((_, right)) = registry.split_once("://") {
            right
                .split_once('/')
                .map(|r| r.0)
                .unwrap_or(registry)
                .to_owned()
        } else {
            registry.to_owned()
        };
        let Some(auth_info) = data.get("auth").and_then(|a| a.as_str()) else {
            continue;
        };
        auth.insert(registry.to_owned(), auth_info.to_owned());
    }
    Ok(auth)
}
