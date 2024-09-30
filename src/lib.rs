use guest::prelude::*;
use k8s_openapi::api::{
    apps::v1::{DaemonSet, Deployment, ReplicaSet, StatefulSet},
    batch::v1::{CronJob, Job},
    core::v1::{Pod, ReplicationController},
};
use kubewarden_policy_sdk::{
    accept_request, logging, protocol_version_guest, request::ValidationRequest, validate_settings,
};
use kubewarden_policy_sdk::{response::ValidationResponse, wapc_guest as guest};
use lazy_static::lazy_static;
use serde::de::DeserializeOwned;
use slog::{o, warn, Logger};

mod validation_result;

mod validation;
use validation::validate_pod_spec;

mod validating_resource;
use validating_resource::ValidatingResource;

mod settings;
use settings::Settings;

lazy_static! {
    static ref LOG_DRAIN: Logger = Logger::root(
        logging::KubewardenDrain::new(),
        o!("policy" => "trusted-repos")
    );
}

#[no_mangle]
pub extern "C" fn wapc_init() {
    register_function("validate", validate);
    register_function("validate_settings", validate_settings::<Settings>);
    register_function("protocol_version", protocol_version_guest);
}

fn validate(payload: &[u8]) -> CallResult {
    let validation_request: ValidationRequest<Settings> = ValidationRequest::new(payload)?;

    match validation_request.request.kind.kind.as_str() {
        "Deployment" => validate_resource::<Deployment>(validation_request),
        "ReplicaSet" => validate_resource::<ReplicaSet>(validation_request),
        "StatefulSet" => validate_resource::<StatefulSet>(validation_request),
        "DaemonSet" => validate_resource::<DaemonSet>(validation_request),
        "ReplicationController" => validate_resource::<ReplicationController>(validation_request),
        "Job" => validate_resource::<Job>(validation_request),
        "CronJob" => validate_resource::<CronJob>(validation_request),
        "Pod" => validate_resource::<Pod>(validation_request),
        _ => {
            // We were forwarded a request we cannot unmarshal or
            // understand, just accept it
            warn!(LOG_DRAIN, "cannot unmarshal resource: this policy does not know how to evaluate this resource; accept it");
            accept_request()
        }
    }
}

// validate any resource that contains a Pod. e.g. Deployment, StatefulSet, ...
fn validate_resource<T: ValidatingResource + DeserializeOwned>(
    validation_request: ValidationRequest<Settings>,
) -> CallResult {
    let resource = match serde_json::from_value::<T>(validation_request.request.object.clone()) {
        Ok(resource) => resource,
        Err(_) => {
            // We were forwarded a request we cannot unmarshal or
            // understand, just accept it
            warn!(LOG_DRAIN, "cannot unmarshal resource: this policy does not know how to evaluate this resource; accept it");
            return accept_request();
        }
    };

    let spec = match resource.spec() {
        Some(spec) => spec,
        None => {
            return accept_request();
        }
    };

    let validation_response: ValidationResponse =
        validate_pod_spec(&spec, &validation_request.settings).into();
    Ok(serde_json::to_vec(&validation_response)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::settings::Registries;

    use kubewarden_policy_sdk::test::Testcase;
    use rstest::*;

    #[rstest]
    // Note: this test cares only about covering the switch statement of the resournce kind
    #[case::deployment("test_data/deployment_creation.json", false)]
    #[case::replicaset("test_data/replicaset_creation.json", false)]
    #[case::statefulset("test_data/statefulset_creation.json", false)]
    #[case::daemonset("test_data/daemonset_creation.json", false)]
    #[case::replicationcontroller("test_data/replicationcontroller_creation.json", false)]
    #[case::job("test_data/job_creation.json", false)]
    #[case::cronjob("test_data/cronjob_creation.json", false)]
    #[case::pod("test_data/pod_creation.json", false)]
    #[case::ingress("test_data/ingress_creation.json", true)]
    fn test_validate(#[case] fixture: &str, #[case] expected_validation_result: bool) {
        let settings = Settings {
            registries: Registries {
                reject: vec!["ghcr.io".to_string(), "docker.io".to_string()]
                    .into_iter()
                    .collect(),
                ..Default::default()
            },

            ..Default::default()
        };

        let test_case = Testcase {
            name: "test_validate".to_string(),
            fixture_file: fixture.to_string(),
            settings,
            expected_validation_result,
        };

        assert!(test_case.eval(validate).is_ok());
    }
}
