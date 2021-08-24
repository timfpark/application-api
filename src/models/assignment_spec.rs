use super::label_match_spec::LabelMatchSpec;
use super::cluster::Cluster;

#[allow(dead_code)]
pub struct AssignmentSpec {
    pub id: String,
    pub matching_labels: Vec<LabelMatchSpec>,

    pub max_assignments: Option<i32>, // None = all that match
}

impl AssignmentSpec {
    #[allow(dead_code)]
    pub fn matches(&self, cluster: &Cluster) -> bool {
        for match_spec in &self.matching_labels {
            if !match_spec.matches(cluster) {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;
    use std::collections::HashMap;
    use crate::models::assignment_spec::AssignmentSpec;
    use crate::models::label_match_spec::LabelMatchSpec;
    use crate::models::cluster::Cluster;

    #[test]
    fn spec_can_match_equality() {
        let labels: HashMap<String, String> = [
            ("name".to_string(), "azure-eastus2-1".to_string()),
            ("cloud".to_string(), "azure".to_string()),
            ("region".to_string(), "eastus2".to_string())
        ].iter().cloned().collect();

        let cluster = Cluster { name: "azure-eastus2-1".to_string(), labels };
        let regex = Regex::new("azure").unwrap();
        let label_spec = LabelMatchSpec { label: "cloud".to_string(), regex };
        let matching_labels = vec![label_spec];
        let assignment_spec = AssignmentSpec { id: "1".to_string(), matching_labels, max_assignments: None };

        assert_eq!(assignment_spec.matches(&cluster), true);
    }

    #[test]
    fn spec_can_match_regex() {
        let labels: HashMap<String, String> = [
            ("name".to_string(), "azure-eastus2-1".to_string()),
            ("cloud".to_string(), "azure".to_string()),
            ("region".to_string(), "eastus2".to_string())
        ].iter().cloned().collect();
        let cluster = Cluster { name: "azure-eastus2-1".to_string(), labels };

        let regex = Regex::new("eastus(.)*").unwrap();
        let label_spec = LabelMatchSpec { label: "region".to_string(), regex };
        let matching_labels = vec![label_spec];
        let assignment_spec = AssignmentSpec { id: "1".to_string(), matching_labels, max_assignments: None };

        assert_eq!(assignment_spec.matches(&cluster), true);
    }

    #[test]
    fn can_not_match_regex() {
        let labels: HashMap<String, String> = [
            ("name".to_string(), "azure-eastus2-1".to_string()),
            ("cloud".to_string(), "azure".to_string()),
            ("region".to_string(), "eastus2".to_string())
        ].iter().cloned().collect();
        let cluster = Cluster { name: "azure-eastus2-1".to_string(), labels };

        let regex = Regex::new("westus(.)*").unwrap();
        let label_spec = LabelMatchSpec { label: "region".to_string(), regex };
        let matching_labels = vec![label_spec];
        let assignment_spec = AssignmentSpec { id: "1".to_string(), matching_labels, max_assignments: None };

        assert_eq!(assignment_spec.matches(&cluster), false);
    }
}

