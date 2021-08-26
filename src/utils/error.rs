use tokio::runtime::Handle;

/// Utility enum that covers all possible errors during reconciliation
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Any error originating from the `kube-rs` crate
    #[error("Kubernetes reported error: {source}")]
    KubeError {
        #[from]
        source: kube::Error,
    },

    /// Error in user input or WorkloadAssignment resource definition, typically missing fields.
    #[error("Invalid WorkloadAssignment CRD: {0}")]
    UserInputError(String),

    #[error("Git error: {source}")]
    GitError {
        #[from]
        source: git2::Error,
    },

    #[error("I/O error: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },

    #[error("Render error: {source}")]
    RenderError {
        #[from]
        source: handlebars::RenderError
    }
}
