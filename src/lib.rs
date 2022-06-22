use guest::prelude::*;
use kubewarden_policy_sdk::wapc_guest as guest;

extern crate kubewarden_policy_sdk as kubewarden;
use kubewarden::{protocol_version_guest, request::ValidationRequest, validate_settings};

extern crate regex;
extern crate url;

use k8s_openapi::api::core::v1 as apicore;

mod settings;
use settings::Settings;

mod image;
use image::Image;

use settings::PodEvaluationResult;

#[no_mangle]
pub extern "C" fn wapc_init() {
    register_function("validate", validate);
    register_function("validate_settings", validate_settings::<Settings>);
    register_function("protocol_version", protocol_version_guest);
}

fn validate(payload: &[u8]) -> CallResult {
    let validation_request: ValidationRequest<Settings> = ValidationRequest::new(payload)?;

    match serde_json::from_value::<apicore::Pod>(validation_request.request.object) {
        Ok(pod) => match validation_request.settings.is_pod_accepted(&pod) {
            PodEvaluationResult::Allowed => kubewarden::accept_request(),
            PodEvaluationResult::NotAllowed(rejection_reasons) => {
                let mut errors = Vec::new();
                if !rejection_reasons.registries_not_allowed.is_empty() {
                    errors.push(format!(
                        "registries not allowed: {}",
                        rejection_reasons.registries_not_allowed.join(", ")
                    ));
                }
                if !rejection_reasons.tags_not_allowed.is_empty() {
                    errors.push(format!(
                        "tags not allowed: {}",
                        rejection_reasons.tags_not_allowed.join(", ")
                    ))
                }
                if !rejection_reasons.images_not_allowed.is_empty() {
                    errors.push(format!(
                        "images not allowed: {}",
                        rejection_reasons.images_not_allowed.join(", ")
                    ))
                }
                kubewarden::reject_request(
                    Some(format!(
                        "not allowed, reported errors: {}",
                        errors.join("; ")
                    )),
                    None,
                    None,
                    None,
                )
            }
        },
        Err(_) => kubewarden::accept_request(),
    }
}
