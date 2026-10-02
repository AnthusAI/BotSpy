"""BotSpy.

Gherkin behavior specifications (features/) are the backbone and the true
source of this project. Implementation code is generated from the specs.
"""

__all__: list[str] = ["Part", "Message", "Session", "SCHEMA_VERSION", "PART_KINDS"]

SCHEMA_VERSION = 1

PART_KINDS = (
    "text",
    "thinking",
    "tool_call",
    "tool_result",
    "attachment",
    "system",
)


class Part:
    """One typed content part of a message.

    Kinds outside PART_KINDS are preserved verbatim in ``extra`` so nothing
    is lost when an agent has content BotSpy does not model yet.
    """

    def __init__(self, kind: str, text: str = "", extra: dict | None = None) -> None:
        if kind not in PART_KINDS:
            if extra is None:
                extra = {}
            extra.setdefault("kind", kind)
            kind = "text"
        self.kind = kind
        self.text = text
        self.extra = extra


class Message:
    """An ordered message inside a session, made of typed parts."""

    def __init__(self, role: str, parts: list[Part], timestamp: str) -> None:
        self.role = role
        self.parts = parts
        self.timestamp = timestamp


class Session:
    """A coding-agent session: ordered messages plus identifiers."""

    def __init__(
        self,
        session_id: str,
        source: str,
        project_id: str,
        messages: list[Message],
    ) -> None:
        self.session_id = session_id
        self.source = source
        self.project_id = project_id
        self.messages = messages
