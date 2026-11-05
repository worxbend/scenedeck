//! Input identities and targeted observations shared by operator controls.

use crate::domain::{
    control::{MediaStatus, SourceFilter},
    operator_action::{ActionTarget, OperatorAction},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InputIdentity {
    pub target: ActionTarget,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InputRead {
    pub target: ActionTarget,
    pub media: bool,
    pub filters: bool,
}

impl InputRead {
    pub fn for_action(action: &OperatorAction) -> Option<Self> {
        match action {
            OperatorAction::RestartMedia { target } => Some(Self {
                target: target.clone(),
                media: true,
                filters: false,
            }),
            OperatorAction::SetFilterEnabled { target, .. }
            | OperatorAction::ToggleFilter { target, .. } => Some(Self {
                target: target.clone(),
                media: false,
                filters: true,
            }),
            _ => None,
        }
    }

    /// Multiple actions against one identity share their property reads.
    pub fn merge_into(self, reads: &mut Vec<Self>) {
        if let Some(existing) = reads.iter_mut().find(|read| {
            self.target
                .matches(&read.target.name, read.target.uuid.as_deref())
        }) {
            existing.media |= self.media;
            existing.filters |= self.filters;
        } else {
            reads.push(self);
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct InputObservation {
    pub target: ActionTarget,
    /// None means not requested; an error never means healthy/inactive.
    pub media: Option<Result<MediaStatus, String>>,
    pub filters: Option<Result<Vec<SourceFilter>, String>>,
}

/// Resolve only an unambiguous identity, never a known UUID by cached name.
pub(crate) fn resolve_input<'a>(
    inventory: &'a [InputIdentity],
    target: &ActionTarget,
) -> Option<&'a InputIdentity> {
    let mut matches = inventory
        .iter()
        .filter(|input| target.matches(&input.target.name, input.target.uuid.as_deref()));
    let input = matches.next()?;
    if matches.next().is_some()
        || inventory
            .iter()
            .filter(|other| other.target.name == input.target.name)
            .count()
            != 1
    {
        return None;
    }
    Some(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_merge_by_identity_but_never_fall_back_to_a_reused_name() {
        let target = ActionTarget {
            name: "Old".into(),
            uuid: Some("one".into()),
        };
        let mut reads = vec![InputRead {
            target: target.clone(),
            media: true,
            filters: false,
        }];
        InputRead {
            target: ActionTarget {
                name: "Renamed".into(),
                uuid: Some("one".into()),
            },
            media: false,
            filters: true,
        }
        .merge_into(&mut reads);
        assert_eq!(reads.len(), 1);
        assert!(reads[0].media && reads[0].filters);
        let mut inventory = vec![InputIdentity {
            target: ActionTarget {
                name: "Renamed".into(),
                uuid: Some("one".into()),
            },
            kind: "media".into(),
        }];
        assert_eq!(
            resolve_input(&inventory, &target).unwrap().target.name,
            "Renamed"
        );
        inventory[0].target = ActionTarget {
            name: "Old".into(),
            uuid: Some("two".into()),
        };
        assert!(resolve_input(&inventory, &target).is_none());
        inventory[0].target = target.clone();
        inventory.push(inventory[0].clone());
        assert!(resolve_input(&inventory, &target).is_none());
    }
}
