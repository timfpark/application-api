use std::collections::HashMap;
use std::fs::{create_dir_all};
use std::path::{Path, PathBuf};
use handlebars::Handlebars;

use crate::utils::error::Error;

#[allow(dead_code)]
pub fn render(template_path: &Path, repo_root_path: &Path, root_relative_path: &Path, values: &HashMap<&str, &str>) -> Result<Vec<PathBuf>, Error> {
    let mut paths = Vec::new();

    let output_path = repo_root_path.join(root_relative_path);
    create_dir_all(&output_path)?;

    let entries = std::fs::read_dir(template_path)?;

    for entry_result in entries {
        let entry = entry_result?;
        let file_type = entry.file_type()?;

        let entry_template_path = entry.path();

        let output_relative_path = Path::new(root_relative_path).join(entry.file_name());
        let output_absolute_path = Path::new(&repo_root_path).join(&output_relative_path);

        if file_type.is_dir() {
            let mut subpaths = render(&entry_template_path, repo_root_path, &output_relative_path, values)?;
            paths.append(&mut subpaths);
        } else {
            println!("adding path to list {:?}", output_relative_path);
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::Path;

    use super::render;

    #[test]
    fn can_render_workload() {
        let mut values: HashMap<&str, &str> = HashMap::new();
        values.insert("CLUSTER_NAME", "my-cluster");

        let template_path = Path::new("./fixtures/template");
        let repo_root_path = Path::new("./fixtures/");
        let root_relative_path = Path::new("workloads/my-cluster");
        let output_path = repo_root_path.join(root_relative_path);

        std::fs::create_dir_all(output_path).unwrap();

        let files_rendered = render(template_path, repo_root_path, root_relative_path, &values).unwrap();

        assert_eq!(files_rendered.len(), 2);
    }
}
