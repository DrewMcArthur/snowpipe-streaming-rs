---
name: plan
description: "Plan how to implement the specified feature."
---

Plan how to implement the specified feature.

Given the implementation details provided as an argument, do this:

1. Run `scripts/setup-plan.sh --json` from the repo root and parse JSON for FEATURE_SPEC, IMPL_PLAN, SPECS_DIR, BRANCH. Use absolute paths in all steps.
2. Read the feature specification (FEATURE_SPEC) and `memory/constitution.md`.
3. Copy `templates/plan-template.md` to IMPL_PLAN if not already present and fill in all sections using the specification and {ARGS} as Technical Context.
4. Ensure the plan includes phases and produces research.md, data-model.md (if needed), contracts/, quickstart.md as appropriate.
5. Report results with BRANCH and generated artifact paths.
