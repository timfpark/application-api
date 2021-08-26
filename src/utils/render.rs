use std::collections::HashMap;
use std::path::Path;
use handlebars::Handlebars;

#[allow(dead_code)]
pub fn render(template_path: &Path, output_path: &Path, values: &HashMap<&str, &str>) -> Result<u32, std::io::Error> {
    let entries = std::fs::read_dir(template_path)?;
    println!("entries: {:?}", entries);

    let mut files_rendered = 0;

    for entry_result in entries {
        let entry = entry_result?;
        let file_type = entry.file_type()?;

        let entry_template_path = entry.path();
        let entry_output_path = Path::new(&output_path).join(entry.file_name());

        if file_type.is_dir() {
            files_rendered += render(&entry_template_path, &entry_output_path, values)?;
        } else {
            let template = std::fs::read_to_string(entry_template_path)?;
            let mut handlebars = Handlebars::new();
            handlebars.register_template_string("template", template).unwrap();

            let rendered_file = match handlebars.render("template", values) {
                Ok(rendered_file) => rendered_file,
                Err(err) => return Err(std::io::Error::from_raw_os_error(34))
            };

            std::fs::write(entry_output_path, rendered_file.as_bytes())?;
            files_rendered += 1;
        }
    }

    Ok(files_rendered)
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
        let output_path = Path::new("./fixtures/workloads/my-cluster");

        std::fs::create_dir_all(output_path).unwrap();

        let files_rendered = render(template_path, output_path, &values).unwrap();

        assert_eq!(files_rendered, 2);
    }
}
