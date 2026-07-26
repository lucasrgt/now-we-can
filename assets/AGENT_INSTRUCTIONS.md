<!-- notyet:instructions:start -->
## Not Yet

This repository uses Not Yet (`notyet`) to turn evidence-backed conditional
deferments into obligations that reappear when their machine-checkable cue is
observed.

1. At task start and after context changes, run `notyet wake` with every event
   supplied by the host. Treat returned deferments as due work, not suggestions.
2. Resolve a completed deferment with `notyet resolve --id <id> --evidence
   "<proof>"`.
3. At task completion, the host must run `notyet collect` over the task, plan,
   final response, and diff. Agents must not add deferments manually.
4. Collection may preserve only a concrete action intentionally blocked by a
   currently false prerequisite and bound to an event, path, or file-content
   cue evidenced in the supplied task artifacts.
5. Aspirations, optional improvements, unfinished current scope, permanent
   behavior, vague future work, and invented cues are not deferments.
6. Run `notyet check` before completion with the same observed events. Exit code
   1 means a due deferment remains unresolved. Exit code 2 means the check did
   not complete and must never be reported as a pass.

Tests and roadmaps do not replace Not Yet. The host owns collection and
delivery; neither may depend on an agent voluntarily remembering the tool.
<!-- notyet:instructions:end -->
