use kube::Error;

use crate::models::application::Application;
use crate::models::application_assignment::ApplicationAssignment;

// TODO: traits don't support async yet :(
pub trait Workflow {
    fn create_deployment(
        &self,
        application: &Application,
        application_assignment: &ApplicationAssignment,
    ) -> Result<(), Error>;
}
