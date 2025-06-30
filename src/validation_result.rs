use std::collections::BTreeSet;

use kubewarden_policy_sdk::response::ValidationResponse;

#[derive(Default, Debug, PartialEq, Eq)]
pub(crate) struct PodRejectionReasons {
    pub(crate) registries_not_allowed: BTreeSet<String>,
    pub(crate) tags_not_allowed: BTreeSet<String>,
    pub(crate) images_not_allowed: BTreeSet<String>,
}

impl PodRejectionReasons {
    pub fn is_empty(&self) -> bool {
        self.registries_not_allowed.is_empty()
            && self.tags_not_allowed.is_empty()
            && self.images_not_allowed.is_empty()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PodSpecValidationResult {
    Allowed,
    NotAllowed(PodRejectionReasons),
}

impl From<PodSpecValidationResult> for ValidationResponse {
    fn from(validation_result: PodSpecValidationResult) -> ValidationResponse {
        match validation_result {
            PodSpecValidationResult::Allowed => ValidationResponse {
                accepted: true,
                message: None,
                code: None,
                mutated_object: None,
                audit_annotations: None,
                warnings: None,
            },
            PodSpecValidationResult::NotAllowed(rejection_reasons) => {
                let mut errors = Vec::new();
                if !rejection_reasons.registries_not_allowed.is_empty() {
                    errors.push(format!(
                        "registries not allowed: {}",
                        rejection_reasons
                            .registries_not_allowed
                            .into_iter()
                            .collect::<Vec<String>>()
                            .join(", ")
                    ));
                }
                if !rejection_reasons.tags_not_allowed.is_empty() {
                    errors.push(format!(
                        "tags not allowed: {}",
                        rejection_reasons
                            .tags_not_allowed
                            .into_iter()
                            .collect::<Vec<String>>()
                            .join(", ")
                    ))
                }
                if !rejection_reasons.images_not_allowed.is_empty() {
                    errors.push(format!(
                        "images not allowed: {}",
                        rejection_reasons
                            .images_not_allowed
                            .into_iter()
                            .collect::<Vec<String>>()
                            .join(", ")
                    ))
                }
                ValidationResponse {
                    accepted: false,
                    message: Some(format!(
                        "not allowed, reported errors: {}",
                        errors.join("; ")
                    )),
                    code: None,
                    mutated_object: None,
                    warnings: None,
                    audit_annotations: None,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest]
    #[case::allowed(PodSpecValidationResult::Allowed, vec![])]
    #[case::not_allowed(
        PodSpecValidationResult::NotAllowed(PodRejectionReasons {
            registries_not_allowed: vec!["registry1".to_string()].into_iter().collect(),
            tags_not_allowed: vec!["tag1".to_string()].into_iter().collect(),
            images_not_allowed: vec!["image1".to_string()].into_iter().collect(),
        }),
        vec!["registry1", "tag1", "image1"]
    )]
    fn pod_spec_validation_result_into_validation_response(
        #[case] result: PodSpecValidationResult,
        #[case] expected_error_msgs: Vec<&str>,
    ) {
        let given_result_is_allowed = match &result {
            PodSpecValidationResult::Allowed => true,
            PodSpecValidationResult::NotAllowed(_) => false,
        };

        let validation_response: ValidationResponse = result.into();

        if expected_error_msgs.is_empty() {
            assert!(validation_response.accepted);
            assert!(
                given_result_is_allowed,
                "you were not supposed to set an error message expectation for an accepted result"
            );
        } else {
            assert!(!validation_response.accepted);
            assert!(
                !given_result_is_allowed,
                "you were not supposed to set an error message expectation for an accepted result"
            );

            let rejection_message = validation_response
                .message
                .as_ref()
                .expect("rejection message not found");
            for expected_error_msg in expected_error_msgs {
                assert!(
                    rejection_message.contains(expected_error_msg),
                    "expected error message not found: {expected_error_msg}"
                );
            }
        }
        assert_eq!(validation_response.code, None);
        assert_eq!(validation_response.mutated_object, None);
        assert_eq!(validation_response.audit_annotations, None);
        assert_eq!(validation_response.warnings, None);
    }
}
