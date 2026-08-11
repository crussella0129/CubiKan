#!/usr/bin/env bash
set -uo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
PROJECT_ROOT=$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)
BASE_REVISION=${SPRINT10_ACCEPTED_BASE:-bb257db8c62083ae8be4e8d77ec63762ba2e8fa8}
SKILL_ROOT=${SPRINT_LOOPS_SKILL_ROOT:-/mnt/c/Users/charl/Animus_Sprint_Loops/codex-cli/skills/sprint-loops}
BOOK_VALIDATOR="$SKILL_ROOT/scripts/check-book.sh"
PHASE_ROUTER="$SKILL_ROOT/scripts/current-phase.sh"
APPENDIX="$PROJECT_ROOT/docs/appendix/potential-derivative-projects.md"
TASKS="$PROJECT_ROOT/docs/work/tasks.md"
COMPLETED="$PROJECT_ROOT/docs/work/completed-tasks.md"
MARKDOWN_RESOLVER="$SCRIPT_DIR/markdown_resolver.py"
AUDIT_HELPER="$SCRIPT_DIR/audit_evidence.py"
AUDIT_EVIDENCE=${SPRINT10_AUDIT_EVIDENCE:-}

passed=0
failed=0
shopt -s nullglob

fail_check() {
  printf '%s\n' "$*" >&2
  return 1
}

require_literal() {
  local file=$1
  local literal=$2
  grep -Fq -- "$literal" "$file" || fail_check "$file: missing required text: $literal"
}

reject_literal() {
  local file=$1
  local literal=$2
  if grep -Fq -- "$literal" "$file"; then
    fail_check "$file: stale or forbidden text remains: $literal"
  fi
}

require_text() {
  local value=$1
  local literal=$2
  [[ "$value" == *"$literal"* ]] || fail_check "section missing required text: $literal"
}

extract_section() {
  local file=$1
  local start=$2
  local end=${3:-}
  awk -v start="$start" -v end="$end" '
    $0 == start { active = 1 }
    active && end != "" && $0 == end { exit }
    active { print }
  ' "$file"
}

extract_section_from_text() {
  local text=$1
  local start=$2
  awk -v start="$start" '
    $0 == start { active = 1 }
    active && /^## / && $0 != start { exit }
    active { print }
  ' <<<"$text"
}

catalog_entry() {
  local number=$1
  awk -v prefix="### $number. " '
    index($0, prefix) == 1 { active = 1 }
    active && /^### [0-9]+[.] / && index($0, prefix) != 1 { exit }
    active && /^## / { exit }
    active { print }
  ' "$APPENDIX"
}

catalog_fields() {
  local entry=$1
  awk '
    /^- \*\*[^*]+:\*\*/ {
      if (active) { printf "%s%c", record, 0 }
      record = $0
      active = 1
      next
    }
    active { record = record "\n" $0 }
    END { if (active) { printf "%s%c", record, 0 } }
  ' <<<"$entry"
}

book_state() {
  local intent=$1
  local matches=("$PROJECT_ROOT/docs/intents/${intent}"-*.md)
  [[ ${#matches[@]} -eq 1 ]] || fail_check "expected one chapter for $intent" || return
  sed -n 's/^- \*\*State:\*\* //p' "${matches[0]}"
}

field_value() {
  local text=$1
  local label=$2
  awk -v prefix="- **$label:** " '
    index($0, prefix) == 1 { print substr($0, length(prefix) + 1); exit }
  ' <<<"$text"
}

strip_code_ticks() {
  local value=$1
  value=${value#\`}
  value=${value%\`}
  printf '%s\n' "$value"
}

test_current_boundary_and_version_matrix_are_current() {
  local section
  section=$(extract_section "$APPENDIX" '## Current CubiKan boundary' '## Architectural layers')
  require_text "$section" 'four deliberately bounded surfaces' || return
  require_text "$section" '`cubikan-core` is a chain-agnostic Rust lifecycle kernel' || return
  require_text "$section" '`cubikan` is the experimental stateless, one-shot, in-memory JSON CLI' || return
  require_text "$section" '`cubikan-backend` is a synchronous, embedded SQLite Rust library' || return
  require_text "$section" '`cubikan-local` is the separate explicit-path durable JSON process adapter' || return
  require_text "$section" 'SQLite schema v1 for lifecycle storage and schema v2' || return
  require_text "$section" 'relationship contract v1 and projection query v1' || return
  require_text "$section" 'exposed through this Rust boundary only' || return
  require_text "$section" 'Protocol v1 remains lifecycle-only' || return
  require_text "$section" 'it does not expose relationship or projection' || return
  printf 'verified four surfaces, schema v1/v2, Rust relationship/projection v1, and lifecycle-only local protocol v1\n'
}

test_capability_map_statuses_match_book() {
  local expected intent actual
  for expected in \
    'INT-0008 proposed' \
    'INT-0009 realized' \
    'INT-0010 realized' \
    'INT-0011 proposed' \
    'INT-0012 realized'; do
    intent=${expected%% *}
    actual=$(book_state "$intent") || return
    [[ "$actual" == "${expected##* }" ]] || fail_check "$intent is $actual; expected ${expected##* }" || return
  done
  require_literal "$APPENDIX" 'INT-0009, INT-0010, and INT-0012 are `realized`; INT-0008 and' || return
  require_literal "$APPENDIX" 'INT-0011 remain `proposed` with no Work or Completion evidence.' || return
  reject_literal "$APPENDIX" 'INT-0008–INT-0012 are proposed' || return
  reject_literal "$APPENDIX" 'INT-0008-INT-0012 are proposed' || return
  reject_literal "$APPENDIX" 'INT-0008 through INT-0012 remain proposed' || return
  require_literal "$APPENDIX" 'Each datum has one canonical authority.' || return
  require_literal "$APPENDIX" 'The Book remains the semantic and historical realization authority' || return
  require_literal "$APPENDIX" '| External Git objects, pull requests, and CI records | Their source provider |' || return
  require_literal "$APPENDIX" '| Business records, PII, retention, RBAC, notifications, reports, and user experience | The bounded domain application |' || return
  require_literal "$APPENDIX" '| Business measurement definitions and authorization policy | The authoring process application or caller |' || return
  require_literal "$APPENDIX" 'depends on realized INT-0009' || return
  require_literal "$APPENDIX" 'depends on realized INT-0010' || return
  printf 'matched five Book states, realized dependencies, and canonical authorities\n'
}

test_supported_consumption_paths_and_authority_transfer_are_explicit() {
  local baseline authority
  baseline=$(extract_section "$APPENDIX" '## Safe CubiKan integration baseline' '## Data-authority map')
  authority=$(extract_section "$APPENDIX" '## Data-authority map' '## CubiKan capability status map')
  require_text "$baseline" 'explicitly pinned crate version' || return
  require_text "$baseline" 'does not create a cross-version Rust' || return
  require_text "$baseline" '`cubikan-backend` provides the local Rust lifecycle, relationship, and' || return
  require_text "$baseline" '`cubikan-local` protocol v1 provides only create, get,' || return
  require_text "$baseline" 'Provider-specific adapters, including Project Book and Git-host connectors,' || return
  require_text "$baseline" 'Any network transport or service likewise requires its' || return
  require_text "$authority" 'The Book remains the semantic and historical realization authority' || return
  require_text "$authority" 'authoritative only for the durable Intent Unit and' || return
  require_text "$authority" 'separately authorized projection or migration intent' || return
  require_text "$authority" 'Book and backend dual-write is prohibited' || return
  printf 'verified available backend/pinned-core paths, separately governed adapters, and one-way authority transfer\n'
}

test_advisory_and_storage_protocol_boundaries_remain_intact() {
  local advisory baseline
  advisory=$(extract_section "$APPENDIX" '# Potential Derivative Projects' '## Current CubiKan boundary')
  baseline=$(extract_section "$APPENDIX" '## Safe CubiKan integration baseline' '## Data-authority map')
  require_text "$advisory" 'Advisory status' || return
  require_text "$advisory" 'recommended derivative repository named in the catalog is asserted to exist,' || return
  require_text "$baseline" 'replace or dual-write that authority' || return
  require_text "$baseline" 'edit a CubiKan database directly or share writable backend storage' || return
  require_text "$baseline" 'persist or decode provisional core Serde as if it were a stable disk or wire' || return
  require_text "$baseline" 'availability does not imply cross-version compatibility' || return
  require_text "$baseline" 'infer authentication, authorization, tenancy, deployment, blockchain, or' || return
  require_text "$baseline" 'network-service behavior from the local boundaries' || return
  printf 'verified advisory status, storage/Serde prohibitions, compatibility limit, and explicit nonclaims\n'
}

test_catalog_remains_complete_and_preserves_edge_meaning() {
  local number entry field slug row theme
  local slugs=(
    cubikan-agent-ops
    cubikan-observatory
    animus-ledger
    cubikan-process-studio
    cubikan-skill-graph
    cubikan-org-app-kit
  )
  local fields=(
    'Problem and outcome'
    'Owned data'
    'Owned policy'
    'CubiKan interaction'
    'Prerequisites'
    'Creation trigger'
    'Explicit non-goals'
  )
  [[ $(grep -Ec '^### [1-6][.] ' "$APPENDIX") -eq 6 ]] || fail_check 'expected exactly six catalog entries' || return
  for number in 1 2 3 4 5 6; do
    entry=$(catalog_entry "$number")
    for field in "${fields[@]}"; do
      [[ $(grep -cF -- "- **$field:**" <<<"$entry") -eq 1 ]] || fail_check "catalog entry $number must contain exactly one $field field" || return
    done
  done
  for slug in "${slugs[@]}"; do
    [[ $(grep -Ec "^### [1-6][.] .*— \`$slug\`$" "$APPENDIX") -eq 1 ]] || fail_check "catalog entry missing or duplicated: $slug" || return
  done
  for field in \
    'manager-facing control plane' \
    'Intent-to-unit, unit-to-artifact, and artifact-to-unit trace views' \
    'agent-score reports' \
    'versioned process-definition packages' \
    'multi-board skill pipelines' \
    'bounded organizational applications' \
    'plan-versus-actual ledger'; do
    grep -Fiq -- "$field" "$APPENDIX" || fail_check "catalog lost required theme family: $field" || return
  done
  require_literal "$APPENDIX" '| CubiKan backend capability |' || return
  require_literal "$APPENDIX" '| Adapter |' || return
  require_literal "$APPENDIX" '| Derivative application |' || return
  require_literal "$APPENDIX" 'The latter four must not reuse `WorkflowEdge`' || return
  reject_literal "$APPENDIX" 'prevent lifecycle phase edges from becoming delegation/provenance/relationship/pipeline edges' || return
  local -A boundary=(
    [DV-01]='local common lifecycle backend available'
    [DV-02]='Book-to-unit mapping and future provenance remain namespaced'
    [DV-03]='INT-0008 owns durable associations; Git/Book/CI connectors remain adapters'
    [DV-04]='INT-0011 owns observations and deterministic evaluation of caller definitions'
    [DV-05]='data-authority map keeps the Book canonical until an explicit migration/projection intent'
    [DV-06]='INT-0012 boundary provides reusable relations/projections, never phase edges'
    [DV-07]='INT-0010 boundary supplies durable lifecycle commands and bounded queries'
    [DV-08]='Explicit relations available through realized INT-0012'
    [DV-09]='Blockchain remains an unselected adapter concern'
  )
  local -A disposition=(
    [DV-01]='Agent Ops coordinates work; Animus Ledger reconciles evidenced work'
    [DV-02]='Agent Ops owns manager/doer execution; Animus reads evidence but does not execute'
    [DV-03]='Observatory owns trace views and governed analytical inference'
    [DV-04]='Process Studio authors/governs definitions; Observatory analyzes results'
    [DV-05]='Animus derives reconciliation without dual-writing Book history'
    [DV-06]='Skill Graph owns executable DAG policy and multi-board routing'
    [DV-07]='Process Studio and the Organizational App Kit remain separate frontends/policy surfaces'
    [DV-08]='Merged across Agent Ops delegation, Skill Graph execution, and Animus reconciliation'
    [DV-09]='Deferred; no blockchain derivative repository is recommended'
  )
  for theme in DV-01 DV-02 DV-03 DV-04 DV-05 DV-06 DV-07 DV-08 DV-09; do
    [[ $(grep -cF -- "| \`$theme\` |" "$APPENDIX") -eq 1 ]] || fail_check "$theme traceability row must occur exactly once" || return
    row=$(grep -F -- "| \`$theme\` |" "$APPENDIX")
    require_text "$row" "${boundary[$theme]}" || return
    require_text "$row" "${disposition[$theme]}" || return
  done
  printf 'verified six complete entries, seven theme families, nine exact responsibility mappings, and phase-edge non-conflation\n'
}

test_catalog_prerequisites_use_realized_capabilities() {
  local number entry flattened intent field field_flat
  for number in 1 2 3 4 5 6; do
    entry=$(catalog_entry "$number")
    flattened=${entry//$'\n'/ }
    for intent in INT-0009 INT-0010 INT-0012; do
      [[ "$entry" == *"$intent"* ]] || continue
      if [[ ! "$flattened" =~ [Rr]ealized.{0,160}${intent} &&
            ! "$flattened" =~ ${intent}.{0,120}(available|exists|supplying) ]]; then
        fail_check "catalog entry $number does not describe $intent as an available realized capability"
        return
      fi
    done
    mapfile -d '' -t entry_fields < <(catalog_fields "$entry")
    for field in "${entry_fields[@]}"; do
      [[ "$field" == *INT-0009* || "$field" == *INT-0010* || "$field" == *INT-0012* ]] || continue
      [[ "$field" == '- **Related intents:**'* ]] && continue
      field_flat=${field//$'\n'/ }
      if grep -Eiq '(waits?|waiting)[[:space:]]+(for|on)[^.]{0,120}INT-(0009|0010|0012)|requires?[[:space:]]+realization[[:space:]]+of[[:space:]]+INT-(0009|0010|0012)|before[[:space:]]+INT-(0009|0010|0012)[^.]{0,80}(exists|is realized)|INT-(0009|0010|0012)[^.]{0,100}(remains?|is|are|be)[[:space:]]+(proposed|future|unrealized|unavailable)' <<<"$field_flat"; then
        fail_check "catalog entry $number contains contradictory stale realized-capability wording: ${field%%$'\n'*}"
        return
      fi
      for intent in INT-0009 INT-0010 INT-0012; do
        [[ "$field" == *"$intent"* ]] || continue
        if [[ ! "$field_flat" =~ [Rr]ealized.{0,160}${intent} &&
              ! "$field_flat" =~ [Aa]vailable.{0,160}${intent} &&
              ! "$field_flat" =~ (from|use|uses|using|through).{0,80}${intent} &&
              ! "$field_flat" =~ ${intent}.{0,160}(available|exist|exists|supplying|owns|canonical|relations|boundary|query|primitives|backed) ]]; then
          fail_check "catalog entry $number field does not locally establish $intent as available: ${field%%$'\n'*}"
          return
        fi
      done
    done
  done
  reject_literal "$APPENDIX" 'realized INT-0008' || return
  reject_literal "$APPENDIX" 'realized INT-0011' || return
  reject_literal "$APPENDIX" 'waits for INT-0009' || return
  reject_literal "$APPENDIX" 'waits for INT-0010' || return
  reject_literal "$APPENDIX" 'waits for INT-0012' || return
  require_literal "$APPENDIX" 'Full bidirectional provenance still' || return
  require_literal "$APPENDIX" 'needs INT-0008; its revision and durable-query prerequisites are already' || return
  require_literal "$APPENDIX" 'shared KPI release additionally requires realization of INT-0011' || return
  require_literal "$APPENDIX" 'a read-only Book/Git prototype may establish that need without' || return
  require_literal "$APPENDIX" 'depending on or writing to the available durable backend' || return
  reject_literal "$APPENDIX" 'before a durable backend exists' || return
  printf 'reviewed all catalog references to realized and still-proposed capabilities\n'
}

test_derivative_creation_remains_unauthorized() {
  local number entry trigger flattened_catalog
  require_literal "$APPENDIX" 'Each primary recommendation below is a boundary proposal, not a repository' || return
  require_literal "$APPENDIX" 'creation request.' || return
  require_literal "$APPENDIX" 'Every recommendation remains conditional' || return
  require_literal "$APPENDIX" 'requires its own selected intent and explicit authorization' || return
  require_literal "$APPENDIX" 'named owners who' || return
  require_literal "$APPENDIX" 'accept its data, policy, and security boundaries' || return
  require_literal "$APPENDIX" 'compatible versioned' || return
  require_literal "$APPENDIX" 'satisfaction of its entry-specific creation trigger' || return
  require_literal "$APPENDIX" 'the repository exists nor that its creation is scheduled' || return
  require_literal "$APPENDIX" 'none is created here' || return
  flattened_catalog=$(extract_section "$APPENDIX" '## Recommended repository catalog' '## Retained-theme traceability')
  flattened_catalog=${flattened_catalog//$'\n'/ }
  if [[ "$flattened_catalog" =~ (All|Each|Every|The|Any|A)[[:space:]]+([^.!?]{0,100}[[:space:]])?(recommended[[:space:]]+|named[[:space:]]+)?(derivative[[:space:]]+)?repositor(y|ies)[[:space:]]+(already[[:space:]]+)?(exist|exists|is[[:space:]]+authorized|are[[:space:]]+authorized|is[[:space:]]+scheduled|are[[:space:]]+scheduled) ]]; then
    fail_check 'catalog contains an added global repository existence/authorization/scheduling claim'
    return
  fi
  if [[ "$flattened_catalog" =~ (cubikan-agent-ops|cubikan-observatory|animus-ledger|cubikan-process-studio|cubikan-skill-graph|cubikan-org-app-kit)[\`]*[[:space:]]+(already[[:space:]]+)?(exists|is[[:space:]]+authorized|is[[:space:]]+scheduled) ]]; then
    fail_check 'catalog contains an added named-repository existence/authorization/scheduling claim'
    return
  fi
  for number in 1 2 3 4 5 6; do
    entry=$(catalog_entry "$number")
    [[ $(grep -cF -- '- **Creation trigger:**' <<<"$entry") -eq 1 ]] || fail_check "catalog entry $number must have exactly one creation trigger" || return
    trigger=$(awk '/^- \*\*Creation trigger:\*\*/ { active = 1 } active && /^- \*\*Separation rationale:\*\*/ { exit } active { print }' <<<"$entry")
    [[ "$trigger" == *'Create only when'* || "$trigger" == *'Create when'* ]] || fail_check "catalog entry $number creation trigger is not conditional" || return
  done
  printf 'verified appendix noncommitment and six conditional creation triggers\n'
}

test_sequence_and_open_questions_exclude_completed_foundations() {
  local sequence questions non_goals term
  sequence=$(extract_section "$APPENDIX" '## Sequencing and creation gates' '## Merged, deferred, and rejected alternatives')
  questions=$(extract_section "$APPENDIX" '## Open questions' '## Appendix-wide non-goals')
  non_goals=$(extract_section "$APPENDIX" '## Appendix-wide non-goals')
  require_text "$sequence" 'Completed reusable backend foundation' || return
  require_text "$sequence" 'INT-0009 and its dependent' || return
  require_text "$sequence" 'INT-0010 are realized' || return
  require_text "$sequence" 'Advanced multi-board relations may use realized' || return
  require_text "$sequence" 'INT-0012' || return
  if grep -Eiq 'whether (INT-0009|INT-0010|INT-0012)|will (INT-0009|INT-0010|INT-0012).*be realized' <<<"$questions"; then
    fail_check 'open questions still ask whether a completed foundation will be realized'
    return
  fi
  for term in compatibility authorization provenance privacy deployment blockchain; do
    grep -Eiq "$term" <<<"$questions" || fail_check "open questions lost unresolved $term policy" || return
  done
  grep -Eiq 'security|authentication|permission|secret|sandbox|trust|key custody' <<<"$questions" || fail_check 'open questions lost unresolved security policy' || return
  grep -Eiq 'evidence|artifact|observation|measurement' <<<"$questions" || fail_check 'open questions lost unresolved evidence policy' || return
  require_text "$questions" 'Which first Process Studio and organizational-app journeys justify their UI' || return
  require_text "$questions" 'What manager/doer identity, permission, approval, cancellation, secret, and' || return
  require_text "$questions" 'Which observation clocks, sources, denominators, windows, units, late-arrival,' || return
  require_text "$questions" 'Which relationship authorization, definition-lifecycle, historical or' || return
  require_text "$questions" 'What skill admission, executor trust, sandbox, artifact, retry/idempotency,' || return
  require_text "$questions" 'What unit of account, valuation, trust, correction, close/reopen, anti-gaming,' || return
  require_text "$non_goals" 'redefine any realized intent (INT-0001–INT-0006, INT-0009, INT-0010, or' || return
  require_text "$non_goals" 'INT-0012), rewrite superseded INT-0007' || return
  require_text "$non_goals" 'advance INT-0008 or INT-0011 out' || return
  reject_literal "$APPENDIX" 'redefine any realized intent, including INT-0001–INT-0007' || return
  printf 'verified realized foundations are closed, eight policy themes remain open, and superseded INT-0007 is not redefined\n'
}

test_backlog_moves_once_to_completion_ledger() {
  local original_item maint_entry task entry integrated_commit normative_commit description
  local pending_tree pending_entry evidence_subject evidence_commit previous_line=0 current_line path
  local restoration=b170e107d08ac1855d6b1be82fbf1ebe25a22f3a
  local -A integrated=(
    [T-1001]=d725411e0bf4c97437544e28c604e48f0c1badbf
    [T-1002]=a4c14cfcaccc23afeebafe28490b63b0683d17e8
    [T-1003]=a3e6aec3afe739091d03103744a82d89ad1c467b
    [T-1004]=336b4e48e791f9a7d0a25e5de84c9404c3e266d2
    [T-1005]=99864da63fc9a51b24ead1d5792c4d6b7f706207
    [T-1006]=9517dc17797f25e7a2d8f924abf1b5d51fb62e5a
    [T-1007]=a7ed48992897c8463ba6cc729e944398c8ae8779
  )
  local -A legacy_subject=(
    [T-1001]='sprint-0: T-001 Correct current CubiKan surfaces and versions'
    [T-1002]='sprint-0: T-002 Correct capability status and authority maps'
    [T-1003]='sprint-0: T-003 Correct safe integration boundaries'
    [T-1004]='sprint-0: T-004 Refresh derivative capability prerequisites'
    [T-1005]='sprint-0: T-005 Preserve derivative creation governance'
    [T-1006]='sprint-0: T-006 Correct derivative sequencing and open questions'
    [T-1007]='sprint-0: T-007 Close derivative appendix maintenance backlog'
  )

  original_item=$(git -C "$PROJECT_ROOT" show "$BASE_REVISION:docs/work/tasks.md" | grep -F -- '- (backlog) [INT-0007] Refresh the non-authoritative derivative-project appendix')
  [[ -n "$original_item" ]] || fail_check 'accepted base does not contain the exact maintenance backlog item' || return
  [[ $(grep -cF -- "$original_item" "$TASKS" || true) -eq 0 ]] || fail_check 'exact maintenance backlog item remains queued' || return
  [[ $(grep -cF -- '## MAINT-001 (post-Sprint 9)' "$COMPLETED") -eq 1 ]] || fail_check 'MAINT-001 must occur exactly once' || return
  maint_entry=$(extract_section "$COMPLETED" '## MAINT-001 (post-Sprint 9)' '## T-1001 (sprint 10)')
  require_text "$maint_entry" 'originating realized backlog authority' || return
  require_text "$maint_entry" 'superseding [INT-0013]' || return
  require_text "$maint_entry" '**Commit:** `a7ed48992897c8463ba6cc729e944398c8ae8779`' || return
  [[ "$maint_entry" != *"$restoration"* ]] || fail_check 'authority restoration is attributed to MAINT-001' || return

  for task in T-1001 T-1002 T-1003 T-1004 T-1005 T-1006 T-1007; do
    [[ $(grep -cF -- "## $task (sprint 10)" "$COMPLETED") -eq 1 ]] || fail_check "$task must occur exactly once" || return
    current_line=$(grep -nF -- "## $task (sprint 10)" "$COMPLETED" | cut -d: -f1)
    (( current_line > previous_line )) || fail_check "completion ledger is out of dependency order at $task" || return
    previous_line=$current_line
    entry=$(extract_section "$COMPLETED" "## $task (sprint 10)")
    integrated_commit=$(strip_code_ticks "$(field_value "$entry" 'Integrated implementation commit')")
    normative_commit=$(strip_code_ticks "$(field_value "$entry" 'Commit')")
    description=$(field_value "$entry" 'Description')
    [[ "$integrated_commit" == "${integrated[$task]}" ]] || fail_check "$task integrated mapping is $integrated_commit; expected ${integrated[$task]}" || return
    [[ "$normative_commit" =~ ^[0-9a-f]{40}$ ]] || fail_check "$task normative Commit is not a full SHA" || return
    [[ "$normative_commit" != "$integrated_commit" && "$normative_commit" != "$restoration" ]] || fail_check "$task normative Commit has invalid attribution" || return
    git -C "$PROJECT_ROOT" cat-file -e "$integrated_commit^{commit}" || fail_check "$task integrated commit is missing" || return
    git -C "$PROJECT_ROOT" cat-file -e "$normative_commit^{commit}" || fail_check "$task normative commit is missing" || return
    git -C "$PROJECT_ROOT" merge-base --is-ancestor "$normative_commit" HEAD || fail_check "$task normative commit is not on tested history" || return
    [[ $(git -C "$PROJECT_ROOT" show -s --format='%s' "$integrated_commit") == "${legacy_subject[$task]}" ]] || fail_check "$task legacy subject does not match locked mapping" || return
    [[ $(git -C "$PROJECT_ROOT" show -s --format='%s' "$normative_commit") == "sprint-10: $task $description" ]] || fail_check "$task normative commit was not created by the Book helper contract" || return

    pending_tree=$(git -C "$PROJECT_ROOT" show "$normative_commit:docs/work/completed-tasks.md") || return
    pending_entry=$(extract_section_from_text "$pending_tree" "## $task (sprint 10)")
    require_text "$pending_entry" '**Commit:** PENDING' || return
    evidence_subject="sprint-10: $task record commit evidence"
    mapfile -t evidence_commits < <(git -C "$PROJECT_ROOT" log --format='%H' --fixed-strings --grep="$evidence_subject" HEAD)
    [[ ${#evidence_commits[@]} -eq 1 ]] || fail_check "$task must have exactly one helper evidence commit" || return
    evidence_commit=${evidence_commits[0]}
    [[ $(git -C "$PROJECT_ROOT" show -s --format='%s' "$evidence_commit") == "$evidence_subject" ]] || fail_check "$task evidence commit subject is inexact" || return
    [[ $(git -C "$PROJECT_ROOT" rev-parse "$evidence_commit^") == "$normative_commit" ]] || fail_check "$task evidence commit is not the direct helper child" || return

    mapfile -t task_paths < <(git -C "$PROJECT_ROOT" show --format= --name-only "$normative_commit")
    printf '%s\n' "${task_paths[@]}" | grep -Fxq 'docs/work/tasks.md' || fail_check "$task helper commit omitted tasks ledger" || return
    printf '%s\n' "${task_paths[@]}" | grep -Fxq 'docs/work/completed-tasks.md' || fail_check "$task helper commit omitted completion ledger" || return
    for path in "${task_paths[@]}"; do
      [[ -n "$path" ]] || continue
      case "$task:$path" in
        T-1001:docs/work/tasks.md|T-1001:docs/work/completed-tasks.md|T-1001:docs/intents/INT-0013-maintain-derivative-ecosystem-current-state.md) ;;
        T-1006:docs/work/tasks.md|T-1006:docs/work/completed-tasks.md|T-1006:docs/appendix/potential-derivative-projects.md) ;;
        T-1002:docs/work/tasks.md|T-1002:docs/work/completed-tasks.md|T-1003:docs/work/tasks.md|T-1003:docs/work/completed-tasks.md|T-1004:docs/work/tasks.md|T-1004:docs/work/completed-tasks.md|T-1005:docs/work/tasks.md|T-1005:docs/work/completed-tasks.md|T-1007:docs/work/tasks.md|T-1007:docs/work/completed-tasks.md) ;;
        *) fail_check "$task helper commit contains unexpected path: $path"; return ;;
      esac
    done
  done
  [[ $(git -C "$PROJECT_ROOT" show -s --format='%s' "$restoration") == 'Restore Book-v2 Sprint Loop authority' ]] || fail_check 'authority-restoration commit is missing or changed' || return
  printf 'verified exact backlog closure, singular MAINT-001, seven ordered Book commits, and seven locked legacy mappings\n'
}

test_appendix_matches_current_project_and_backend_guides() {
  local root_readme="$PROJECT_ROOT/README.md"
  local backend="$PROJECT_ROOT/crates/cubikan-backend/README.md"
  local local_guide="$PROJECT_ROOT/crates/cubikan-local/README.md"
  require_literal "$root_readme" '`cubikan-core` is a chain-agnostic Rust library' || return
  require_literal "$root_readme" '`cubikan` executable is an experimental stateless' || return
  require_literal "$root_readme" '`cubikan-backend` adds a synchronous, embedded SQLite boundary' || return
  require_literal "$root_readme" 'separate `cubikan-local` executable' || return
  require_literal "$backend" '| SQLite schema | 1 and 2 |' || return
  require_literal "$backend" '| Relationship contract | 1 |' || return
  require_literal "$backend" '| Projection query | 1 |' || return
  require_literal "$backend" '| [`cubikan-local` JSON protocol](../cubikan-local/README.md) | 1 | Five lifecycle operations only |' || return
  require_literal "$backend" '`open` never migrates.' || return
  require_literal "$local_guide" 'This local protocol stays version 1' || return
  require_literal "$local_guide" 'SQLite schemas v1 and v2, relationship contract' || return
  require_literal "$local_guide" 'does not acquire relationship or projection' || return
  require_literal "$local_guide" 'Rust-only relationship boundary' || return
  require_literal "$APPENDIX" 'Protocol v1 remains lifecycle-only' || return
  require_literal "$APPENDIX" 'relationship contract v1 and projection query v1' || return
  require_literal "$APPENDIX" 'availability does not imply cross-version compatibility' || return
  require_literal "$APPENDIX" 'future schema migration, and compatibility' || return
  printf 'cross-checked surfaces, versions, migration, Rust-only relationships, and protocol limits across four guides\n'
}

test_appendix_links_and_book_navigation_resolve() {
  local validator_output resolver_output validator_status resolver_status
  [[ -f "$BOOK_VALIDATOR" ]] || fail_check "missing Book validator: $BOOK_VALIDATOR" || return
  [[ -f "$MARKDOWN_RESOLVER" ]] || fail_check "missing Markdown resolver: $MARKDOWN_RESOLVER" || return
  validator_output=$(bash "$BOOK_VALIDATOR" "$PROJECT_ROOT" 2>&1)
  validator_status=$?
  resolver_output=$(python3 "$MARKDOWN_RESOLVER" "$PROJECT_ROOT" 2>&1)
  resolver_status=$?
  printf '%s\n%s\n' "$validator_output" "$resolver_output"
  (( validator_status == 0 )) || fail_check 'authoritative Book validator failed' || return
  (( resolver_status == 0 )) || fail_check 'Markdown path/fragment or intent-navigation resolution failed' || return
  [[ "$validator_output" == *'valid v2 Book (13 intent chapters)'* ]] || fail_check "unexpected Book validation output: $validator_output" || return
}

test_documentation_maintenance_scope_is_non_product() {
  local path
  local allowed_required=(
    docs/SUMMARY.md
    docs/appendix/potential-derivative-projects.md
    docs/intents/INT-0007-define-cubikan-derivative-ecosystem.md
    docs/intents/INT-0013-maintain-derivative-ecosystem-current-state.md
    docs/sprints/s10/sprint-meta.md
    docs/sprints/s10/sprint-plans/build-plan.md
    docs/sprints/s10/sprint-plans/critique.md
    docs/sprints/s10/sprint-plans/test-plan.md
    docs/sprints/s10/sprint-research/research-report.md
    docs/work/tasks.md
    docs/work/completed-tasks.md
  )
  git -C "$PROJECT_ROOT" cat-file -e "$BASE_REVISION^{commit}" || fail_check "missing accepted base $BASE_REVISION" || return
  git -C "$PROJECT_ROOT" merge-base --is-ancestor "$BASE_REVISION" HEAD || fail_check "$BASE_REVISION is not an ancestor of HEAD" || return
  mapfile -t changed < <(git -C "$PROJECT_ROOT" diff --name-only "$BASE_REVISION..HEAD")
  for path in "${changed[@]}"; do
    case "$path" in
      docs/SUMMARY.md|docs/appendix/potential-derivative-projects.md|docs/intents/INT-0007-define-cubikan-derivative-ecosystem.md|docs/intents/INT-0013-maintain-derivative-ecosystem-current-state.md|docs/work/tasks.md|docs/work/completed-tasks.md|docs/sprints/s10/*|docs/sprints/s10/**) ;;
      *) fail_check "out-of-scope accepted-base path: $path"; return ;;
    esac
  done
  for path in "${allowed_required[@]}"; do
    printf '%s\n' "${changed[@]}" | grep -Fxq -- "$path" || fail_check "accepted-base diff is missing required path: $path" || return
  done
  git -C "$PROJECT_ROOT" diff --quiet "$BASE_REVISION..HEAD" -- '*.rs' Cargo.toml Cargo.lock ':(glob)crates/*/Cargo.toml' .github docs/work/remote-profile.md || fail_check 'Rust, manifest/lockfile, CI, or remote-profile scope changed' || return
  mapfile -t previous_intents < <(git -C "$PROJECT_ROOT" ls-tree -r --name-only "$BASE_REVISION" docs/intents/INT-*.md)
  for path in "${previous_intents[@]}"; do
    [[ "$path" == docs/intents/INT-0007-define-cubikan-derivative-ecosystem.md ]] && continue
    git -C "$PROJECT_ROOT" diff --quiet "$BASE_REVISION..HEAD" -- "$path" || fail_check "pre-existing intent semantics changed outside INT-0007: $path" || return
  done
  require_literal "$PROJECT_ROOT/docs/intents/INT-0007-define-cubikan-derivative-ecosystem.md" '- **State:** superseded' || return
  require_literal "$PROJECT_ROOT/docs/intents/INT-0007-define-cubikan-derivative-ecosystem.md" 'blob/b6daf73cf4c12e496466ebdcb393b3204e7ffeb7/docs/appendix/potential-derivative-projects.md' || return
  [[ -z $(git -C "$PROJECT_ROOT" ls-files 'sprints/**' 'agent-tasks/**' decisions.md) ]] || fail_check 'tracked legacy root Sprint Loop authority remains' || return
  for path in "$PROJECT_ROOT/sprints" "$PROJECT_ROOT/agent-tasks" "$PROJECT_ROOT/decisions.md"; do
    [[ ! -e "$path" ]] || fail_check "duplicate writable legacy authority exists: $path" || return
  done
  git -C "$PROJECT_ROOT" diff --check "$BASE_REVISION..HEAD" || fail_check 'accepted-base diff fails git diff --check' || return
  printf 'verified bounded documentation/Book provenance, immutable product/CI/remote surfaces, and no legacy authority\n'
}

verify_workspace_regression_gates() {
  printf 'cargo gate: +stable fmt --all -- --check\n'
  cargo +stable fmt --all -- --check || return
  printf 'cargo gate: +stable clippy --workspace --all-targets --all-features --locked --offline -- -D warnings\n'
  CARGO_NET_OFFLINE=true cargo +stable clippy --workspace --all-targets --all-features --locked --offline -- -D warnings || return
  printf 'cargo gate: +stable RUSTFLAGS=-D warnings check --workspace --all-targets --all-features --locked --offline\n'
  CARGO_NET_OFFLINE=true RUSTFLAGS='-D warnings' cargo +stable check --workspace --all-targets --all-features --locked --offline || return
  printf 'cargo gate: +stable test --workspace --all-targets --all-features --locked --offline\n'
  CARGO_NET_OFFLINE=true cargo +stable test --workspace --all-targets --all-features --locked --offline || return
  printf 'cargo gate: +stable test --workspace --doc --all-features --locked --offline\n'
  CARGO_NET_OFFLINE=true cargo +stable test --workspace --doc --all-features --locked --offline || return
  printf 'verified all five offline workspace regression gates\n'
}

verify_repository_hygiene() {
  local validator_output resolver_output phase
  git -C "$PROJECT_ROOT" diff --check || fail_check 'working-tree diff fails git diff --check' || return
  git -C "$PROJECT_ROOT" diff --check "$BASE_REVISION..HEAD" || fail_check 'accepted-base diff fails git diff --check' || return
  validator_output=$(bash "$BOOK_VALIDATOR" "$PROJECT_ROOT" 2>&1) || fail_check "$validator_output" || return
  resolver_output=$(python3 "$MARKDOWN_RESOLVER" "$PROJECT_ROOT" 2>&1) || fail_check "$resolver_output" || return
  python3 "$MARKDOWN_RESOLVER" --self-test || fail_check 'Markdown reference-style parser self-test failed' || return
  python3 "$AUDIT_HELPER" --self-test || fail_check 'bounded action-audit negative self-tests failed' || return
  phase=$(bash "$PHASE_ROUTER" 2>&1) || fail_check "$phase" || return
  [[ "$validator_output" == *'valid v2 Book (13 intent chapters)'* ]] || fail_check "unexpected Book validator output: $validator_output" || return
  [[ "$resolver_output" == *'13 Book intents; 0 errors'* ]] || fail_check "unexpected Markdown resolver output: $resolver_output" || return
  [[ "$phase" == test || "$phase" == loop ]] || fail_check "unexpected Sprint Loop phase: $phase" || return
  [[ -z $(git -C "$PROJECT_ROOT" ls-files 'sprints/**' 'agent-tasks/**' decisions.md) ]] || fail_check 'legacy writable authority is tracked' || return
  for legacy_path in "$PROJECT_ROOT/sprints" "$PROJECT_ROOT/agent-tasks" "$PROJECT_ROOT/decisions.md"; do
    [[ ! -e "$legacy_path" ]] || fail_check "legacy writable authority exists: $legacy_path" || return
  done
  printf '%s\n%s\nlayout-router: %s\n' "$validator_output" "$resolver_output" "$phase"
}

verify_no_derivative_repository_operations() {
  local branch fetch_url push_url remote head commit_count slug
  local slugs=(
    cubikan-agent-ops
    cubikan-observatory
    animus-ledger
    cubikan-process-studio
    cubikan-skill-graph
    cubikan-org-app-kit
  )
  [[ -n "$AUDIT_EVIDENCE" ]] || fail_check 'audit mode requires --audit-evidence FILE (or SPRINT10_AUDIT_EVIDENCE)' || return
  [[ -r "$AUDIT_EVIDENCE" ]] || fail_check "audit evidence is not readable: $AUDIT_EVIDENCE" || return
  branch=$(git -C "$PROJECT_ROOT" branch --show-current)
  [[ "$branch" == dev ]] || fail_check "tested work branch is $branch; expected dev" || return
  mapfile -t remotes < <(git -C "$PROJECT_ROOT" remote)
  [[ ${#remotes[@]} -eq 1 && ${remotes[0]} == origin ]] || fail_check 'local Git configuration must contain only origin' || return
  fetch_url=$(git -C "$PROJECT_ROOT" remote get-url origin)
  push_url=$(git -C "$PROJECT_ROOT" remote get-url --push origin)
  for remote in "$fetch_url" "$push_url"; do
    case "$remote" in
      https://github.com/crussella0129/CubiKan|https://github.com/crussella0129/CubiKan.git|git@github.com:crussella0129/CubiKan.git) ;;
      *) fail_check "unexpected CubiKan remote target: $remote"; return ;;
    esac
    for slug in "${slugs[@]}"; do
      [[ "${remote,,}" != *"/${slug,,}"* && "${remote,,}" != *":${slug,,}"* ]] || fail_check "local remote targets derivative slug: $remote" || return
    done
  done
  head=$(git -C "$PROJECT_ROOT" rev-parse HEAD)
  commit_count=$(git -C "$PROJECT_ROOT" rev-list --count "$BASE_REVISION..HEAD")
  printf 'tested_head=%s\nbranch=%s\norigin.fetch=%s\norigin.push=%s\nsprint_commit_count=%s\n' "$head" "$branch" "$fetch_url" "$push_url" "$commit_count"
  git -C "$PROJECT_ROOT" log --reverse --format='sprint_commit=%H%x09%s' "$BASE_REVISION..HEAD"
  python3 "$AUDIT_HELPER" "$PROJECT_ROOT" "$AUDIT_EVIDENCE" || return
  printf 'verified bounded durable evidence: only CubiKan/dev mutations; no targeted derivative repository was found and no derivative mutation was recorded\n'
}

run_check() {
  local name=$1
  local output status
  output=$($name 2>&1)
  status=$?
  if (( status == 0 )); then
    printf 'PASS %s\n' "$name"
    ((passed += 1))
  else
    printf 'FAIL %s\n' "$name"
    ((failed += 1))
  fi
  if [[ -n "$output" ]]; then
    while IFS= read -r line; do
      printf '  %s\n' "$line"
    done <<<"$output"
  fi
}

unit_checks=(
  test_current_boundary_and_version_matrix_are_current
  test_capability_map_statuses_match_book
  test_supported_consumption_paths_and_authority_transfer_are_explicit
  test_advisory_and_storage_protocol_boundaries_remain_intact
  test_catalog_remains_complete_and_preserves_edge_meaning
  test_catalog_prerequisites_use_realized_capabilities
  test_derivative_creation_remains_unauthorized
  test_sequence_and_open_questions_exclude_completed_foundations
  test_backlog_moves_once_to_completion_ledger
)

integration_checks=(
  test_appendix_matches_current_project_and_backend_guides
  test_appendix_links_and_book_navigation_resolve
  test_documentation_maintenance_scope_is_non_product
)

e2e_checks=(
  verify_workspace_regression_gates
  verify_repository_hygiene
  verify_no_derivative_repository_operations
)

usage() {
  cat <<'USAGE'
Usage:
  documentation-checks.sh structural
  documentation-checks.sh rust
  documentation-checks.sh audit --audit-evidence FILE
  documentation-checks.sh all --audit-evidence FILE
  documentation-checks.sh list

structural runs all unit/integration checks plus repository hygiene without
network access. rust runs the five Cargo gates in offline mode. audit consumes
one bounded JSON evidence file; it performs no provider calls. all runs all 15
finalized checks and requires the audit file.
USAGE
}

mode=${1:-structural}
shift || true
while (( $# > 0 )); do
  case "$1" in
    --audit-evidence)
      [[ $# -ge 2 ]] || { usage >&2; exit 2; }
      AUDIT_EVIDENCE=$2
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      printf 'unknown argument: %s\n' "$1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ "$mode" == list ]]; then
  printf '%s\n' "${unit_checks[@]}" "${integration_checks[@]}" "${e2e_checks[@]}"
  exit 0
fi

printf 'sprint-10-checks: mode=%s base=%s head=%s\n' "$mode" "$BASE_REVISION" "$(git -C "$PROJECT_ROOT" rev-parse HEAD)"
case "$mode" in
  structural)
    for check in "${unit_checks[@]}" "${integration_checks[@]}" verify_repository_hygiene; do
      run_check "$check"
    done
    ;;
  rust)
    run_check verify_workspace_regression_gates
    ;;
  audit)
    run_check verify_no_derivative_repository_operations
    ;;
  all)
    for check in "${unit_checks[@]}" "${integration_checks[@]}" "${e2e_checks[@]}"; do
      run_check "$check"
    done
    ;;
  *)
    printf 'unknown mode: %s\n' "$mode" >&2
    usage >&2
    exit 2
    ;;
esac

printf 'sprint-10-checks: %d passed, %d failed, %d selected\n' "$passed" "$failed" "$((passed + failed))"
(( failed == 0 ))
