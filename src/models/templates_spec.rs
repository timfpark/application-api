use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::template_spec::TemplateSpec;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct TemplatesSpec {
    pub application: TemplateSpec,
    pub global: Option<TemplateSpec>,
}
