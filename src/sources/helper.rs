use regex::Regex;

pub fn filter_based_on_regex(
    tags: Vec<String>,
    version_regex: &Option<String>,
) -> anyhow::Result<Vec<String>> {
    if let Some(regex) = version_regex {
        let re = Regex::new(regex)?;
        Ok(tags.into_iter().filter(|t| re.is_match(t)).collect())
    } else {
        Ok(tags)
    }
}
