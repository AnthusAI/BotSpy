"""Behave step definitions for the normalized schema."""

from behave import given, then, when

from botspy import Message, Part, Session

KINDS = ("text", "thinking", "tool_call", "tool_result", "attachment", "system")


@given('a session from "{source}" in project "{project}"')
def step_session(context, source, project):
    context.session = Session(session_id="s-1", source=source, project_id=project, messages=[])


@when("I append a message with parts:")
def step_append_parts_table(context):
    parts = [Part(row["kind"], row["text"]) for row in context.table]
    context.expected_kinds = [row["kind"] for row in context.table]
    context.session.messages.append(
        Message(role="user", parts=parts, timestamp="2026-10-02T12:00:00Z")
    )


@when('I append a message with a part of unknown kind "{kind}"')
def step_append_unknown_part(context, kind):
    context.session.messages.append(
        Message(role="user", parts=[Part(kind)], timestamp="2026-10-02T12:01:00Z")
    )


@then("the session has {count:d} message with {parts:d} parts in the given order")
def step_check_order(context, count, parts):
    assert len(context.session.messages) == count
    message = context.session.messages[0]
    assert len(message.parts) == parts
    assert [p.kind for p in message.parts] == context.expected_kinds


@then("every part kind is one of {kinds}")
def step_check_kinds(context, kinds):
    allowed = tuple(kinds.replace(" ", "").split(","))
    for message in context.session.messages:
        for part in message.parts:
            assert part.kind in allowed, f"unexpected kind: {part.kind}"


@then('the part is normalized to kind "{kind}"')
def step_normalized_kind(context, kind):
    part = context.session.messages[0].parts[0]
    assert part.kind == kind


@then('the original kind "{kind}" is preserved in extra')
def step_extra_preserved(context, kind):
    part = context.session.messages[0].parts[0]
    assert part.extra.get("kind") == kind
