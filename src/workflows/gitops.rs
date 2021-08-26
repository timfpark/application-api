use git2::{Cred, RemoteCallbacks, Repository};
use git2::build::RepoBuilder;
use std::collections::HashMap;
use std::env;
use std::fs::{create_dir, create_dir_all, remove_dir_all};
use std::path::Path;
use std::thread;
use tempfile::tempdir;

use crate::models::workload::Workload;
use crate::models::workload_assignment::WorkloadAssignment;
use crate::utils::render::render;
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

    fn clone_deployment_repo(&self, workload: &Workload) -> Result<Repository, Error> {
        // let temp_dir = tempdir()?;
        // let repo_path = temp_dir.path();

        let repo_path = Path::new("/Users/timothypark/dev/multicloud/test/app");

        std::fs::create_dir_all(repo_path).unwrap();
        std::fs::remove_dir_all(repo_path).unwrap();
        std::fs::create_dir_all(repo_path).unwrap();

        let mut repo_builder = self.get_repo_builder();

        match repo_builder.clone(&workload.spec.templates.cluster.source, repo_path) {
            Ok(repo) => { Ok(repo) },
            Err(err) => { Err(Error::GitError { source: err } ) }
        }
    }

    fn clone_workload_gitops_repo(&self) -> Result<Repository, Error> {
        // let temp_path = env::temp_dir();
        // let repo_path = Path::new(&temp_path).join(&self.workload_repo_url);

        let repo_path = Path::new("/Users/timothypark/dev/multicloud/test/workload");

        std::fs::create_dir_all(repo_path).unwrap();
        std::fs::remove_dir_all(repo_path).unwrap();
        std::fs::create_dir_all(repo_path).unwrap();

        let mut repo_builder = self.get_repo_builder();

        match repo_builder.clone(&self.workload_repo_url, repo_path) {
            Ok(repo) => {
                Ok(repo)
            },
            Err(err) => {
                Err(Error::GitError { source: err } )
            }
        }
    }

    pub fn link(&self, cluster_path: &Path) -> Result<(), Error> {
        // read all directory names (aka workloads)
        let entries = std::fs::read_dir(cluster_path)?;

        let mut workloads = Vec::new();
        for entry_result in entries {
            let entry = entry_result?;
            let file_type = entry.file_type()?;

            if file_type.is_dir() {
                workloads.push(entry.file_name());
            }
        }

        let workload_list: String = workloads.into_iter().map(|workload| {
            let display_string = workload.to_string_lossy();
            format!("- {}\n", display_string)
        }).collect();
        let kustomization =
format!("apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization
resources:
{}
", workload_list);

        let kustomize_path = cluster_path.join("kustomization.yaml");

        std::fs::write(kustomize_path, kustomization.as_bytes())?;

        Ok(())
    }

    pub fn create_deployment(&self, workload: &Workload, workload_assignment: &WorkloadAssignment) -> Result<(), Error> {
        // clone app repo specified by workload.spec.templates.deployment.source
        let workload_deployment_repo = self.clone_deployment_repo(workload)?;

        // clone workload cluster gitops repo specified by workload_repo_url
        let workload_gitops_repo = self.clone_workload_gitops_repo()?;

        let template_path = Path::new(workload_deployment_repo.path())
                                        .join("..")
                                        .join(&workload.spec.templates.cluster.path);

        let cluster_path = Path::new(workload_gitops_repo.path())
                                        .join("..") // path() points to .git -> backout one level
                                        .join("workloads") // TODO: should be less opinionated / more configurable about where workloads go
                                        .join(&workload_assignment.spec.cluster);

        let output_path = cluster_path.join(&workload_assignment.spec.workload);

        create_dir_all(&output_path)?;
        remove_dir_all(&output_path)?;
        create_dir(&output_path)?;

        // build global template variables
        let mut values: HashMap<&str, &str> = HashMap::new();
        values.insert("CLUSTER_NAME", &workload_assignment.spec.cluster);

        render(&template_path, &output_path, &values)?;

        self.link(&cluster_path)?;

        // add and commit workload cluster gitops repo
        // see https://zsiciarz.github.io/24daysofrust/book/vol2/day16.html

        Ok(())
    }

    pub fn delete_deployment(&self, workload: &Workload, workload_assignment: &WorkloadAssignment) -> Result<(), Error> {
        println!("gitopsworkflow: delete_deployment");

        // clone workload cluster gitops repo specified by workload_repo_url
        let workload_gitops_repo = self.clone_workload_gitops_repo()?;

        // delete workload path in workload cluster gitops repo
        let output_path = Path::new(workload_gitops_repo.path())
                                       .join("workloads")
                                       .join(&workload_assignment.spec.cluster)
                                       .join(&workload_assignment.spec.workload);

        remove_dir_all(&output_path)?;

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

    #[test]
    fn can_create_deployment() {
        let workflow = GitopsWorkflow {
            workload_repo_url: "https://github.com/timfpark/workload-gitops".to_string()
        };

        let workload = Workload {
            api_version: "v1".to_string(),
            kind: "Workload".to_string(),
            metadata: ObjectMeta {
                name: Some("cluster-agent".to_string()),
                namespace: Some("default".to_string()),
                ..ObjectMeta::default()
            },
            spec: WorkloadSpec {
                templates: TemplatesSpec {
                    cluster: TemplateSpec {
                        source: "https://github.com/timfpark/cluster-agent".to_string(),
                        path: "templates/deployment".to_string()
                    },

                    global: None
                }
            }
        };

        let workload_assignment = WorkloadAssignment {
            api_version: "v1".to_string(),
            kind: "WorkloadAssignment".to_string(),
            metadata: ObjectMeta {
                name: Some("azure-eastus-1-cluster-agent".to_string()),
                namespace: Some("default".to_string()),
                ..ObjectMeta::default()
            },
            spec: WorkloadAssignmentSpec {
                cluster: "azure-eastus2-1".to_string(),
                workload: "cluster-agent".to_string()
            }
        };

        match workflow.create_deployment(&workload, &workload_assignment) {
            Err(err) => {
                eprintln!("create deployment failed with: {:?}", err);
                assert_eq!(false, true);
            }
            Ok(_) => {}
        }
    }
}
