//! Steps for inline data and content-addressed attachments (spec
//! 08_inline_data).

use crate::steps::{last_message, record_message_on_current, BotSpyWorld};
use botspy::{content_hash, KnownPart, Message, Part, Role};
use cucumber::{then, when};

fn record_parts(world: &mut BotSpyWorld, parts: Vec<Part>) {
    record_message_on_current(
        world,
        Message {
            role: Role::Assistant,
            parts,
            ..Message::default()
        },
    );
}

#[when(
    regex = r#"I record an inline image part with media type "([^"]+)" and base64 data "([^"]+)""#
)]
fn record_inline_image(world: &mut BotSpyWorld, mime: String, data: String) {
    record_parts(
        world,
        vec![Part::Known(KnownPart::InlineData {
            mime,
            data_ref: Some(content_hash(&data)),
            data: Some(data),
            extra: None,
        })],
    );
}

#[when(regex = r#"I record the same inline image twice in one message"#)]
fn record_same_image_twice(world: &mut BotSpyWorld) {
    let data = "iVBORw0KGgo...".to_string();
    let part = || {
        Part::Known(KnownPart::InlineData {
            mime: "image/png".to_string(),
            data_ref: Some(content_hash(&data)),
            data: Some(data.clone()),
            extra: None,
        })
    };
    record_parts(world, vec![part(), part()]);
}

#[when(regex = r#"I record an image part whose data lives in blob "([^"]+)""#)]
fn record_blob_image(world: &mut BotSpyWorld, blob_ref: String) {
    record_parts(
        world,
        vec![Part::Known(KnownPart::InlineData {
            mime: "image/png".to_string(),
            data_ref: Some(blob_ref),
            data: None,
            extra: None,
        })],
    );
}

#[when(
    regex = r#"I record an attachment part with path "([^"]+)", mime "([^"]+)" and size ([0-9]+)"#
)]
fn record_attachment(world: &mut BotSpyWorld, path: String, mime: String, size: u64) {
    record_parts(
        world,
        vec![Part::Known(KnownPart::Attachment {
            path,
            mime,
            size,
            extra: None,
        })],
    );
}

#[then(regex = r#"part ([0-9]+) has mime "([^"]+)""#)]
fn part_mime(world: &mut BotSpyWorld, index: usize, mime: String) {
    let part = &last_message(world).parts[index - 1];
    let actual = match part {
        Part::Known(KnownPart::InlineData { mime, .. }) => mime,
        Part::Known(KnownPart::Attachment { mime, .. }) => mime,
        other => panic!("part {index} carries no mime: {other:?}"),
    };
    assert_eq!(actual, &mime);
}

#[then(regex = r#"part ([0-9]+) has inline data"#)]
fn part_has_inline_data(world: &mut BotSpyWorld, index: usize) {
    let part = &last_message(world).parts[index - 1];
    match part {
        Part::Known(KnownPart::InlineData { data: Some(_), .. }) => {}
        other => panic!("part {index} has no inline data: {other:?}"),
    }
}

#[then(regex = r#"both parts have the same content hash"#)]
fn same_content_hash(world: &mut BotSpyWorld) {
    let message = last_message(world);
    let refs: Vec<&str> = message
        .parts
        .iter()
        .map(|part| match part {
            Part::Known(KnownPart::InlineData {
                data_ref: Some(r), ..
            }) => r.as_str(),
            other => panic!("part is not content-addressed inline data: {other:?}"),
        })
        .collect();
    assert!(
        refs.len() >= 2,
        "expected two content-addressed parts, got {}",
        refs.len()
    );
    assert_eq!(refs[0], refs[1], "identical bytes hash differently");
}

#[then(regex = r#"each part has data_ref equal to that hash"#)]
fn data_ref_is_content_hash(world: &mut BotSpyWorld) {
    for part in &last_message(world).parts {
        match part {
            Part::Known(KnownPart::InlineData {
                data: Some(data),
                data_ref: Some(data_ref),
                ..
            }) => assert_eq!(
                *data_ref,
                content_hash(data),
                "data_ref is not the content hash of the data"
            ),
            other => panic!("part is not inline data: {other:?}"),
        }
    }
}

#[then(regex = r#"part ([0-9]+) has data_ref "([^"]+)""#)]
fn part_data_ref(world: &mut BotSpyWorld, index: usize, data_ref: String) {
    let part = &last_message(world).parts[index - 1];
    match part {
        Part::Known(KnownPart::InlineData {
            data_ref: Some(actual),
            ..
        }) => assert_eq!(actual, &data_ref),
        other => panic!("part {index} has no data_ref: {other:?}"),
    }
}

#[then(regex = r#"part ([0-9]+) has no inline data"#)]
fn part_no_inline_data(world: &mut BotSpyWorld, index: usize) {
    let part = &last_message(world).parts[index - 1];
    match part {
        Part::Known(KnownPart::InlineData { data: None, .. }) => {}
        other => panic!("part {index} unexpectedly carries inline data: {other:?}"),
    }
}
