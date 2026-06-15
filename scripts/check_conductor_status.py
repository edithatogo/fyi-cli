import os
import json
import re
from datetime import datetime

workspace_dir = r"C:\Users\60217257\OneDrive - Flinders\repos\legal-nz\fyi-cli"
conductor_dir = os.path.join(workspace_dir, ".conductor")

# 1. Check required files
required_files = [
    os.path.join(conductor_dir, "tech-stack.md"),
    os.path.join(conductor_dir, "workflow.md"),
    os.path.join(conductor_dir, "product.md")
]

for rf in required_files:
    if not os.path.exists(rf):
        print(f"ERROR: Missing setup file {rf}")
        exit(1)

tracks_dir = os.path.join(conductor_dir, "tracks")
if not os.path.exists(tracks_dir) or not os.path.isdir(tracks_dir):
    print("ERROR: tracks directory not found")
    exit(1)

tracks = [d for d in os.listdir(tracks_dir) if os.path.isdir(os.path.join(tracks_dir, d))]
if not tracks:
    print("ERROR: No tracks found")
    exit(1)

# Read product name
product_name = "fyi-cli"
product_path = os.path.join(conductor_dir, "product.md")
if os.path.exists(product_path):
    with open(product_path, "r", encoding="utf-8") as f:
        content = f.read()
        m = re.search(r"#\s+(.*)", content)
        if m:
            product_name = m.group(1).strip()

tracks_summary = []
overall_done = 0
overall_in_progress = 0
overall_pending = 0
overall_total = 0

all_in_progress_tasks = []
all_pending_tasks = []

# Gather stats
for track_id in sorted(tracks):
    track_path = os.path.join(tracks_dir, track_id)
    spec_path = os.path.join(track_path, "spec.md")
    plan_path = os.path.join(track_path, "plan.md")
    meta_path = os.path.join(track_path, "metadata.json")
    
    # Get status
    status = "pending"
    if os.path.exists(meta_path):
        try:
            with open(meta_path, "r", encoding="utf-8") as f:
                meta = json.load(f)
                status = meta.get("status", "pending")
        except:
            pass
            
    # Try parsing spec.md for Status
    blockers = []
    if os.path.exists(spec_path):
        with open(spec_path, "r", encoding="utf-8") as f:
            spec_content = f.read()
            # Look for blockers
            for line in spec_content.splitlines():
                if "BLOCKER:" in line or "❌" in line:
                    blockers.append(line.strip())
            
            # Look for Status in spec.md (e.g. ## Status or Status: value)
            status_match = re.search(r"##\s+Status\s*\n\s*([a-zA-Z\-_\s]+)", spec_content, re.IGNORECASE)
            if status_match:
                status = status_match.group(1).strip().lower()
            else:
                status_match2 = re.search(r"Status:\s*([a-zA-Z\-_\s]+)", spec_content, re.IGNORECASE)
                if status_match2:
                    status = status_match2.group(1).strip().lower()
    
    # Normalize status name
    if status in ["done", "completed"]:
        status = "completed"
    elif status in ["in-progress", "in_progress", "active"]:
        status = "in-progress"
    elif status in ["blocked", "block"]:
        status = "blocked"
    else:
        status = "pending"

    # Tasks
    done_count = 0
    ip_count = 0
    pending_count = 0
    
    current_task = None
    next_action = None
    
    if os.path.exists(plan_path):
        with open(plan_path, "r", encoding="utf-8") as f:
            plan_content = f.read()
            # Look for blockers in plan.md as well
            for line in plan_content.splitlines():
                if "BLOCKER:" in line or "❌" in line:
                    blockers.append(line.strip())
                
                # Checkboxes
                # - [x]
                # - [/]
                # - [ ]
                if "- [x]" in line or "- [X]" in line:
                    done_count += 1
                elif "- [/]" in line:
                    ip_count += 1
                    task_name = line.replace("- [/]", "").strip()
                    if not current_task:
                        current_task = task_name
                    all_in_progress_tasks.append((task_name, track_id))
                elif "- [ ]" in line:
                    pending_count += 1
                    task_name = line.replace("- [ ]", "").strip()
                    if not next_action:
                        next_action = task_name
                    all_pending_tasks.append((task_name, track_id))

    total = done_count + ip_count + pending_count
    
    # Fallback status check based on checkboxes if metadata/spec status is vague
    if status == "in-progress" and ip_count == 0 and pending_count == 0 and done_count > 0:
        status = "completed"
    
    overall_done += done_count
    overall_in_progress += ip_count
    overall_pending += pending_count
    overall_total += total
    
    tracks_summary.append({
        "track_id": track_id,
        "status": status,
        "done": done_count,
        "in_progress": ip_count,
        "pending": pending_count,
        "total": total,
        "current_task": current_task or "None",
        "next_action": next_action or "None",
        "blockers": ", ".join(blockers) if blockers else "None"
    })

# Project status
project_status = "On Track"
if any(t["status"] == "blocked" or t["blockers"] != "None" for t in tracks_summary):
    project_status = "Blocked"

# Print formatting
print(f"TIMESTAMP: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
print(f"PRODUCT: {product_name}")
print(f"PROJECT_STATUS: {project_status}")
print("---")
print(json.dumps(tracks_summary, indent=2))
print("---")
print(json.dumps({
    "overall_done": overall_done,
    "overall_in_progress": overall_in_progress,
    "overall_pending": overall_pending,
    "overall_total": overall_total,
    "all_in_progress": all_in_progress_tasks,
    "all_pending": all_pending_tasks
}, indent=2))
