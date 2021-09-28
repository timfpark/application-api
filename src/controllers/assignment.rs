use kube::api::{Patch, PatchParams};
use kube::{Api, Client};
use log::debug;
use serde_json::{json, Value};

use crate::models::application::Application;
use crate::models::assignment::ApplicationAssignment;
use crate::utils::error::Error;
use crate::workflows::gitops::GitopsWorkflow;

pub struct ApplicationAssignmentController {
    client: Client,
    workflow: GitopsWorkflow,
}

impl ApplicationAssignmentController {
    pub fn new(client: Client) -> Self {
        // TODO: need mechanism to configure downstream cluster gitops repo
        let workflow =
            GitopsWorkflow::new("git@github.com:timfpark/workload-cluster-gitops").unwrap();

        ApplicationAssignmentController {
            client: client.clone(),
            workflow,
        }
    }

    /// Adds a finalizer record into an `ApplicationAssignment` kind of resource. If the finalizer already exists,
    /// this action has no effect.
    ///
    /// # Arguments:
    /// - `client` - Kubernetes client to modify the `ApplicationAssignment` resource with.
    /// - `name` - Name of the `ApplicationAssignment` resource to modify. Existence is not verified
    /// - `namespace` - Namespace where the `ApplicationAssignment` resource with given `name` resides.
    ///
    /// Note: Does not check for resource's existence for simplicity.
    pub async fn add_finalizer_record(
        &self,
        name: &str,
        namespace: &str,
    ) -> Result<ApplicationAssignment, Error> {
        debug!("Application add_finalizer_record");

        let api: Api<ApplicationAssignment> = Api::namespaced(self.client.clone(), namespace);
        let finalizer: Value = json!({
            "metadata": {
                "finalizers": ["application-assignment.microsoft.com"]
            }
        });

        let patch: Patch<&Value> = Patch::Merge(&finalizer);
        Ok(api.patch(name, &PatchParams::default(), &patch).await?)
    }

    /// Deploy the Application on the Cluster specified by the ApplicationAssignment.
    ///
    /// # Arguments
    /// - `client` - A Kubernetes client to create the deployment with.
    /// - `name` - Name of the deployment to be created
    /// - `replicas` - Number of pod replicas for the Deployment to contain
    /// - `namespace` - Namespace to create the Kubernetes Deployment in.
    ///
    /// Note: It is assumed the resource does not already exists for simplicity. Returns an `Error` if it does.
    pub async fn create_deployment(&self, name: &str, namespace: &str) -> Result<(), Error> {
        debug!("Application create_deployment");

        let application_api: Api<Application> = Api::namespaced(self.client.clone(), namespace);
        let application_assignment_api: Api<ApplicationAssignment> =
            Api::namespaced(self.client.clone(), namespace);

        let application_assignment = application_assignment_api.get(name).await?;
        debug!("{:?}", application_assignment);

        let application = application_api
            .get(&application_assignment.spec.application)
            .await?;
        debug!("{:?}", application);

        self.workflow
            .create_deployment(&application, &application_assignment)?;

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
        debug!("Application delete_deployment");

        let application_assignment_api: Api<ApplicationAssignment> =
            Api::namespaced(self.client.clone(), namespace);

        let application_assignment = application_assignment_api.get(name).await?;
        debug!("{:?}", application_assignment);

        self.workflow.delete_deployment(&application_assignment)?;

        Ok(())
    }

    /// Removes all finalizers from an `ApplicationAssignment` resource. If there are no finalizers already, this
    /// action has no effect.
    ///
    /// # Arguments:
    /// - `client` - Kubernetes client to modify the `ApplicationAssignment` resource with.
    /// - `name` - Name of the `ApplicationAssignment` resource to modify. Existence is not verified
    /// - `namespace` - Namespace where the `ApplicationAssignment` resource with given `name` resides.
    ///
    /// Note: Does not check for resource's existence for simplicity.
    pub async fn delete_finalizer_record(
        &self,
        name: &str,
        namespace: &str,
    ) -> Result<ApplicationAssignment, Error> {
        debug!("Application delete_finalizer_record");

        let api: Api<ApplicationAssignment> = Api::namespaced(self.client.clone(), namespace);
        let finalizer: Value = json!({
            "metadata": {
                "finalizers": null
            }
        });

        let patch: Patch<&Value> = Patch::Merge(&finalizer);
        Ok(api.patch(name, &PatchParams::default(), &patch).await?)
    }
}
