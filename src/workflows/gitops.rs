use kube::{Error};

use crate::models::workload::Workload;
use crate::models::workload_assignment::WorkloadAssignment;

pub struct GitopsWorkflow {
    pub workload_repo_url: String,
}

impl GitopsWorkflow {
    pub async fn create_deployment(&self, workload: &Workload, workload_assignment: &WorkloadAssignment) -> Result<(), Error> {
        println!("gitopsworkflow: create_deployment");

        Ok(())
    }

    pub async fn delete_deployment(&self, workload: &Workload, workload_assignment: &WorkloadAssignment) -> Result<(), Error> {
        println!("gitopsworkflow: delete_deployment");

        Ok(())
    }
}
