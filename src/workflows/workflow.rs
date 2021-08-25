use kube::{Error};

use crate::models::workload::Workload;
use crate::models::workload_assignment::WorkloadAssignment;

// TODO: traits don't support async yet :(
pub trait Workflow {
    fn create_deployment(&self, workload: &Workload, workload_assignment: &WorkloadAssignment) -> Result<(), Error>;
}
