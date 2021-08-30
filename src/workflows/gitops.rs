use git2::{Cred, ObjectType, Direction, Index, Oid, PushOptions, RemoteCallbacks, Repository, Signature};
use git2::build::RepoBuilder;
use handlebars::Handlebars;
use std::collections::HashMap;
use std::env;
use std::fs::{create_dir_all, remove_dir_all};
use std::path::{Path, PathBuf};
use tempfile::{TempDir, tempdir};

use crate::models::workload::Workload;
use crate::models::workload_assignment::WorkloadAssignment;
use crate::utils::error::Error;

pub struct GitopsWorkflow {
    pub workload_repo_url: String,
}

impl GitopsWorkflow {
    pub fn new(workload_repo_url: &str) -> Result<GitopsWorkflow, Error> {
        return Ok(GitopsWorkflow {
            workload_repo_url: workload_repo_url.to_string(),
        })
    }

    fn get_auth_callback(&self) -> RemoteCallbacks {
        // Prepare callbacks.
        let mut callbacks = RemoteCallbacks::new();


        // TODO: Migrate to secrets
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            let home_dir = env::var("HOME").unwrap();
            let private_ssh_key_path = std::path::Path::new(&home_dir).join(".ssh/id_rsa");

            Cred::ssh_key(
                username_from_url.unwrap(),
                None,
                &private_ssh_key_path,
                None,
            )
        });

        callbacks
    }

    fn get_repo_builder(&self) -> RepoBuilder {

        let auth_callback = self.get_auth_callback();

        // Prepare fetch options.
        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.remote_callbacks(auth_callback);

        // Prepare builder.
        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_options);

        builder
    }

    fn clone_deployment_repo(&self, workload: &Workload, deployment_temp_dir: &TempDir) -> Result<Repository, Error> {
        let repo_path = deployment_temp_dir.path().join("template");

        let mut repo_builder = self.get_repo_builder();

        match repo_builder.clone(&workload.spec.templates.workload.source, &repo_path) {
            Ok(repo) => { Ok(repo) },
            Err(err) => { Err(Error::GitError { source: err } ) }
        }
    }

    fn clone_workload_gitops_repo(&self, workload_gitops_temp_dir: &TempDir) -> Result<Repository, Error> {
        let repo_path = workload_gitops_temp_dir.path().join("gitops");

        let mut repo_builder = self.get_repo_builder();

        match repo_builder.clone(&self.workload_repo_url, &repo_path) {
            Ok(repo) => {
                Ok(repo)
            },
            Err(err) => {
                Err(Error::GitError { source: err } )
            }
        }
    }

    fn render(&self, template_path: &Path, repo_root_path: &Path, root_relative_path: &Path, values: &HashMap<&str, &str>) -> Result<Vec<PathBuf>, Error> {
        let mut paths = Vec::new();

        let output_path = repo_root_path.join(root_relative_path);
        create_dir_all(&output_path)?;

        let entries = std::fs::read_dir(template_path)?;

        for entry_result in entries {
            let entry = entry_result?;
            let file_type = entry.file_type()?;
            let is_dotted_file_name = entry.file_name().to_str().unwrap().chars().next().unwrap() == '.';

            let entry_template_path = entry.path();

            let output_relative_path = Path::new(root_relative_path).join(entry.file_name());
            let output_absolute_path = Path::new(&repo_root_path).join(&output_relative_path);

            if file_type.is_dir() {
                if !is_dotted_file_name {
                    let mut subpaths = self.render(&entry_template_path, repo_root_path, &output_relative_path, values)?;
                    paths.append(&mut subpaths);
                }
            } else {
                // println!("adding path to list {:?}", output_relative_path);
                paths.push(output_relative_path);

                let template = std::fs::read_to_string(entry_template_path)?;
                let mut handlebars = Handlebars::new();
                handlebars.register_template_string("template", template).unwrap();

                let rendered_file = match handlebars.render("template", values) {
                    Ok(rendered_file) => rendered_file,
                    Err(err) => return Err(Error::RenderError { source: err } )
                };

                std::fs::write(output_absolute_path, rendered_file.as_bytes())?;
            }
        }

        Ok(paths)
    }

    fn link(&self, cluster_path: &Path) -> Result<(), Error> {
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
            format!("    - {}\n", display_string)
        }).collect();
        let kustomization =
    format!("apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization
resources:
{}", workload_list);

        let kustomization_path = cluster_path.join("kustomization.yaml");

        std::fs::write(kustomization_path, kustomization.as_bytes())?;

        Ok(())
    }

    fn commit_files(&self, repo: &Repository, index: &mut Index, paths: Vec<PathBuf>, message: &str) -> Result<Oid, Error> {
       for path in paths.iter() {
            // println!("adding path to index {:?}", path);
            index.add_path(path)?;
        }

        let oid = index.write_tree()?;

        // TODO: Add mechanism to provide identity of commits.
        let signature = Signature::now("Workload API", "workload-api@example.com")?;

        let obj = repo.head()?.resolve()?.peel(ObjectType::Commit)?;
        let parent_commit = match obj.into_commit() {
            Ok(commit) => commit,
            Err(_) => return Err(Error::GitError { source: git2::Error::from_str("Couldn't find commit") })
        };

        let tree = repo.find_tree(oid)?;

        repo.commit(Some("HEAD"), //  point HEAD to our new commit
                    &signature, // author
                    &signature, // committer
                    message, // commit message
                    &tree, // tree
                    &[&parent_commit])?; // parents

        Ok(oid)
    }

    fn push(&self, repo: &Repository, url: &str, branch: &str) -> Result<(), Error> {

        let mut remote = match repo.find_remote("origin") {
            Ok(r) => r,
            Err(_) => repo.remote("origin", url)?,
        };

        let connect_auth_callback = self.get_auth_callback();
        remote.connect_auth(Direction::Push, Some(connect_auth_callback), None)?;

        let ref_spec = format!("refs/heads/{}:refs/heads/{}", branch, branch);

        let push_auth_callback = self.get_auth_callback();
        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(push_auth_callback);

        remote.push(&[ref_spec], Some(&mut push_options))?;

        Ok(())
    }

    pub fn create_deployment(&self, workload: &Workload, workload_assignment: &WorkloadAssignment) -> Result<Oid, Error> {
        let deployment_temp_dir = tempdir()?;
        let workload_gitops_temp_dir = tempdir()?;

        // clone app repo specified by workload.spec.templates.deployment.source
        let workload_template_repo = self.clone_deployment_repo(workload, &deployment_temp_dir)?;

        // clone workload cluster gitops repo specified by workload_repo_url
        let workload_gitops_repo = self.clone_workload_gitops_repo(&workload_gitops_temp_dir)?;

        let template_path = Path::new(workload_template_repo.path()).parent().unwrap()
                                        .join(&workload.spec.templates.workload.path);

        let workload_gitops_repo_path = Path::new(workload_gitops_repo.path()).parent().unwrap();

        // TODO: should be less opinionated / more configurable about where workloads go
        let cluster_relative_path = Path::new("workloads")
                                            .join(&workload_assignment.spec.cluster);
        let cluster_path = workload_gitops_repo_path.join(&cluster_relative_path);

        let output_relative_path = cluster_relative_path.join(&workload_assignment.spec.workload);

        let mut index = workload_gitops_repo.index()?;
        index.remove_dir(&output_relative_path, 0)?;

        // build template context variables
        let mut template_values: HashMap<&str, &str> = HashMap::new();
        template_values.insert("clusterName", &workload_assignment.spec.cluster);

        // TODO: Fetch assigned cluster when we are using Cluster API
        template_values.insert("cloud", "azure");
        template_values.insert("cloudRegion", "eastus2");

        // add in values from Workload
        if let Some(values) = &workload.spec.values {
            for (key, value) in values.iter() {
                template_values.insert(&key, &value);
            }
        }

        let mut paths = self.render(&template_path, workload_gitops_repo_path, &output_relative_path, &template_values)?;
        self.link(&cluster_path)?;

        let kustomization_path = cluster_relative_path.join("kustomization.yaml");
        paths.push(kustomization_path);

        // TODO(ENH): Support different messages
        let message = format!("Reconciling created WorkloadAssignment {} for Workload {} for Cluster {}", workload_assignment.metadata.name.as_ref().unwrap(), workload_assignment.spec.workload, workload_assignment.spec.cluster);

        // add and commit output path in workload cluster gitops repo
        let oid = self.commit_files(&workload_gitops_repo, &mut index, paths, &message)?;

        // TODO: make more flexible to support different branches
        self.push(&workload_gitops_repo, &self.workload_repo_url, "main")?;

        Ok(oid)
    }

    pub fn delete_deployment(&self, _workload: &Workload, workload_assignment: &WorkloadAssignment) -> Result<(), Error> {
        println!("gitopsworkflow: delete_deployment");
        let workload_gitops_temp_dir = tempdir()?;

        // clone workload cluster gitops repo specified by workload_repo_url
        let workload_gitops_repo = self.clone_workload_gitops_repo(&workload_gitops_temp_dir)?;

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
    use std::collections::HashMap;
    use std::path::Path;

    use crate::models::templates_spec::TemplatesSpec;
    use crate::models::template_spec::TemplateSpec;
    use crate::models::workload::{Workload, WorkloadSpec};
    use crate::models::workload_assignment::{WorkloadAssignment, WorkloadAssignmentSpec};

    use super::GitopsWorkflow;

    #[test]
    fn can_create_deployment() {
        let workflow = GitopsWorkflow::new("git@github.com:timfpark/workload-cluster-gitops").unwrap();

        let mut values: HashMap<String, String> = HashMap::new();
        values.insert("ring".to_string(), "main".to_string());

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
                    workload: TemplateSpec {
                        method: Some("git".to_string()),
                        source: "git@github.com:timfpark/cluster-agent".to_string(),
                        path: "templates/deployment".to_string()
                    },

                    global: None
                },
                values: Some(values)
            }
        };

        let workload_assignment = WorkloadAssignment {
            api_version: "v1".to_string(),
            kind: "WorkloadAssignment".to_string(),
            metadata: ObjectMeta {
                name: Some("azure-eastus2-1-cluster-agent".to_string()),
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

    #[test]
    fn can_render_workload() {
        let workflow = GitopsWorkflow::new("git@github.com:timfpark/workload-cluster-gitops").unwrap();

        let mut values: HashMap<&str, &str> = HashMap::new();
        values.insert("CLUSTER_NAME", "my-cluster");

        let template_path = Path::new("./fixtures/template");
        let repo_root_path = Path::new("./fixtures/");
        let root_relative_path = Path::new("workloads/my-cluster");
        let output_path = repo_root_path.join(root_relative_path);

        std::fs::create_dir_all(output_path).unwrap();

        let paths = workflow.render(template_path, repo_root_path, root_relative_path, &values).unwrap();

        assert_eq!(paths.len(), 2);
    }
}
