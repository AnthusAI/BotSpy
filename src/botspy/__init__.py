"""BotSpy.

Gherkin behavior specifications (features/) are the backbone and the true
source of this project. Implementation code is generated from the specs.
"""

from botspy.schema import SCHEMA_VERSION, Message, Part, Session

__all__ = [
    "PART_KINDS",
    "SCHEMA_VERSION",
    "Message",
    "Part",
    "Session",
]
