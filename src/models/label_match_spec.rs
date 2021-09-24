use super::cluster::Cluster;
use regex::Regex;

#[allow(dead_code)]
pub struct LabelMatchSpec {
    pub label: String,
    pub regex: Regex,
}

impl LabelMatchSpec {
    pub fn matches(&self, cluster: &Cluster) -> bool {
        match cluster.labels.get(&self.label) {
            Some(cluster_label_value) => self.regex.is_match(cluster_label_value),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::models::cluster::Cluster;
    use crate::models::label_match_spec::LabelMatchSpec;
    use regex::Regex;
    use std::collections::HashMap;

    #[test]
    fn can_match_exact_spec() {
        let labels: HashMap<String, String> = [("name".to_string(), "azure-eastus2-1".to_string())]
            .iter()
            .cloned()
            .collect();

        let cluster = Cluster {
            name: "azure-eastus2-1".to_string(),
            labels,
        };
        let regex = Regex::new("azure-eastus2-1").unwrap();
        let cluster_spec = LabelMatchSpec {
            label: "name".to_string(),
            regex,
        };

        assert_eq!(cluster_spec.matches(&cluster), true);
    }

    #[test]
    fn can_match_regex() {
        let labels: HashMap<String, String> = [("name".to_string(), "azure-eastus2-1".to_string())]
            .iter()
            .cloned()
            .collect();

        let cluster = Cluster {
            name: "azure-eastus2-1".to_string(),
            labels,
        };
        let regex = Regex::new("azure-(.)*").unwrap();

        let cluster_spec = LabelMatchSpec {
            label: "name".to_string(),
            regex,
        };

        assert_eq!(cluster_spec.matches(&cluster), true);
    }

    #[test]
    fn can_not_match_regex() {
        let labels: HashMap<String, String> = [("name".to_string(), "gcp-eastus2-1".to_string())]
            .iter()
            .cloned()
            .collect();

        let cluster = Cluster {
            name: "gcp-eastus2-1".to_string(),
            labels,
        };
        let regex = Regex::new("azure-(.)*").unwrap();

        let cluster_spec = LabelMatchSpec {
            label: "name".to_string(),
            regex,
        };

        assert_eq!(cluster_spec.matches(&cluster), false);
    }
}
