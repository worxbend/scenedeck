//! Bounded property reads for saved actions and independent input inspection.

use super::client::{collect_bounded_ordered, ObsClient, MAX_CONCURRENT_OBS_REQUESTS};
use crate::domain::{
    control::{MediaStatus, SourceFilter},
    operator_input::{resolve_input, InputIdentity, InputObservation, InputRead},
};
use crate::infra::error::AppError;
use obws::requests::{inputs::InputId, sources::SourceId};

impl ObsClient {
    pub(crate) async fn get_operator_inputs(
        &self,
        inventory: &[InputIdentity],
        reads: &[InputRead],
    ) -> Vec<InputObservation> {
        let mut merged = Vec::new();
        for read in reads {
            read.clone().merge_into(&mut merged);
        }
        let selected: Vec<_> = merged
            .iter()
            .filter_map(|read| {
                resolve_input(inventory, &read.target).map(|input| (input.clone(), read.clone()))
            })
            .collect();
        let result = collect_bounded_ordered(
            selected,
            MAX_CONCURRENT_OBS_REQUESTS,
            |(input, read): (InputIdentity, InputRead)| async move {
                let uuid = match input
                    .target
                    .uuid
                    .as_deref()
                    .map(|id| id.parse().map(InputId::Uuid))
                    .transpose()
                {
                    Ok(id) => id.and_then(|id| id.as_uuid()),
                    Err(error) => {
                        return Ok::<_, AppError>(InputObservation {
                            target: input.target.clone(),
                            media: read
                                .media
                                .then(|| Err(format!("Invalid input identity: {error}"))),
                            filters: read
                                .filters
                                .then(|| Err(format!("Invalid input identity: {error}"))),
                        })
                    }
                };
                let name = &input.target.name;
                let media = if read.media {
                    Some(
                        self.request(
                            self.inner
                                .media_inputs()
                                .status(uuid.map(InputId::Uuid).unwrap_or(InputId::Name(name))),
                        )
                        .await
                        .map(|status| MediaStatus {
                            state: super::mapper::map_media_state(status.state),
                            duration_ms: status.duration.map(|d| d.whole_milliseconds() as i64),
                            cursor_ms: status.cursor.map(|d| d.whole_milliseconds() as i64),
                        })
                        .map_err(|error| error.to_string()),
                    )
                } else {
                    None
                };
                let filters = if read.filters {
                    Some(
                        self.request(
                            self.inner
                                .filters()
                                .list(uuid.map(SourceId::Uuid).unwrap_or(SourceId::Name(name))),
                        )
                        .await
                        .map(|filters| {
                            filters
                                .into_iter()
                                .map(|filter| SourceFilter {
                                    name: filter.name,
                                    kind: filter.kind,
                                    enabled: filter.enabled,
                                })
                                .collect()
                        })
                        .map_err(|error| error.to_string()),
                    )
                } else {
                    None
                };
                Ok::<_, AppError>(InputObservation {
                    target: input.target.clone(),
                    media,
                    filters,
                })
            },
        )
        .await;
        // Each target retains its own failures; this bounded collector cannot fail.
        result.expect("operator property reads retain errors per input")
    }
}
