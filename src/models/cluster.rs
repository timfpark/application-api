use std::collections::HashMap;

#[allow(dead_code)]
pub struct Cluster {
    pub name: String,
    pub labels: HashMap<String, String>,
}
