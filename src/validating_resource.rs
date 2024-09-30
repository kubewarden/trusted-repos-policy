use k8s_openapi::api::{
    apps::v1::{DaemonSet, Deployment, ReplicaSet, StatefulSet},
    batch::v1::{CronJob, Job},
    core::v1::{Pod, PodSpec, ReplicationController},
};

/// Represents all resources that can be validated with this policy
pub trait ValidatingResource {
    fn spec(&self) -> Option<PodSpec>;
}

impl ValidatingResource for Pod {
    fn spec(&self) -> Option<PodSpec> {
        self.spec.clone()
    }
}

impl ValidatingResource for Deployment {
    fn spec(&self) -> Option<PodSpec> {
        self.spec.as_ref()?.template.spec.clone()
    }
}

impl ValidatingResource for ReplicaSet {
    fn spec(&self) -> Option<PodSpec> {
        self.spec.as_ref()?.template.as_ref()?.spec.clone()
    }
}

impl ValidatingResource for StatefulSet {
    fn spec(&self) -> Option<PodSpec> {
        self.spec.as_ref()?.template.spec.clone()
    }
}

impl ValidatingResource for DaemonSet {
    fn spec(&self) -> Option<PodSpec> {
        self.spec.as_ref()?.template.spec.clone()
    }
}

impl ValidatingResource for ReplicationController {
    fn spec(&self) -> Option<PodSpec> {
        self.spec.as_ref()?.template.as_ref()?.spec.clone()
    }
}

impl ValidatingResource for Job {
    fn spec(&self) -> Option<PodSpec> {
        self.spec.as_ref()?.template.spec.clone()
    }
}

impl ValidatingResource for CronJob {
    fn spec(&self) -> Option<PodSpec> {
        self.spec
            .as_ref()?
            .job_template
            .spec
            .as_ref()?
            .template
            .spec
            .clone()
    }
}
