//! Workspace requests bound to a loopback client.

use crate::controller::workspace::WorkspaceRequest;
use crate::domain::workflow::ScopeBinding;
use crate::obs::client::ObsClient;

/// The `Show` collection on `client`'s endpoint.
pub(crate) fn binding(client: &ObsClient) -> ScopeBinding {
    ScopeBinding {
        endpoint: client.endpoint().into(),
        collection: "Show".into(),
    }
}

/// A first-epoch request for the `Text` input, without sources or a
/// screenshot.
pub(crate) fn request(client: &ObsClient) -> WorkspaceRequest {
    WorkspaceRequest {
        operator_reads: Vec::new(),
        target_uuid: None,
        epoch: 1,
        sequence: 1,
        scope: binding(client),
        scene: None,
        input: Some("Text".into()),
        include_sources: false,
        screenshot: false,
        result_id: None,
    }
}
