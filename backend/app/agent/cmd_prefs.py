from app.agent import config
from app.agent.commands import CommandError, command

KEYS = {"claudeBin", "claudeFolder", "claudeFirstMessage"}


@command("prefs.get", "Preferences", description="The saved preferences: the Claude Code program, its starting folder and first message.")
def get() -> dict:
    return {k: v for k, v in config.load_prefs().items() if k in KEYS}


@command("prefs.set", "Save Preferences", {"values": {"type": "object"}}, description="Saves preferences; unknown keys are refused.")
def set_(values: dict | None = None) -> dict:
    values = values or {}
    unknown = set(values) - KEYS
    if unknown:
        raise CommandError(f"Unknown preference {', '.join(sorted(unknown))}")
    return {k: v for k, v in config.save_prefs(values).items() if k in KEYS}
