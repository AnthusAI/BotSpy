"""Behave environment for BotSpy behavior specifications."""


def before_all(context):
    context.schema_version = 1
