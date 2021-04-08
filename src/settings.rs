use k8s_openapi::api::core::v1 as apicore;
use serde::{Deserialize, Serialize};

use kubewarden::settings::Validatable;

use crate::Image;

#[derive(Serialize, Deserialize, Default, Debug)]
pub(crate) struct Settings {
    registries: Option<Registries>,
    tags: Option<Tags>,
    images: Option<Images>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub(crate) struct Registries {
    allow: Option<Vec<String>>,
    reject: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub(crate) struct Tags {
    reject: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub(crate) struct Images {
    allow: Option<Vec<String>>,
    reject: Option<Vec<String>>,
}

#[derive(Default)]
pub(crate) struct PodRejectionReasons {
    pub(crate) registries_not_allowed: Vec<String>,
    pub(crate) tags_not_allowed: Vec<String>,
    pub(crate) images_not_allowed: Vec<String>,
}

impl PodRejectionReasons {
    fn is_empty(&self) -> bool {
        self.registries_not_allowed.is_empty()
            && self.tags_not_allowed.is_empty()
            && self.images_not_allowed.is_empty()
    }
}

pub(crate) enum PodEvaluationResult {
    Allowed,
    NotAllowed(PodRejectionReasons),
}

impl Validatable for Settings {
    fn validate(&self) -> Result<(), String> {
        if let Some(registries) = &self.registries {
            if registries.allow.is_some() == registries.reject.is_some() {
                return Err("only one of registries allow or reject can be provided, and one must be provided".to_string());
            }
        }
        if let Some(images) = &self.images {
            if images.allow.is_some() == images.reject.is_some() {
                return Err(
                    "only one of images allow or reject can be provided, and one must be provided"
                        .to_string(),
                );
            }
        }
        Ok(())
    }
}

impl Settings {
    pub(crate) fn is_pod_accepted(&self, pod: &apicore::Pod) -> PodEvaluationResult {
        let mut rejection_reasons = PodRejectionReasons::default();

        pod.spec
            .as_ref()
            .map(|pod_spec| {
                let empty_containers = Vec::new();
                let init_containers = pod_spec
                    .init_containers
                    .as_ref()
                    .unwrap_or(&empty_containers);
                let containers = &pod_spec.containers;

                vec![init_containers, containers]
                    .iter()
                    .for_each(|containers| {
                        containers.iter().for_each(|container| {
                            if let Some(container_image) = &container.image {
                                let image = Image::new(container_image);
                                if let Ok(image) = image {
                                    if let Some(registry) = &image.registry {
                                        if !self.is_allowed_registry(&registry) {
                                            rejection_reasons
                                                .registries_not_allowed
                                                .push(registry.clone())
                                        }
                                    }
                                    if let Some(tag) = &image.tag {
                                        if !self.is_allowed_tag(&tag) {
                                            rejection_reasons.tags_not_allowed.push(tag.clone());
                                        }
                                    }
                                    if !self.is_allowed_image(&image) {
                                        rejection_reasons.images_not_allowed.push(image.image);
                                    }
                                }
                            }
                        });
                    });

                if rejection_reasons.is_empty() {
                    PodEvaluationResult::Allowed
                } else {
                    PodEvaluationResult::NotAllowed(rejection_reasons)
                }
            })
            .unwrap_or(PodEvaluationResult::Allowed)
    }

    fn is_allowed_registry(&self, registry: &str) -> bool {
        self.registries
            .as_ref()
            .map(|registries| {
                if let Some(allowed_registries) = &registries.allow {
                    allowed_registries
                        .iter()
                        .any(|allowed_registry| registry == allowed_registry)
                } else if let Some(rejected_registries) = &registries.reject {
                    !rejected_registries
                        .iter()
                        .any(|rejected_registry| registry == rejected_registry)
                } else {
                    true
                }
            })
            .or(Some(true))
            .unwrap_or(false)
    }

    fn is_allowed_tag(&self, tag: &str) -> bool {
        self.tags
            .as_ref()
            .map(|tags| {
                if let Some(rejected_tags) = &tags.reject {
                    !rejected_tags.iter().any(|rejected_tag| tag == rejected_tag)
                } else {
                    true
                }
            })
            .or(Some(true))
            .unwrap_or(false)
    }

    fn is_allowed_image(&self, image: &Image) -> bool {
        self.images
            .as_ref()
            .map(|images| {
                if let Some(allowed_images) = &images.allow {
                    allowed_images
                        .iter()
                        .any(|allowed_image| allowed_image == image)
                } else if let Some(rejected_images) = &images.reject {
                    !rejected_images
                        .iter()
                        .any(|rejected_image| rejected_image == image)
                } else {
                    true
                }
            })
            .or(Some(true))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_allowed_registry() {
        let settings: Settings = Default::default();
        assert!(settings.is_allowed_registry(&String::from("docker.io")));

        let settings = Settings {
            registries: Some(Registries {
                allow: Some(vec![String::from("allowed-registry.com")]),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(settings.is_allowed_registry(&String::from("allowed-registry.com")));
        assert!(!settings.is_allowed_registry(&String::from("allowed-registry.com:5001")));
        assert!(!settings.is_allowed_registry(&String::from("docker.io")));

        let settings = Settings {
            registries: Some(Registries {
                reject: Some(vec![String::from("forbidden-registry.com")]),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(settings.is_allowed_registry(&String::from("docker.io")));
        assert!(settings.is_allowed_registry(&String::from("non-forbidden-registry.com:5001")));
        assert!(!settings.is_allowed_registry(&String::from("forbidden-registry.com")));
    }

    #[test]
    fn test_is_allowed_tag() {
        let settings: Settings = Default::default();
        assert!(settings.is_allowed_tag(&String::from("latest")));

        let settings = Settings {
            tags: Some(Tags {
                reject: Some(vec![String::from("latest")]),
            }),
            ..Default::default()
        };
        assert!(!settings.is_allowed_tag(&String::from("latest")));
    }

    #[test]
    fn is_allowed_image() {
        let image = Image::new("example.com/image:tag@sha256:3fc9b689459d738f8c88a3a48aa9e33542016b7a4052e001aaa536fca74813cb");
        println!("{:?}", image);

        let image = Image::new("image:tag");
        println!("{:?}", image);

        let image = Image::new("registry.com/image:tag");
        println!("{:?}", image);

        let image = Image::new("registry.com/image@sha256:3fc9b689459d738f8c88a3a48aa9e33542016b7a4052e001aaa536fca74813cb");
        println!("{:?}", image);

        let image = Image::new("quay.io/etcd/etcd:1.1.1@sha256:3fc9b689459d738f8c88a3a48aa9e33542016b7a4052e001aaa536fca74813cb");
        println!("{:?}", image);
        let image = Image::new("quay.io/etcd/etcd@sha256:3fc9b689459d738f8c88a3a48aa9e33542016b7a4052e001aaa536fca74813cb");
        println!("{:?}", image);
        let image = Image::new("redis:v1.2");
        println!("{:?}", image);
    }

    #[test]
    fn valid_allowed_registries() {
        assert_eq!(
            Settings {
                registries: Some(Registries {
                    allow: Some(vec!("allowed-registry.com".to_string())),
                    reject: None,
                },),
                tags: None,
                images: None,
            }
            .validate(),
            Ok(()),
        );
    }

    #[test]
    fn valid_rejected_registries() {
        assert_eq!(
            Settings {
                registries: Some(Registries {
                    allow: None,
                    reject: Some(vec!("rejected-registry.com".to_string())),
                },),
                tags: None,
                images: None,
            }
            .validate(),
            Ok(()),
        );
    }

    #[test]
    fn invalid_allowed_and_rejected_registries() {
        assert_eq!(
            Settings {
                registries: Some(Registries {
                    allow: Some(vec!("allowed-registry.com".to_string())),
                    reject: Some(vec!("rejected-registry.com".to_string())),
                },),
                tags: None,
                images: None,
            }
            .validate(),
            Err(
                "only one of registries allow or reject can be provided, and one must be provided"
                    .to_string()
            ),
        );
    }

    #[test]
    fn invalid_none_allowed_nor_rejected_registries() {
        assert_eq!(
            Settings {
                registries: Some(Registries {
                    allow: None,
                    reject: None,
                },),
                tags: None,
                images: None,
            }
            .validate(),
            Err(
                "only one of registries allow or reject can be provided, and one must be provided"
                    .to_string()
            ),
        );
    }

    #[test]
    fn valid_allowed_images() {
        assert_eq!(
            Settings {
                registries: None,
                tags: None,
                images: Some(Images {
                    allow: Some(vec!("some-registry.com/some/allowed/image:tag".to_string())),
                    reject: None,
                },),
            }
            .validate(),
            Ok(()),
        );
    }

    #[test]
    fn valid_rejected_images() {
        assert_eq!(
            Settings {
                registries: None,
                tags: None,
                images: Some(Images {
                    allow: None,
                    reject: Some(vec!("some-registry.com/some/rejected/image:tag".to_string())),
                },),
            }
            .validate(),
            Ok(()),
        );
    }

    #[test]
    fn invalid_allowed_and_rejected_images() {
        assert_eq!(
            Settings {
                registries: None,
                tags: None,
                images: Some(Images {
                    allow: Some(vec!("some-registry.com/some/allowed/image:tag".to_string())),
                    reject: Some(vec!("some-registry.com/some/rejected/image:tag".to_string())),
                },),
            }
            .validate(),
            Err(
                "only one of images allow or reject can be provided, and one must be provided"
                    .to_string()
            ),
        );
    }

    #[test]
    fn invalid_none_allowed_nor_rejected_images() {
        assert_eq!(
            Settings {
                registries: None,
                tags: None,
                images: Some(Images {
                    allow: None,
                    reject: None,
                },),
            }
            .validate(),
            Err(
                "only one of images allow or reject can be provided, and one must be provided"
                    .to_string()
            ),
        );
    }
}
