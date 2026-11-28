use crate::obs::testing::scripted::ScriptedObs;
use crate::obs::testing::wire::{input, request_data, Reply};
use serde_json::json;

#[tokio::test]
async fn operator_reads_share_catalog_and_retain_independent_property_errors() {
    use crate::domain::operator_action::ActionTarget;
    use crate::domain::operator_input::InputRead;
    let uuid = "00000000-0000-0000-0000-000000000001";
    let target = ActionTarget {
        name: "Old name".into(),
        uuid: Some(uuid.into()),
    };
    let (client, server) = ScriptedObs::ordered(vec![
        ("GetInputList", Reply::Ok(json!({"inputs":[input("Renamed")]}))),
        (
            "GetMediaInputStatus",
            Reply::Ok(json!({"mediaState":"OBS_MEDIA_STATE_PLAYING","mediaDuration":5000,"mediaCursor":1000})),
        ),
        ("GetSourceFilterList", Reply::err(702)),
    ])
    .connect()
    .await;
    let inventory = client.get_input_catalog().await.unwrap();
    let reads = vec![
        InputRead {
            target: target.clone(),
            media: true,
            filters: false,
        },
        InputRead {
            target,
            media: false,
            filters: true,
        },
        InputRead {
            target: ActionTarget {
                name: "Renamed".into(),
                uuid: Some("00000000-0000-0000-0000-000000000002".into()),
            },
            media: true,
            filters: true,
        },
    ];
    let observed = client.get_operator_inputs(&inventory, &reads).await;
    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].target.name, "Renamed");
    assert_eq!(
        observed[0]
            .media
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .cursor_ms,
        Some(1000)
    );
    assert!(observed[0].filters.as_ref().unwrap().is_err());
    let requests = server.await.unwrap();
    assert_eq!(
        requests.len(),
        4,
        "one catalog and only requested properties, deduplicated by UUID"
    );
    for (index, field, absent) in [
        (2, "inputUuid", "inputName"),
        (3, "sourceUuid", "sourceName"),
    ] {
        assert_eq!(request_data(&requests[index])[field], uuid);
        assert!(request_data(&requests[index]).get(absent).is_none());
    }
}

#[tokio::test]
async fn operator_read_failure_cannot_hide_filters_or_dispatch_invalid_identity() {
    use crate::domain::operator_action::ActionTarget;
    use crate::domain::operator_input::InputRead;
    let (client, server) = ScriptedObs::ordered(vec![
        (
            "GetInputList",
            Reply::Ok(json!({"inputs":[input("Image")]})),
        ),
        ("GetMediaInputStatus", Reply::err(604)),
        ("GetSourceFilterList", Reply::Ok(json!({"filters":[]}))),
    ])
    .connect()
    .await;
    let mut inventory = client.get_input_catalog().await.unwrap();
    let read = InputRead {
        target: inventory[0].target.clone(),
        media: true,
        filters: true,
    };
    let observed = client.get_operator_inputs(&inventory, &[read]).await;
    assert!(observed[0].media.as_ref().unwrap().is_err());
    assert_eq!(observed[0].filters, Some(Ok(Vec::new())));
    inventory[0].target = ActionTarget {
        name: "Image".into(),
        uuid: Some("invalid UUID".into()),
    };
    let observed = client
        .get_operator_inputs(
            &inventory,
            &[InputRead {
                target: inventory[0].target.clone(),
                media: true,
                filters: true,
            }],
        )
        .await;
    assert!(observed[0].media.as_ref().unwrap().is_err());
    assert!(observed[0].filters.as_ref().unwrap().is_err());
    assert_eq!(server.await.unwrap().len(), 4);
}
