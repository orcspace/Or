import json,re
from pathlib import Path
root=Path(__file__).resolve().parents[1]
path=root/"docs/requirements.json"
records=json.loads(path.read_text(encoding="utf-8"))
clauses={}
for name in ("AGENTS.md","docs/MCP.md","docs/SPEC-AUDIT.md"):
    for line in (root/name).read_text(encoding="utf-8").splitlines():
        match=re.search(r"<!-- (REQ-(?:[MA])?\d{2}-\d{3}) -->",line)
        if match: clauses[match.group(1)]=re.sub(r"<!-- REQ-(?:[MA])?\d{2}-\d{3} -->","",line).strip()
for record in records:
    record["clause"]=clauses[record["id"]]
path.write_text(json.dumps(records,ensure_ascii=False,indent=2)+"\n",encoding="utf-8")
