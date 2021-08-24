pub enum Source {
    Git,
    File
}

#[allow(dead_code)]
pub struct TemplateSpec {
    pub source: Source,

    // location template is stored (source dependent)
    pub location: String,
}
