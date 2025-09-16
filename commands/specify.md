---
name: specify
description: "Start a new feature by creating a specification and feature branch."
---

Start a new feature by creating a specification and feature branch.

Given the feature description provided as an argument, do this:

1. Run the script `scripts/create-new-feature.sh --json "{ARGS}"` from the repo root and parse its JSON for BRANCH_NAME and SPEC_FILE. All future paths must be absolute.
2. Load `templates/spec-template.md` and create the initial specification at SPEC_FILE, filling in the placeholders with concrete details derived from the arguments while preserving headings/order.
3. Report completion with the new branch name and spec file path.
