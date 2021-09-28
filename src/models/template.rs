use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct TemplateSpec {
    pub method: Option<String>,
    pub source: String,
    pub path: String,
}
