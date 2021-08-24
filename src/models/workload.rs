use super::assignment_spec::AssignmentSpec;
use super::overrides_spec::OverridesSpec;
use super::template_spec::TemplateSpec;

#[allow(dead_code)]
pub struct Workload {
    // workload assignment specifications
    pub assignments: Vec<AssignmentSpec>,

    // The workload deployment template source for deployment in workload clusters.
    pub workload_template: TemplateSpec,

    // An optional global template source for deployment in the control plane clusters.
    pub global_template: Option<TemplateSpec>,

    // value override specs for the generic workload template.
    pub overrides: OverridesSpec
}
