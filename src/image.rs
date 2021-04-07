use anyhow::{anyhow, Result};
use core::fmt::Display;
use regex::Regex;
use url::{Host, Url};

#[derive(Default, Debug)]
pub(crate) struct Image {
    pub(crate) image: String,
    pub(crate) registry: Option<String>,
    pub(crate) name: String,
    pub(crate) tag: Option<String>,
    pub(crate) sha256: Option<String>,
}

impl ToString for Image {
    fn to_string(&self) -> String {
        format!(
            "{}{}{}{}",
            self.registry
                .as_ref()
                .map(|registry| format!("{}/", registry))
                .unwrap_or_default(),
            self.name,
            self.tag
                .as_ref()
                .map(|tag| format!(":{}", tag))
                .unwrap_or_default(),
            self.sha256
                .as_ref()
                .map(|sha256| format!("@sha256:{}", sha256))
                .unwrap_or_default(),
        )
    }
}

impl PartialEq<Image> for String {
    fn eq(&self, other: &Image) -> bool {
        self == &other.image
    }
}

impl PartialEq<String> for Image {
    fn eq(&self, other: &String) -> bool {
        &self.image == other
    }
}

impl Image {
    pub(crate) fn new<T>(image: T) -> Result<Image>
    where
        T: Into<String> + Display + Copy + Clone,
    {
        let orig_image = image.clone().into();
        let image_with_scheme = format!("registry://{}", image);
        let url = Url::parse(&image_with_scheme);

        let image_has_slash = image.into().chars().any(|c| c == '/');

        let parse_image_reference = if image_has_slash {
            Regex::new(
                r"^(registry://)?([^/]+/)*(?P<image>[^:@]+)(:(?P<tag>[^@]+))?(@sha256:(?P<sha256>[A-Fa-f0-9]{64}))?$"
            ).unwrap()
        } else {
            Regex::new(
                r"^(?P<image>[^:@]+)(:(?P<tag>[^@]+))?(@sha256:(?P<sha256>[A-Fa-f0-9]{64}))?$",
            )
            .unwrap()
        };

        let registry = if image_has_slash {
            url.clone()
                .and_then(|url| {
                    url.host()
                        .map(|host| match host {
                            Host::Domain(domain) => domain.into(),
                            Host::Ipv4(address) => format!("{}", address),
                            Host::Ipv6(address) => format!("{}", address),
                        })
                        .ok_or(url::ParseError::EmptyHost)
                })
                .and_then(|host| {
                    url.clone().map(|url| {
                        url.port()
                            .map_or(host.clone(), |port| format!("{}:{}", host, port))
                    })
                })
        } else {
            Ok("docker.io".into())
        };

        parse_image_reference
            .captures(&image.into())
            .map(|captures| {
                (
                    captures.name("image").map(|image| image.as_str()),
                    captures.name("tag").map(|tag| tag.as_str()),
                    captures.name("sha256").map(|sha256| sha256.as_str()),
                )
            })
            .map(|(image, tag, sha256)| Image {
                image: orig_image,
                name: String::from(image.unwrap_or_default()),
                registry: registry.ok(),
                tag: tag.map(|tag| tag.to_string()),
                sha256: sha256.map(|sha256| sha256.to_string()),
            })
            .ok_or_else(|| anyhow!("could not parse {} as an image", &image))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_comparison() -> Result<()> {
        let image = Image::new("image")?;
        assert_eq!(image, "image".to_string());

        let image = Image::new("path/to/image")?;
        assert_eq!(image, "path/to/image".to_string());

        let image = Image::new("image:tag")?;
        assert_eq!(image, "image:tag".to_string());

        let image = Image::new("path/to/image:tag")?;
        assert_eq!(image, "path/to/image:tag".to_string());

        let image = Image::new("example.com/image")?;
        assert_eq!(image, "example.com/image".to_string());

        let image = Image::new("example.com/path/to/image")?;
        assert_eq!(image, "example.com/path/to/image".to_string());

        let image = Image::new("example.com/image:tag")?;
        assert_eq!(image, "example.com/image:tag".to_string());

        let image = Image::new("example.com/path/to/image:tag")?;
        assert_eq!(image, "example.com/path/to/image:tag".to_string());

        let image = Image::new("example.com:5000/image")?;
        assert_eq!(image, "example.com:5000/image".to_string());

        let image = Image::new("example.com:5000/path/to/image")?;
        assert_eq!(image, "example.com:5000/path/to/image".to_string());

        let image = Image::new("example.com:5000/image:tag")?;
        assert_eq!(image, "example.com:5000/image:tag".to_string());

        let image = Image::new("example.com:5000/path/to/image:tag")?;
        assert_eq!(image, "example.com:5000/path/to/image:tag".to_string());

        let image = Image::new("10.0.0.100/image")?;
        assert_eq!(image, "10.0.0.100/image".to_string());

        let image = Image::new("10.0.0.100/path/to/image")?;
        assert_eq!(image, "10.0.0.100/path/to/image".to_string());

        let image = Image::new("10.0.0.100/image:tag")?;
        assert_eq!(image, "10.0.0.100/image:tag".to_string());

        let image = Image::new("10.0.0.100/path/to/image:tag")?;
        assert_eq!(image, "10.0.0.100/path/to/image:tag".to_string());

        let image = Image::new("10.0.0.100:5000/image")?;
        assert_eq!(image, "10.0.0.100:5000/image".to_string());

        let image = Image::new("10.0.0.100:5000/path/to/image")?;
        assert_eq!(image, "10.0.0.100:5000/path/to/image".to_string());

        let image = Image::new("10.0.0.100:5000/image:tag")?;
        assert_eq!(image, "10.0.0.100:5000/image:tag".to_string());

        let image = Image::new("10.0.0.100:5000/path/to/image:tag")?;
        assert_eq!(image, "10.0.0.100:5000/path/to/image:tag".to_string());

        let image = Image::new("example.com/image:tag@sha256:3fc9b689459d738f8c88a3a48aa9e33542016b7a4052e001aaa536fca74813cb")?;
        assert_eq!(image, "example.com/image:tag@sha256:3fc9b689459d738f8c88a3a48aa9e33542016b7a4052e001aaa536fca74813cb".to_string());

        let image = Image::new("example.com/path/to/image:tag@sha256:3fc9b689459d738f8c88a3a48aa9e33542016b7a4052e001aaa536fca74813cb")?;
        assert_eq!(image, "example.com/path/to/image:tag@sha256:3fc9b689459d738f8c88a3a48aa9e33542016b7a4052e001aaa536fca74813cb".to_string());

        let image = Image::new("example.com/image@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049")?;
        assert_eq!(image, "example.com/image@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049".to_string());

        let image = Image::new("example.com/path/to/image@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049")?;
        assert_eq!(image, "example.com/path/to/image@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049".to_string());

        let image = Image::new("example.com/image:tag@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049")?;
        assert_eq!(image, "example.com/image:tag@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049".to_string());

        let image = Image::new("example.com/path/to/image:tag@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049")?;
        assert_eq!(image, "example.com/path/to/image:tag@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049".to_string());

        let image = Image::new("example.com:5000/image@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049")?;
        assert_eq!(image, "example.com:5000/image@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049".to_string());

        let image = Image::new("example.com:5000/path/to/image@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049")?;
        assert_eq!(image, "example.com:5000/path/to/image@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049".to_string());

        let image = Image::new("example.com:5000/image:tag@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049")?;
        assert_eq!(image, "example.com:5000/image:tag@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049".to_string());

        let image = Image::new("example.com:5000/path/to/image:tag@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049")?;
        assert_eq!(image, "example.com:5000/path/to/image:tag@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049".to_string());

        let image = Image::new("10.0.0.100/image@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049")?;
        assert_eq!(image, "10.0.0.100/image@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049".to_string());

        let image = Image::new("10.0.0.100/path/to/image@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049")?;
        assert_eq!(image, "10.0.0.100/path/to/image@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049".to_string());

        let image = Image::new("10.0.0.100/image:tag@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049")?;
        assert_eq!(image, "10.0.0.100/image:tag@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049".to_string());

        let image = Image::new("10.0.0.100/path/to/image:tag@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049")?;
        assert_eq!(image, "10.0.0.100/path/to/image:tag@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049".to_string());

        let image = Image::new("10.0.0.100:5000/image@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049")?;
        assert_eq!(image, "10.0.0.100:5000/image@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049".to_string());

        let image = Image::new("10.0.0.100:5000/path/to/image@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049")?;
        assert_eq!(image, "10.0.0.100:5000/path/to/image@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049".to_string());

        let image = Image::new("10.0.0.100:5000/image:tag@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049")?;
        assert_eq!(image, "10.0.0.100:5000/image:tag@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049".to_string());

        let image = Image::new("10.0.0.100:5000/path/to/image:tag@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049")?;
        assert_eq!(image, "10.0.0.100:5000/path/to/image:tag@sha256:73475cb40a568e8da8a045ced110137e159f890ac4da883b6b17dc651b3a8049".to_string());

        Ok(())
    }

    #[test]
    fn parse_host() -> Result<()> {
        let image = Image::new("example.com/image:tag")?;
        assert_eq!(image.registry, Some("example.com".into()));

        let image = Image::new("example.com/path/to/image:tag")?;
        assert_eq!(image.registry, Some("example.com".into()));

        let image = Image::new("example.com:5000/image:tag")?;
        assert_eq!(image.registry, Some("example.com:5000".into()));

        let image = Image::new("example.com:5000/path/to/image:tag")?;
        assert_eq!(image.registry, Some("example.com:5000".into()));

        let image = Image::new("10.0.0.100/image:tag")?;
        assert_eq!(image.registry, Some("10.0.0.100".into()));

        let image = Image::new("10.0.0.100/path/to/image:tag")?;
        assert_eq!(image.registry, Some("10.0.0.100".into()));

        let image = Image::new("10.0.0.100:5000/image:tag")?;
        assert_eq!(image.registry, Some("10.0.0.100:5000".into()));

        let image = Image::new("10.0.0.100:5000/path/to/image:tag")?;
        assert_eq!(image.registry, Some("10.0.0.100:5000".into()));

        Ok(())
    }

    #[test]
    fn parse_image() -> Result<()> {
        let image = Image::new("image")?;
        assert_eq!(image.name, "image");

        let image = Image::new("image:tag")?;
        assert_eq!(image.name, "image");

        let image = Image::new("example.com/image")?;
        assert_eq!(image.name, "image");

        let image = Image::new("example.com/image:tag")?;
        assert_eq!(image.name, "image");

        let image = Image::new("example.com:5000/image")?;
        assert_eq!(image.name, "image");

        let image = Image::new("example.com:5000/image:tag")?;
        assert_eq!(image.name, "image");

        let image = Image::new("10.0.0.100/image")?;
        assert_eq!(image.name, "image");

        let image = Image::new("10.0.0.100/image:tag")?;
        assert_eq!(image.name, "image");

        let image = Image::new("10.0.0.100:5000/image")?;
        assert_eq!(image.name, "image");

        let image = Image::new("10.0.0.100:5000/image:tag")?;
        assert_eq!(image.name, "image");

        Ok(())
    }
}
