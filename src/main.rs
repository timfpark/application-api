use controllers::workload_assignment::WorkloadAssignmentController;
use futures::stream::StreamExt;
use kube::Resource;
use kube::ResourceExt;
use kube::{api::ListParams, client::Client, Api};
use kube_runtime::controller::{Context, ReconcilerAction};
use kube_runtime::Controller;
use tokio::time::Duration;

mod controllers;
mod models;
mod workflows;
mod utils;

use controllers::workload_assignment;
use models::workload_assignment::WorkloadAssignment;
use utils::error::Error;
use workflows::gitops::GitopsWorkflow;


#[tokio::main]
async fn main() {
    // First, a Kubernetes client must be obtained using the `kube` crate
    // The client will later be moved to the custom controller
    let kubernetes_client: Client = Client::try_default()
        .await
        .expect("Expected a valid KUBECONFIG environment variable.");

    // Preparation of resources used by the `kube_runtime::Controller`
    let crd_api: Api<WorkloadAssignment> = Api::all(kubernetes_client.clone());
    let context: Context<ContextData> = Context::new(ContextData::new(kubernetes_client.clone()));

    // The controller comes from the `kube_runtime` crate and manages the reconciliation process.
    // It requires the following information:
    // - `kube::Api<T>` this controller "owns". In this case, `T = WorkloadAssignment`, as this controller owns the `WorkloadAssignment` resource,
    // - `kube::api::ListParams` to select the `WorkloadAssignment` resources with. Can be used for WorkloadAssignment filtering `WorkloadAssignment` resources before reconciliation,
    // - `reconcile` function with reconciliation logic to be called each time a resource of `WorkloadAssignment` kind is created/updated/deleted,
    // - `on_error` function to call whenever reconciliation fails.
    Controller::new(crd_api.clone(), ListParams::default())
        .run(reconcile, on_error, context)
        .for_each(|reconciliation_result| async move {
            println!("{:?}", reconciliation_result);
            match reconciliation_result {
                Ok(workload_assignment_resource) => {
                    println!("Reconciliation successful. Resource: {:?}", workload_assignment_resource);
                }
                Err(reconciliation_err) => {
                    eprintln!("Reconciliation error: {:?}", reconciliation_err)
                }
            }
        })
        .await;
}

/// Context injected with each `reconcile` and `on_error` method invocation.
struct ContextData {
    controller: WorkloadAssignmentController
}

impl ContextData {
    /// Constructs a new instance of ContextData.
    ///
    /// # Arguments:
    /// - `client`: A Kubernetes client to make Kubernetes REST API requests with. Resources
    /// will be created and deleted with this client.
    pub fn new(client: Client) -> Self {
        let controller = WorkloadAssignmentController::new(client);
        ContextData { controller }
    }
}

/// Action to be taken upon an `WorkloadAssignment` resource during reconciliation
enum Action {
    /// Create the subresources, this includes spawning `n` pods with WorkloadAssignment service
    Create,
    /// Delete all subresources created in the `Create` phase
    Delete,
    /// This `WorkloadAssignment` resource is in desired state and requires no actions to be taken
    NoOp,
}

async fn reconcile(workload_assignment: WorkloadAssignment, context: Context<ContextData>) -> Result<ReconcilerAction, Error> {
    let workload_assignment_controller = &context.get_ref().controller; // The `Client` is shared -> a clone from the reference is obtained

    // The resource of `WorkloadAssignment` kind is required to have a namespace set. However, it is not guaranteed
    // the resource will have a `namespace` set. Therefore, the `namespace` field on object's metadata
    // is optional and Rust forces the programmer to check for it's existence first.
    let namespace: String = match workload_assignment.namespace() {
        None => {
            // If there is no namespace to deploy to defined, reconciliation ends with an error immediately.
            return Err(Error::UserInputError(
                "Expected WorkloadAssignment resource to be namespaced. Can't deploy to an unknown namespace."
                    .to_owned(),
            ));
        }
        // If namespace is known, proceed. In a more advanced version of the operator, perhaps
        // the namespace could be checked for existence first.
        Some(namespace) => namespace,
    };

    // Performs action as decided by the `determine_action` function.
    return match determine_action(&workload_assignment) {
        Action::Create => {
            println!("Action::Create");
            // Creates a deployment with `n` WorkloadAssignment service pods, but applies a finalizer first.
            // Finalizer is applied first, as the operator might be shut down and restarted
            // at any time, leaving subresources in intermediate state. This prevents leaks on
            // the `WorkloadAssignment` resource deletion.
            let name = workload_assignment.name(); // Name of the WorkloadAssignment resource is used to name the subresources as well.

            // Apply the finalizer first. If that fails, the `?` operator invokes automatic conversion
            // of `kube::Error` to the `Error` defined in this crate.
            workload_assignment_controller.add_finalizer_record(&name, &namespace).await?;

            // Invoke creation of a Kubernetes built-in resource named deployment with `n` WorkloadAssignment service pods.
            workload_assignment_controller.create_deployment(&workload_assignment.name(), &namespace).await?;

            Ok(ReconcilerAction {
                // Finalizer is added, deployment is deployed, re-check in 10 seconds.
                requeue_after: Some(Duration::from_secs(10)),
            })
        }
        Action::Delete => {
            println!("Action::Delete");
            // Deletes any subresources related to this `WorkloadAssignment` resources. If and only if all subresources
            // are deleted, the finalizer is removed and Kubernetes is free to remove the `WorkloadAssignment` resource.

            // First, delete the deployment. If there is any error deleting the deployment, it is
            // automatically converted into `Error` defined in this crate and the reconciliation is ended
            // with that error.

            // Note: A more advanced implementation would check for the Deployment's existence.
            workload_assignment_controller.delete_deployment(&workload_assignment.name(), &namespace).await?;

            // Once the deployment is successfully removed, remove the finalizer to make it possible
            // for Kubernetes to delete the `WorkloadAssignment` resource.
            workload_assignment_controller.delete_finalizer_record(&workload_assignment.name(), &namespace).await?;

            Ok(ReconcilerAction {
                requeue_after: None, // Makes no sense to delete after a successful delete, as the resource is gone
            })
        }
        Action::NoOp => {
            println!("Action::NoOp");

            Ok(ReconcilerAction {
                // The resource is already in desired state, do nothing and re-check after 10 seconds

                requeue_after: Some(Duration::from_secs(10)),
            })
        },
    };
}

/// Resources arrives into reconciliation queue in a certain state. This function looks at
/// the state of given `WorkloadAssignment` resource and decides which actions needs to be performed.
/// The finite set of possible actions is represented by the `Action` enum.
///
/// # Arguments
/// - `workload_assignment`: A reference to `WorkloadAssignment` being reconciled to decide next action upon.
fn determine_action(workload_assignment: &WorkloadAssignment) -> Action {
    return if workload_assignment.meta().deletion_timestamp.is_some() {
        Action::Delete
    } else if workload_assignment.meta().finalizers.is_empty() {
        Action::Create
    } else {
        Action::NoOp
    };
}

/// Actions to be taken when a reconciliation fails - for whatever reason.
/// Prints out the error to `stderr` and requeues the resource for another reconciliation after
/// five seconds.
///
/// # Arguments
/// - `error`: A reference to the `kube::Error` that occurred during reconciliation.
/// - `_context`: Unused argument. Context Data "injected" automatically by kube-rs.
fn on_error(error: &Error, _context: Context<ContextData>) -> ReconcilerAction {
    eprintln!("Reconciliation error:\n{:?}", error);
    ReconcilerAction {
        requeue_after: Some(Duration::from_secs(5)),
    }
}
