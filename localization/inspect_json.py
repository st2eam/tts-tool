from __future__ import annotations

import json
from pathlib import Path


source = Path(__file__).parent / "source" / "3689375010.original.json"
data = json.loads(source.read_text(encoding="utf-8"))


def walk(objects: list[dict]) -> None:
    for obj in objects:
        nickname = obj.get("Nickname", "")
        description = obj.get("Description", "")
        if nickname or description:
            print(obj.get("GUID", ""), repr(nickname), repr(description))
        walk(obj.get("ContainedObjects", []))


walk(data["ObjectStates"])
