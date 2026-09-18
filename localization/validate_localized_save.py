from __future__ import annotations

import json
import os
import re
from pathlib import Path
from urllib.parse import unquote


root = Path(__file__).parent
source = json.loads((root / "source" / "3689375010.original.json").read_text(encoding="utf-8"))
output_path = root / "output" / "THE_GANG_中文版.json"
output = json.loads(output_path.read_text(encoding="utf-8"))
raw = output_path.read_text(encoding="utf-8")
uris = set(re.findall(r'file:///[^\"]+', raw))
missing = [unquote(uri[8:]) for uri in uris if not os.path.isfile(unquote(uri[8:]))]

assert len(source["ObjectStates"]) == len(output["ObjectStates"])
assert not missing, missing
print(f"top-level objects: {len(output['ObjectStates'])}")
print(f"local URI references: {raw.count('file:///')} ({len(uris)} unique)")
print("all local assets present: yes")
