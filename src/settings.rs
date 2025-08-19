use std::{collections::HashSet, str::FromStr};

use kubewarden_policy_sdk::settings::Validatable;
use oci_spec::distribution::Reference;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Deserialize, Serialize, Default, Debug)]
#[serde(default)]
pub(crate) struct Registries {
    pub allow: HashSet<String>,
    pub reject: HashSet<String>,
}

impl Registries {
    fn validate(&self) -> Result<(), String> {
        if !self.allow.is_empty() && !self.reject.is_empty() {
            return Err("only one of registries allow or reject can be provided".to_string());
        }
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Default, Debug)]
#[serde(default)]
pub(crate) struct Tags {
    pub reject: HashSet<String>,
}

impl Tags {
    /// Validate the tags against the OCI spec
    fn validate(&self) -> Result<(), String> {
        let invalid_tags: Vec<String> = self
            .reject
            .iter()
            .filter(|tag| Reference::from_str(format!("hello:{tag}").as_str()).is_err())
            .cloned()
            .collect();

        if !invalid_tags.is_empty() {
            return Err(format!(
                "tags {invalid_tags:?} are invalid, they must be valid OCI tags",
            ));
        }

        Ok(())
    }
}

/// Custom type to represent an image reference. It's required to implement
/// the `Deserialize` trait to be able to use it in the `Settings` struct.
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct ImageRef(oci_spec::distribution::Reference);
impl ImageRef {
    pub fn new(reference: oci_spec::distribution::Reference) -> Self {
        ImageRef(reference)
    }

    pub fn repository(&self) -> &str {
        self.0.repository()
    }
    pub fn registry(&self) -> &str {
        self.0.registry()
    }
}

impl From<Reference> for ImageRef {
    fn from(reference: Reference) -> Self {
        ImageRef(reference)
    }
}

impl<'de> Deserialize<'de> for ImageRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        let reference = Reference::from_str(&s).map_err(serde::de::Error::custom)?;

        Ok(ImageRef(reference))
    }
}

impl Serialize for ImageRef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.whole())
    }
}

#[derive(Deserialize, Serialize, Default, Debug)]
#[serde(default)]
pub(crate) struct Images {
    pub allow: HashSet<ImageRef>,
    pub reject: HashSet<ImageRef>,
}

impl Images {
    /// An image cannot be present in both allow and reject lists
    fn validate(&self) -> Result<(), String> {
        if !self.allow.is_empty() && !self.reject.is_empty() {
            return Err("only one of images allow or reject can be provided".to_string());
        }
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Default, Debug)]
#[serde(default)]
pub(crate) struct Settings {
    pub registries: Registries,
    pub tags: Tags,
    pub images: Images,
}

impl Validatable for Settings {
    fn validate(&self) -> Result<(), String> {
        let errors = vec![
            self.registries.validate(),
            self.images.validate(),
            self.tags.validate(),
        ]
        .into_iter()
        .filter_map(Result::err)
        .collect::<Vec<String>>();

        if !errors.is_empty() {
            return Err(errors.join(", "));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest]
    #[case::empty_settings(Vec::new(), Vec::new(), true)]
    #[case::allow_only(vec!["allowed-registry.com".to_string()], Vec::new(), true)]
    #[case::reject_only(Vec::new(), vec!["forbidden-registry.com".to_string()], true)]
    #[case::allow_and_reject(
        vec!["allowed-registry.com".to_string()],
        vec!["forbidden-registry.com".to_string()],
        false
    )]
    fn validate_registries(
        #[case] allow: Vec<String>,
        #[case] reject: Vec<String>,
        #[case] is_valid: bool,
    ) {
        let registries = Registries {
            allow: allow.into_iter().collect(),
            reject: reject.into_iter().collect(),
        };

        let result = registries.validate();
        if is_valid {
            assert!(result.is_ok(), "{result:?}");
        } else {
            assert!(result.is_err(), "was supposed to be invalid");
        }
    }

    #[rstest]
    #[case::empty_settings(Vec::new(), Vec::new(), true)]
    #[case::allow_only(vec!["allowed-image".to_string()], Vec::new(), true)]
    #[case::reject_only(Vec::new(), vec!["forbidden-image".to_string()], true)]
    #[case::allow_and_reject(
        vec!["allowed-image.com".to_string()],
        vec!["forbidden-image.com".to_string()],
        false
    )]
    fn validate_images(
        #[case] allow: Vec<String>,
        #[case] reject: Vec<String>,
        #[case] is_valid: bool,
    ) {
        let images = Images {
            allow: allow
                .iter()
                .map(|image| ImageRef(Reference::from_str(image).unwrap()))
                .collect(),
            reject: reject
                .iter()
                .map(|image| ImageRef(Reference::from_str(image).unwrap()))
                .collect(),
        };

        let result = images.validate();
        if is_valid {
            assert!(result.is_ok(), "{result:?}");
        } else {
            assert!(result.is_err(), "was supposed to be invalid");
        }
    }

    #[rstest]
    #[case::good_input(
        r#"{
            "allow": [],
            "reject": [
                "busybox",
                "busybox:latest",
                "registry.com/image@sha256:3fc9b689459d738f8c88a3a48aa9e33542016b7a4052e001aaa536fca74813cb",
                "quay.io/etcd/etcd:1.1.1@sha256:3fc9b689459d738f8c88a3a48aa9e33542016b7a4052e001aaa536fca74813cb"
            ]
        }"#,
        true
    )]
    #[case::bad_input(
        r#"{
            "allow": [],
            "reject": [
                "busybox",
                "registry.com/image@sha256",
            ]
        }"#,
        false
    )]
    fn deserialize_images(#[case] input: &str, #[case] valid: bool) {
        let image: Result<Images, _> = serde_json::from_str(input);
        if valid {
            assert!(image.is_ok(), "{image:?}");
        } else {
            assert!(image.is_err(), "was supposed to be invalid");
        }
    }

    #[rstest]
    #[case::empty_settings(Vec::new(), true)]
    #[case::valid_tags(vec!["latest".to_string()], true)]
    #[case::invalid_tags(vec!["latest".to_string(), "1.0.0+rc3".to_string()], false)]
    fn validate_tags(#[case] tags: Vec<String>, #[case] is_valid: bool) {
        let tags = Tags {
            reject: tags.into_iter().collect(),
        };

        let result = tags.validate();
        if is_valid {
            assert!(result.is_ok(), "{result:?}");
        } else {
            assert!(result.is_err(), "was supposed to be invalid");
        }
    }

    #[rstest]
    #[case::empty_settings(Settings::default(), true)]
    #[case::valid_settings(
        Settings {
            registries: Registries {
                allow: vec!["registry.com".to_string()].into_iter().collect(),
                ..Registries::default()
            },
            tags: Tags {
                reject: vec!["latest".to_string()].into_iter().collect(),
            },
            images: Images {
                reject: vec!["busybox".to_string()].into_iter().map(|image| Reference::from_str(&image).unwrap().into()).collect(),
                ..Images::default()
            },
        },
        true
    )]
    #[case::bad_registries(
        Settings {
            registries: Registries {
                allow: vec!["registry.com".to_string()].into_iter().collect(),
                reject: vec!["registry2.com".to_string()].into_iter().collect(),
            },
            tags: Tags {
                reject: vec!["latest".to_string()].into_iter().collect(),
            },
            images: Images {
                reject: vec!["busybox".to_string()].into_iter().map(|image| Reference::from_str(&image).unwrap().into()).collect(),
                ..Images::default()
            },
        },
        false
    )]
    fn validate_settings(#[case] settings: Settings, #[case] is_valid: bool) {
        let result = settings.validate();
        if is_valid {
            assert!(result.is_ok(), "{result:?}");
        } else {
            assert!(result.is_err(), "was supposed to be invalid");
        }
    }
}
