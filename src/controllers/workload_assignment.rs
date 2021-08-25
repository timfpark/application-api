use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::core::v1::{Container, ContainerPort, PodSpec, PodTemplateSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;
use kube::api::{DeleteParams, ListParams, ObjectMeta, PostParams, Patch, PatchParams};
use kube::{Api, Client};
use serde_json::{json, Value};
use std::collections::BTreeMap;

use crate::models::workload::Workload;
use crate::models::workload_assignment::WorkloadAssignment;
use crate::utils::error::Error;
use crate::workflows::gitops::GitopsWorkflow;
use crate::workflows::workflow::Workflow;

pub struct WorkloadAssignmentController {
    client: Client,
    workflow: GitopsWorkflow
}

impl WorkloadAssignmentController {
    pub fn new(client: Client) -> Self {
        let workflow = GitopsWorkflow {
            workload_repo_url: "https://github.com/timfpark/workload-gitops".to_string()
        };

        WorkloadAssignmentController { client: client.clone(), workflow }
    }

    /// Adds a finalizer record into an `WorkloadAssignment` kind of resource. If the finalizer already exists,
    /// this action has no effect.
    ///
    /// # Arguments:
    /// - `client` - Kubernetes client to modify the `WorkloadAssignment` resource with.
    /// - `name` - Name of the `WorkloadAssignment` resource to modify. Existence is not verified
    /// - `namespace` - Namespace where the `WorkloadAssignment` resource with given `name` resides.
    ///
    /// Note: Does not check for resource's existence for simplicity.
    pub async fn add_finalizer_record(&self, name: &str, namespace: &str) -> Result<WorkloadAssignment, Error> {
        println!("Workload add_finalizer_record");

        let api: Api<WorkloadAssignment> = Api::namespaced(self.client.clone(), namespace);
        let finalizer: Value = json!({
            "metadata": {
                "finalizers": ["workload-assignments.example.com"]
            }
        });

        let patch: Patch<&Value> = Patch::Merge(&finalizer);
        Ok(api.patch(name, &PatchParams::default(), &patch).await?)
    }

    /// Deploy the Workload on the Cluster specified by the WorkloadAssignment.
    ///
    /// # Arguments
    /// - `client` - A Kubernetes client to create the deployment with.
    /// - `name` - Name of the deployment to be created
    /// - `replicas` - Number of pod replicas for the Deployment to contain
    /// - `namespace` - Namespace to create the Kubernetes Deployment in.
    ///
    /// Note: It is assumed the resource does not already exists for simplicity. Returns an `Error` if it does.
    pub async fn create_deployment(&self, name: &str, namespace: &str) -> Result<(), Error> {
        println!("Workload create_deployment");

        let workload_api: Api<Workload> = Api::namespaced(self.client.clone(), namespace);
        let workload_assignment_api: Api<WorkloadAssignment> = Api::namespaced(self.client.clone(), namespace);

        let workload_assignment = workload_assignment_api.get(name).await?;
        println!("{:?}", workload_assignment);

        let workload = workload_api.get(&workload_assignment.spec.workload).await?;
        println!("{:?}", workload);

        self.workflow.create_deployment(&workload, &workload_assignment).await?;

        Ok(())
    }

    /// Deletes an existing deployment.
    ///
    /// # Arguments:
    /// - `client` - A Kubernetes client to delete the Deployment with
    /// - `name` - Name of the deployment to delete
    /// - `namespace` - Namespace the existing deployment resides in
    ///
    /// Note: It is assumed the deployment exists for simplicity. Otherwise returns an Error.
    pub async fn delete_deployment(&self, name: &str, namespace: &str) -> Result<(), Error> {
        println!("Workload delete_deployment");

        let workload_api: Api<Workload> = Api::namespaced(self.client.clone(), namespace);
        let workload_assignment_api: Api<WorkloadAssignment> = Api::namespaced(self.client.clone(), namespace);

        let workload_assignment = workload_assignment_api.get(name).await?;
        println!("{:?}", workload_assignment);

        let workload = workload_api.get(&workload_assignment.spec.workload).await?;
        println!("{:?}", workload);

        self.workflow.delete_deployment(&workload, &workload_assignment).await?;

        Ok(())
    }

    /// Removes all finalizers from an `WorkloadAssignment` resource. If there are no finalizers already, this
    /// action has no effect.
    ///
    /// # Arguments:
    /// - `client` - Kubernetes client to modify the `WorkloadAssignment` resource with.
    /// - `name` - Name of the `WorkloadAssignment` resource to modify. Existence is not verified
    /// - `namespace` - Namespace where the `WorkloadAssignment` resource with given `name` resides.
    ///
    /// Note: Does not check for resource's existence for simplicity.
    pub async fn delete_finalizer_record(&self, name: &str, namespace: &str) -> Result<WorkloadAssignment, Error> {
        println!("Workload delete_finalizer_record");

        let api: Api<WorkloadAssignment> = Api::namespaced(self.client.clone(), namespace);
        let finalizer: Value = json!({
            "metadata": {
                "finalizers": null
            }
        });

        let patch: Patch<&Value> = Patch::Merge(&finalizer);
        Ok(api.patch(name, &PatchParams::default(), &patch).await?)
    }
}







