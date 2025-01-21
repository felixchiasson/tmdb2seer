use secrecy::Secret;
use serde::Deserialize;

pub fn deserialize_secret_string<'de, D>(deserializer: D) -> Result<Secret<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    String::deserialize(deserializer).map(Secret::new)
}
