use git2::{Cred, RemoteCallbacks, Repository};
use git2::build::RepoBuilder;
use std::env;
use std::path::Path;
use std::thread;
use tempfile::tempdir;

use crate::models::workload::Workload;
use crate::models::workload_assignment::WorkloadAssignment;
use crate::utils::error::Error;

pub struct GitopsWorkflow {
    pub workload_repo_url: String,
}

impl GitopsWorkflow {
    fn get_repo_builder(&self) -> RepoBuilder {
        // Prepare callbacks.
        let mut callbacks = RemoteCallbacks::new();

        // TODO: Migrate to secrets
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            println!("{:?}", username_from_url);

            Cred::ssh_key(
                username_from_url.unwrap(),
                None,
                std::path::Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
                None,
            )
        });

        // Prepare fetch options.
        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        // Prepare builder.
        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_options);

        builder
    }

    fn clone_deployment_repo(&self, workload: &Workload) -> Result<Repository, git2::Error> {
        // let temp_dir = tempdir()?;
        // let repo_path = temp_dir.path();

        let repo_path = Path::new("/Users/timothypark/dev/multicloud/test/deployment");
        println!("deployment_repo {:?}", repo_path);
        let mut repo_builder = self.get_repo_builder();

        return repo_builder.clone(&workload.spec.templates.deployment.source, repo_path);
    }

    fn clone_workload_gitops_repo(&self) -> Result<Repository, git2::Error> {
        // let temp_path = env::temp_dir();
        // let repo_path = Path::new(&temp_path).join(&self.workload_repo_url);

        let repo_path = Path::new("/Users/timothypark/dev/multicloud/test/workload");
        println!("workload_repo {:?}", repo_path);
        let mut repo_builder = self.get_repo_builder();

        return repo_builder.clone(&self.workload_repo_url, repo_path);
    }

    pub async fn create_deployment(&self, workload: &Workload, workload_assignment: &WorkloadAssignment) -> Result<(), Error> {
        println!("gitopsworkflow: create_deployment");

        // clone app repo specified by workload.spec.templates.deployment.source
        let workload_deployment_repo = self.clone_deployment_repo(workload)?;

        // clone workload cluster gitops repo specified by workload_repo_url
        let workload_gitops_repo = self.clone_workload_gitops_repo()?;

        // pass (app repo + path) into renderer with workload and cluster
        // renderer templates deployment with variables and returns path

        // copy workload subdirectory at templated path into workload cluster gitops repo at cluster path
        // link subdirectory into kustomization
        // add and commit workload cluster gitops repo

        Ok(())
    }

    pub async fn delete_deployment(&self, workload: &Workload, workload_assignment: &WorkloadAssignment) -> Result<(), Error> {
        println!("gitopsworkflow: delete_deployment");

        // clone workload cluster gitops repo specified by workload_repo_url

        // delete workload path in workload cluster gitops repo

        // add deleted files and make commit

        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use kube::core::metadata::ObjectMeta;
    use crate::models::templates_spec::TemplatesSpec;
    use crate::models::template_spec::TemplateSpec;
    use crate::models::workload::{Workload, WorkloadSpec};
    use crate::models::workload_assignment::{WorkloadAssignment, WorkloadAssignmentSpec};

    use super::GitopsWorkflow;

    #[tokio::test]
    async fn can_create_deployment() {
        std::thread::spawn(|| {
            let workflow = GitopsWorkflow {
                workload_repo_url: "https://github.com/timfpark/workload-gitops".to_string()
            };

            let workload = Workload {
                api_version: "v1".to_string(),
                kind: "Workload".to_string(),
                metadata: ObjectMeta {
                    name: Some("test-workload".to_string()),
                    namespace: Some("default".to_string()),
                    ..ObjectMeta::default()
                },
                spec: WorkloadSpec {
                    templates: TemplatesSpec {
                        deployment: TemplateSpec {
                            source: "https://github.com/timfpark/cluster-agent".to_string(),
                            path: "deployment".to_string()
                        },

                        global: None
                    }
                }
            };

            let workload_assignment = WorkloadAssignment {
                api_version: "v1".to_string(),
                kind: "WorkloadAssignment".to_string(),
                metadata: ObjectMeta {
                    name: Some("test-workload-assignment".to_string()),
                    namespace: Some("default".to_string()),
                    ..ObjectMeta::default()
                },
                spec: WorkloadAssignmentSpec {
                    cluster: "azure-eastus2-1".to_string(),
                    workload: "test-workload".to_string()
                }
            };

            tokio_test::block_on(async {
                match workflow.create_deployment(&workload, &workload_assignment).await {
                    Err(err) => {
                        eprintln!("create deployment failed with: {:?}", err);
                        assert_eq!(false, true);
                    }
                    _ => {}
                }
            })
        }).join().expect("Thread panicked")
    }
}
