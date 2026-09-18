"""Audit every image/PDF URL referenced by the localized TTS save."""

from __future__ import annotations

import json
import re
from collections import Counter
from pathlib import Path
from urllib.parse import unquote

from PIL import Image


SAVE = Path(r"D:\Documents\My Games\Tabletop Simulator\Mods\Workshop\3689375010_zh.json")
CACHE = Path(r"D:\Documents\My Games\Tabletop Simulator\Mods\Images")


def walk(value, path=""):
    if isinstance(value, dict):
        for key, item in value.items():
            yield from walk(item, f"{path}/{key}")
    elif isinstance(value, list):
        for index, item in enumerate(value):
            yield from walk(item, f"{path}/{index}")
    elif isinstance(value, str) and (value.startswith("file:///") or value.startswith("http")):
        yield path, value


data = json.loads(SAVE.read_text(encoding="utf-8"))
cache_names = [item.name.lower() for item in CACHE.iterdir() if item.is_file()]
refs = list(walk(data))
local, remote, missing_local, uncached_remote, invalid_png = [], [], [], [], []

for path, url in refs:
    if url.startswith("file:///"):
        local.append((path, url))
        filename = Path(unquote(url[8:]))
        if not filename.is_file():
            missing_local.append((path, str(filename)))
        elif filename.suffix.lower() == ".png":
            try:
                with Image.open(filename) as image:
                    image.verify()
            except Exception as error:
                invalid_png.append((str(filename), str(error)))
    else:
        remote.append((path, url))
        normalized = re.sub(r"[^a-z0-9]", "", url.lower())
        if not any(name.startswith(normalized) for name in cache_names):
            uncached_remote.append((path, url))

print(f"all resource references: {len(refs)}")
print(f"local refs: {len(local)} ({len(set(url for _, url in local))} unique)")
print(f"remote refs: {len(remote)} ({len(set(url for _, url in remote))} unique)")
print(f"missing local files: {len(missing_local)}")
print(f"invalid local PNGs: {len(invalid_png)}")
print(f"uncached remote refs: {len(uncached_remote)}")
for path, url in uncached_remote[:80]:
    print(f"UNCACHED {path}: {url}")
