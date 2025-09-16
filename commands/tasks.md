---
name: tasks
description: "Break down the plan into executable tasks."
---

Break down the plan into executable tasks.

1. Run `scripts/check-task-prerequisites.sh --json` and parse FEATURE_DIR and AVAILABLE_DOCS.
2. Read plan.md and any available docs to derive concrete tasks.
3. Use `templates/tasks-template.md` as the base, generating numbered tasks (T001, T002, …) with clear file paths and dependency notes. Mark tasks that can run in parallel with [P].
4. Write the result to FEATURE_DIR/tasks.md.
