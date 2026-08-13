#!/usr/bin/bash
set -euo pipefail

readonly TOOL_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
readonly PROJECT_ROOT="$(cd -- "$TOOL_DIR/../.." && pwd -P)"
readonly VERIFIER="$TOOL_DIR/verify-pins.sh"
readonly PINS="$PROJECT_ROOT/chain/pins.toml"
readonly TEST_ROOT="$(mktemp -d)"
trap 'rm -rf -- "$TEST_ROOT"' EXIT

declare -a IDENTITY_CASES=()
declare -A IDENTITY_CLASSES=()
declare -A IDENTITY_ACCEPTED=()

die() {
    printf 'verify-pins.test: %s\n' "$*" >&2
    exit 1
}

mutated_value() {
    local value="$1" first
    case "$value" in
        *[!0-9]*) ;;
        '') ;;
        *) printf '%s\n' "$((10#$value + 1))"; return ;;
    esac
    if [[ "$value" =~ ^[0-9a-f]{40}$ || "$value" =~ ^[0-9a-f]{64}$ ]]; then
        first="${value:0:1}"
        if [[ "$first" == 0 ]]; then
            printf '1%s\n' "${value:1}"
        else
            printf '0%s\n' "${value:1}"
        fi
        return
    fi
    printf '%s-mutated\n' "$value"
}

mutate_pin() {
    local source="$1" destination="$2" section="$3" key="$4" replacement="$5"
    awk -v wanted_section="$section" -v wanted_key="$key" -v replacement="$replacement" '
        /^\[[^]]+\]$/ {
            current = substr($0, 2, length($0) - 2)
        }
        /^[A-Za-z0-9_.-]+[[:space:]]*=/ {
            split($0, parts, "=")
            candidate = parts[1]
            gsub(/[[:space:]]/, "", candidate)
            if (current == wanted_section && candidate == wanted_key) {
                print candidate " = \"" replacement "\""
                changed++
                next
            }
        }
        { print }
        END { if (changed != 1) exit 42 }
    ' "$source" >"$destination" || die "could not mutate [$section] $key exactly once"
}

expect_static_rejection() {
    local label="$1" copy="$2" output="$TEST_ROOT/$label.output"
    if "$VERIFIER" --test-static "$copy" >"$output" 2>&1; then
        die "$label mutation passed static preflight"
    fi
    if /usr/bin/grep -F 'static-preflight-passed' "$output" >/dev/null; then
        die "$label mutation reached the dependent-execution boundary"
    fi
    if /usr/bin/grep -F 'dependent-boundary-not-entered' "$output" >/dev/null; then
        die "$label mutation emitted an identity success marker"
    fi
}

pin_value() {
    local section="$1" key="$2"
    /usr/bin/awk -v section="[$section]" -v key="$key" '
        $0 == section { active = 1; next }
        /^\[/ { active = 0 }
        active && $0 ~ "^" key "[[:space:]]*=" {
            line = $0
            sub("^" key "[[:space:]]*=[[:space:]]*", "", line)
            if (line !~ /^"[^"]*"$/) exit 2
            sub(/^"/, "", line); sub(/"$/, "", line)
            print line
            found++
        }
        END { if (found != 1) exit 1 }
    ' "$PINS" || die "could not read [$section] $key exactly once"
}

add_identity_case() {
    local class="$1" subject="$2" mutation_kind="$3"
    [[ -z "${IDENTITY_CLASSES[$class]:-}" ]] || die "duplicate identity-test class: $class"
    [[ "$subject" == /* ]] || die "identity-test subject is not absolute for $class: $subject"
    IDENTITY_CLASSES["$class"]=1
    IDENTITY_CASES+=("$class|$subject|$mutation_kind")
}

initialize_identity_cases() {
    local revision node_version node_platform zombienet_revision rusqlite_version tool foundation_snapshot
    revision="$(pin_value polkadot_sdk revision)"
    zombienet_revision="$(pin_value zombienet revision)"
    node_version="$(pin_value node version)"
    node_platform="$(pin_value node platform)"
    rusqlite_version="$(pin_value rusqlite version)"
    foundation_snapshot="$PROJECT_ROOT/$(pin_value foundation snapshot_path)"

    # Downloaded release archives and binaries are hundreds of megabytes in
    # aggregate. Their rejection subjects are sparse same-size files, so this
    # test never duplicates the authoritative payloads.
    add_identity_case sdk_archive "$PROJECT_ROOT/chain/.cache/downloads/polkadot-sdk-$revision.tar.gz" sparse
    add_identity_case zombienet_archive "$PROJECT_ROOT/chain/.cache/downloads/zombienet-$zombienet_revision.tar.gz" sparse
    add_identity_case node_archive "$PROJECT_ROOT/chain/.cache/downloads/node-v$node_version-$node_platform.tar.xz" sparse
    add_identity_case rusqlite_archive "$PROJECT_ROOT/chain/.cache/downloads/rusqlite-$rusqlite_version.crate" sparse
    add_identity_case asset_polkadot "$PROJECT_ROOT/chain/.cache/downloads/polkadot" sparse
    add_identity_case asset_polkadot_parachain "$PROJECT_ROOT/chain/.cache/downloads/polkadot-parachain" sparse
    add_identity_case asset_polkadot_omni_node "$PROJECT_ROOT/chain/.cache/downloads/polkadot-omni-node" sparse
    add_identity_case asset_chain_spec_builder "$PROJECT_ROOT/chain/.cache/downloads/chain-spec-builder" sparse
    add_identity_case asset_frame_omni_bencher "$PROJECT_ROOT/chain/.cache/downloads/frame-omni-bencher" sparse

    add_identity_case argv_grammar "$PROJECT_ROOT/$(pin_value repository_tools argv_grammar_path)" copy
    add_identity_case argv_normalizer "$PROJECT_ROOT/$(pin_value repository_tools argv_normalizer_path)" copy
    add_identity_case loopback_wrapper "$PROJECT_ROOT/$(pin_value repository_tools loopback_wrapper_path)" copy
    add_identity_case sealed_exec "$PROJECT_ROOT/$(pin_value repository_tools sealed_exec_path)" copy

    for tool in \
        bash unshare ip ss netcat git rustup env awk sha256sum dd mount stat tar \
        patch diff find sort iconv uname dirname readlink sed grep head wc cp rm \
        mkdir mktemp curl chmod mv python; do
        add_identity_case "host_$tool" "$(pin_value host_tools "${tool}_path")" fake-file
    done

    add_identity_case root_manifest "$foundation_snapshot/payload/root/workspace-manifest.toml" copy
    add_identity_case root_lock "$foundation_snapshot/payload/root/lockfile.lock" copy
    add_identity_case gitignore "$foundation_snapshot/payload/root/gitignore" copy
    add_identity_case root_toolchain "$foundation_snapshot/payload/root/rust-toolchain.toml" copy
    add_identity_case root_ci "$foundation_snapshot/payload/root/github/workflows/ci.yml" copy
    add_identity_case chain_manifest "$foundation_snapshot/payload/chain/workspace-manifest.toml" copy
    add_identity_case chain_lock "$foundation_snapshot/payload/chain/lockfile.lock" copy
    add_identity_case chain_toolchain "$foundation_snapshot/payload/chain/rust-toolchain.toml" copy
    add_identity_case chain_pallet_manifest "$foundation_snapshot/payload/chain/pallets/cubikan/manifest.toml" copy
    add_identity_case chain_pallet_source "$foundation_snapshot/payload/chain/pallets/cubikan/src/lib.rs" copy
    add_identity_case chain_runtime_manifest "$foundation_snapshot/payload/chain/runtime/manifest.toml" copy
    add_identity_case chain_runtime_build "$foundation_snapshot/payload/chain/runtime/build.rs" copy
    add_identity_case chain_runtime_source "$foundation_snapshot/payload/chain/runtime/src/lib.rs" copy
    add_identity_case root_dependency_manifest "$PROJECT_ROOT/$(pin_value root_dependency_contract manifest_path)" copy
    add_identity_case root_dependency_lock "$PROJECT_ROOT/$(pin_value root_dependency_contract lock_path)" copy
    add_identity_case root_dependency_source "$PROJECT_ROOT/$(pin_value root_dependency_contract source_path)" copy
    add_identity_case rusqlite_patch "$PROJECT_ROOT/$(pin_value rusqlite patch_path)" copy
    add_identity_case runtime_wasm "$PROJECT_ROOT/$(pin_value foundation runtime_wasm_reference_path)" sparse
    add_identity_case foundation_snapshot_tree "$foundation_snapshot" fake-tree
    add_identity_case rusqlite_vendor_tree "$PROJECT_ROOT/$(pin_value rusqlite vendor_path)" fake-tree
    add_identity_case rust_toolchain_tree "$(pin_value rust toolchain_path)" fake-tree
    add_identity_case wasm_target_tree "$(pin_value rust wasm_target_path)" fake-tree
}

assert_identity_class_inventory_is_closed() {
    local expected="$TEST_ROOT/expected-identity-classes" actual="$TEST_ROOT/actual-identity-classes"
    local spec class subject mutation_kind
    for spec in "${IDENTITY_CASES[@]}"; do
        IFS='|' read -r class subject mutation_kind <<<"$spec"
        printf '%s\n' "$class"
    done | LC_ALL=C /usr/bin/sort >"$expected"

    /usr/bin/awk '
        /^verify_identity_subject\(\) \{/ { active = 1; next }
        active && /^[[:space:]]*esac$/ { exit }
        active {
            line = $0
            sub(/^[[:space:]]+/, "", line)
            if (line ~ /^[a-z0-9_]+(\|[a-z0-9_]+)*\)/) {
                sub(/\).*/, "", line)
                count = split(line, classes, "|")
                for (item = 1; item <= count; item++) print classes[item]
            } else if (line ~ /\$\{class#host_\}.*=~[[:space:]]*\^\(/) {
                sub(/^.*=~[[:space:]]*\^\(/, "", line)
                sub(/\)\$.*$/, "", line)
                count = split(line, classes, "|")
                for (item = 1; item <= count; item++) print "host_" classes[item]
            }
        }
    ' "$VERIFIER" | LC_ALL=C /usr/bin/sort >"$actual"

    if ! /usr/bin/diff -u -- "$actual" "$expected"; then
        die 'identity-test cases do not exactly cover the verifier class inventory'
    fi
    [[ "$(/usr/bin/wc -l <"$expected")" == 69 ]] || die 'closed identity inventory no longer contains 69 classes'
}

expect_identity_acceptance() {
    local class="$1" subject="$2" output="$TEST_ROOT/$class.exact.output"
    local marker="identity-checked:$class:dependent-boundary-not-entered"
    if ! "$VERIFIER" --test-identity "$class" "$subject" >"$output" 2>&1; then
        /usr/bin/sed -n '1,12p' "$output" >&2
        die "$class rejected its authoritative subject: $subject"
    fi
    /usr/bin/grep -F "$marker" "$output" >/dev/null || die "$class exact check omitted its boundary marker"
    IDENTITY_ACCEPTED["$class"]=1
}

make_identity_mutant() {
    local class="$1" subject="$2" mutation_kind="$3" destination="$4" size
    case "$mutation_kind" in
        copy)
            /usr/bin/cp -- "$subject" "$destination"
            printf '\nidentity-test-mutation:%s\n' "$class" >>"$destination"
            ;;
        sparse)
            size="$(/usr/bin/stat -c '%s' -- "$subject")"
            /usr/bin/truncate --size="$size" "$destination"
            ;;
        fake-file)
            printf 'identity-test-fake-executable:%s\n' "$class" >"$destination"
            ;;
        fake-tree)
            /usr/bin/mkdir -p -- "$destination"
            printf 'identity-test-fake-tree:%s\n' "$class" >"$destination/mutated"
            ;;
        *) die "unknown mutation kind for $class: $mutation_kind" ;;
    esac
}

expect_identity_rejection() {
    local class="$1" subject="$2" output="$TEST_ROOT/$class.mutated.output"
    if "$VERIFIER" --test-identity "$class" "$subject" >"$output" 2>&1; then
        die "$class mutation passed its identity check"
    fi
    if /usr/bin/grep -F 'dependent-boundary-not-entered' "$output" >/dev/null; then
        die "$class mutation reached the dependent-execution boundary"
    fi
}

extract_function() {
    local function_name="$1"
    awk -v signature="${function_name}() {" '
        $0 == signature { active = 1 }
        active { print }
        active && /^}$/ { exit }
    ' "$VERIFIER"
}

extract_locked_dispatch() {
    /usr/bin/awk '
        /^verify_complete_static_inputs$/ { active = 1 }
        active { print }
        active && /all exact pins, offline closures, native build, and Wasm build verified/ { exit }
    ' "$VERIFIER"
}

unique_line_containing() {
    local source="$1" needle="$2"
    printf '%s\n' "$source" | /usr/bin/awk -v needle="$needle" '
        index($0, needle) { line = NR; matches++ }
        END {
            if (matches != 1) exit 1
            print line
        }
    '
}

assert_unique_order() {
    local source="$1" earlier="$2" later="$3" label="$4" earlier_line later_line
    earlier_line="$(unique_line_containing "$source" "$earlier")" || die "$label: expected exactly one earlier verifier command: $earlier"
    later_line="$(unique_line_containing "$source" "$later")" || die "$label: expected exactly one later verifier command: $later"
    ((earlier_line < later_line)) || die "$label: verifier check no longer precedes dependent work"
}

write_registry_fixture_harness() {
    local destination="$1"
    {
        printf '%s\n' '#!/usr/bin/bash -p' 'set -euo pipefail'
        printf '%s\n' \
            'readonly PROJECT_ROOT="${FIXTURE_PROJECT_ROOT:?}"' \
            'readonly CARGO_HOME="${FIXTURE_CARGO_HOME:?}"' \
            'readonly DEPENDENT_SENTINEL="${FIXTURE_DEPENDENT_SENTINEL:?}"' \
            'die() { printf "fixture: %s\\n" "$*" >&2; exit 1; }' \
            'pin() {' \
            '    case "$1:$2" in' \
            '        root_dependency_contract:lock_path) printf "%s\\n" fixture-contract/Cargo.lock ;;' \
            '        foundation:snapshot_path) printf "%s\\n" foundation ;;' \
            '        *) die "unexpected pin lookup: $1.$2" ;;' \
            '    esac' \
            '}' \
            'sha256_file() { local output; output="$(/usr/bin/sha256sum -- "$1")"; printf "%s\\n" "${output%% *}"; }' \
            'require_hash() {' \
            '    local path="$1" expected="$2" actual' \
            '    actual="$(sha256_file "$path")"' \
            '    [[ "$actual" == "$expected" ]] || die "fixture SHA-256 mismatch"' \
            '}'
        extract_function verify_registry_archive_cache
        printf '%s\n' 'verify_registry_archive_cache' '/usr/bin/touch -- "$DEPENDENT_SENTINEL"'
    } >"$destination"
    /usr/bin/chmod 0700 -- "$destination"
}

write_sdk_fixture_harness() {
    local destination="$1" function_name="$2" path_pin="$3"
    {
        printf '%s\n' '#!/usr/bin/bash -p' 'set -euo pipefail'
        printf '%s\n' \
            'readonly PROJECT_ROOT="${FIXTURE_PROJECT_ROOT:?}"' \
            'readonly TASK_TMP="${FIXTURE_TASK_TMP:?}"' \
            'readonly SDK_ARCHIVE="${FIXTURE_SDK_ARCHIVE:?}"' \
            'readonly DEPENDENT_SENTINEL="${FIXTURE_DEPENDENT_SENTINEL:?}"' \
            'die() { printf "fixture: %s\\n" "$*" >&2; exit 1; }' \
            'pin() {' \
            '    case "$1:$2" in' \
            '        polkadot_sdk:revision) printf "%s\\n" fixture-revision ;;' \
            '        polkadot_sdk:archive_size) printf "%s\\n" "$FIXTURE_ARCHIVE_SIZE" ;;' \
            '        polkadot_sdk:archive_sha256) printf "%s\\n" "$FIXTURE_ARCHIVE_SHA256" ;;' \
            "        polkadot_sdk:$path_pin) printf '%s\\n' \"\$FIXTURE_CANONICAL_RELATIVE_PATH\" ;;" \
            '        host_tools:git_path) printf "%s\\n" /usr/bin/git ;;' \
            '        *) die "unexpected pin lookup: $1.$2" ;;' \
            '    esac' \
            '}' \
            'sha256_file() { local output; output="$(/usr/bin/sha256sum -- "$1")"; printf "%s\\n" "${output%% *}"; }' \
            'require_hash() {' \
            '    local path="$1" expected="$2" actual' \
            '    actual="$(sha256_file "$path")"' \
            '    [[ "$actual" == "$expected" ]] || die "fixture SHA-256 mismatch"' \
            '}' \
            'require_size() {' \
            '    local path="$1" expected="$2" actual' \
            '    actual="$(/usr/bin/stat -c %s -- "$path")"' \
            '    [[ "$actual" == "$expected" ]] || die "fixture size mismatch"' \
            '}' \
            'safe_extract() {' \
            '    local archive="$1" destination="$2" compression="$3"' \
            '    [[ "$compression" == gz ]] || die "unexpected fixture compression"' \
            '    /usr/bin/mkdir -p -- "$destination"' \
            '    /usr/bin/tar -xzf "$archive" -C "$destination" --no-same-owner --no-same-permissions' \
            '}'
        extract_function "$function_name"
        printf '%s\n' "$function_name" '/usr/bin/touch -- "$DEPENDENT_SENTINEL"'
    } >"$destination"
    /usr/bin/chmod 0700 -- "$destination"
}

expect_fixture_mutation_rejection() {
    local label="$1" harness="$2" project_root="$3" task_tmp="$4" archive="$5"
    local archive_size="$6" archive_sha256="$7" canonical_path="$8" sentinel="$9"
    /usr/bin/rm -f -- "$sentinel"
    if FIXTURE_PROJECT_ROOT="$project_root" \
        FIXTURE_TASK_TMP="$task_tmp" \
        FIXTURE_SDK_ARCHIVE="$archive" \
        FIXTURE_ARCHIVE_SIZE="$archive_size" \
        FIXTURE_ARCHIVE_SHA256="$archive_sha256" \
        FIXTURE_CANONICAL_RELATIVE_PATH="$canonical_path" \
        FIXTURE_DEPENDENT_SENTINEL="$sentinel" \
        /usr/bin/bash -p "$harness" >"$TEST_ROOT/$label.output" 2>&1; then
        die "$label mutation passed its canonical identity check"
    fi
    [[ ! -e "$sentinel" ]] || die "$label mutation reached dependent execution"
}

test_static_path_contains_no_dependent_execution() {
    local static_functions='' function_name function_body branch
    for function_name in \
        verify_complete_static_inputs verify_local_static_inputs \
        verify_pin_contract verify_repository_tool_bytes verify_host_tool_bytes \
        verify_lock_and_manifest_bytes verify_root_dependency_contract_bytes \
        verify_rusqlite_bytes verify_downloads; do
        function_body="$(extract_function "$function_name")"
        [[ -n "$function_body" ]] || die "$function_name function is missing"
        static_functions+="$function_body"$'\n'
    done
    branch="$(awk '
        /^if \[\[ "\$mode" == teststatic \]\]; then$/ { active = 1 }
        active { print }
        active && /^fi$/ { exit }
    ' "$VERIFIER")"
    [[ -n "$branch" ]] || die 'test-static dispatch branch is missing'

    # Static mode may parse literals and hash checked repository files. It must
    # return before archives, downloaded executables, toolchains, Cargo, or the
    # namespace/dependent-build path can be reached.
    if printf '%s\n%s\n' "$static_functions" "$branch" | rg -n \
        '^[[:space:]]*(download_exact|fetch_all|safe_extract|verify_repository_tool_behavior|verify_host_tool_behavior|verify_sdk_and_scaffold|verify_node_and_zombienet|verify_asset_capabilities|verify_rusqlite_reconstruction|verify_toolchain|verify_feature_closures_and_builds)([[:space:]]|$)|/(cargo|rustup|curl|tar|patch)([[:space:]]|$)|^[[:space:]]*exec([[:space:]]|$)|\$LOOPBACK' \
        | /usr/bin/grep -v '\$LOOPBACK_CONTENT' >/dev/null; then
        die 'test-static path contains a dependent execution command'
    fi
    [[ "$branch" == *'verify_complete_static_inputs'* ]] || die 'test-static does not call its complete static preflight'
    [[ "$branch" == *'static-preflight-passed'* ]] || die 'test-static success marker is missing'
    [[ "$branch" == *'exit 0'* ]] || die 'test-static does not terminate before normal verification'
}

test_pin_document_mutations_fail_before_execution() {
    local section key value replacement copy label count=0
    while IFS=$'\034' read -r section key value; do
        [[ -n "$key" ]] || continue
        count=$((count + 1))
        label="$(printf '%03d' "$count")-${section:-top}-$key"
        label="${label//[^A-Za-z0-9_.-]/_}"
        copy="$TEST_ROOT/$label.toml"
        replacement="$(mutated_value "$value")"
        mutate_pin "$PINS" "$copy" "$section" "$key" "$replacement"
        expect_static_rejection "$label" "$copy"
    done < <(awk '
        /^\[[^]]+\]$/ {
            section = substr($0, 2, length($0) - 2)
            next
        }
        /^[A-Za-z0-9_.-]+[[:space:]]*=[[:space:]]*"[^"]*"[[:space:]]*$/ {
            line = $0
            key = line
            sub(/[[:space:]]*=.*/, "", key)
            value = line
            sub(/^[^=]*=[[:space:]]*"/, "", value)
            sub(/"[[:space:]]*$/, "", value)
            printf "%s\034%s\034%s\n", section, key, value
        }
    ' "$PINS")
    [[ $count -ge 100 ]] || die "identity inventory unexpectedly small: $count literals"

    copy="$TEST_ROOT/duplicate-key.toml"
    awk '{ print; if ($0 == "channel = \"1.93.0\"") print }' "$PINS" >"$copy"
    expect_static_rejection duplicate-key "$copy"

    copy="$TEST_ROOT/missing-key.toml"
    awk '$0 != "channel = \"1.93.0\""' "$PINS" >"$copy"
    expect_static_rejection missing-key "$copy"

    copy="$TEST_ROOT/unknown-key.toml"
    awk '{ print } END { print "unknown_identity = \"not-authoritative\"" }' "$PINS" >"$copy"
    expect_static_rejection unknown-key "$copy"

    copy="$TEST_ROOT/nonliteral-key.toml"
    awk '{ if ($0 == "channel = \"1.93.0\"") print "channel = 1.93"; else print }' "$PINS" >"$copy"
    expect_static_rejection nonliteral-key "$copy"

    copy="$TEST_ROOT/placeholder.toml"
    mutate_pin "$PINS" "$copy" rust channel PENDING
    expect_static_rejection placeholder "$copy"
}

test_chain_dependency_toolchain_metadata_and_artifact_pins_are_exact() {
    local spec class subject mutation_kind
    assert_identity_class_inventory_is_closed
    for spec in "${IDENTITY_CASES[@]}"; do
        IFS='|' read -r class subject mutation_kind <<<"$spec"
        expect_identity_acceptance "$class" "$subject"
    done
}

test_nested_chain_workspace_has_only_allowlisted_root_delta() {
    local output
    test_static_path_contains_no_dependent_execution
    output="$("$VERIFIER" --test-static "$PINS")"
    [[ "$output" == *'static-preflight-passed'* ]] || die 'exact pins and workspace delta did not pass static preflight'
}

test_pin_verifier_rejects_every_identity_mismatch() {
    local spec class subject mutation_kind mutant
    for spec in "${IDENTITY_CASES[@]}"; do
        IFS='|' read -r class subject mutation_kind <<<"$spec"
        [[ "${IDENTITY_ACCEPTED[$class]:-}" == 1 ]] || die "$class mutation was attempted before its authoritative subject passed"
        if [[ "$mutation_kind" == fake-tree ]]; then
            mutant="$TEST_ROOT/$class.mutated-tree"
        else
            mutant="$TEST_ROOT/$class.mutated-file"
        fi
        make_identity_mutant "$class" "$subject" "$mutation_kind" "$mutant"
        expect_identity_rejection "$class" "$mutant"
    done
    test_pin_document_mutations_fail_before_execution
}

test_canonical_registry_archive_cache_identities_precede_source_materialization() {
    local fixture="$TEST_ROOT/registry-cache" project_root cargo_home archive checksum harness sentinel
    local lock body locked_dispatch
    project_root="$fixture/project"
    cargo_home="$fixture/cargo-home"
    archive="$cargo_home/registry/cache/fixture-index/fixture-crate-1.2.3.crate"
    harness="$fixture/registry-harness.sh"
    sentinel="$fixture/dependent-executed"
    /usr/bin/mkdir -p -- \
        "$project_root/chain" "$project_root/fixture-contract" \
        "$project_root/foundation/payload/root" "$project_root/foundation/payload/chain" \
        "$cargo_home/registry/cache/fixture-index"
    printf '%s\n' 'canonical registry archive fixture' >"$archive"
    checksum="$(/usr/bin/sha256sum -- "$archive")"
    checksum="${checksum%% *}"
    lock="$fixture/lockfile"
    printf '[[package]]\nname = "fixture-crate"\nversion = "1.2.3"\nchecksum = "%s"\n' "$checksum" >"$lock"
    /usr/bin/cp -- "$lock" "$project_root/Cargo.lock"
    /usr/bin/cp -- "$lock" "$project_root/chain/Cargo.lock"
    /usr/bin/cp -- "$lock" "$project_root/fixture-contract/Cargo.lock"
    /usr/bin/cp -- "$lock" "$project_root/foundation/payload/root/lockfile.lock"
    /usr/bin/cp -- "$lock" "$project_root/foundation/payload/chain/lockfile.lock"
    write_registry_fixture_harness "$harness"

    FIXTURE_PROJECT_ROOT="$project_root" \
        FIXTURE_CARGO_HOME="$cargo_home" \
        FIXTURE_DEPENDENT_SENTINEL="$sentinel" \
        /usr/bin/bash -p "$harness"
    [[ -e "$sentinel" ]] || die 'exact canonical registry archive fixture never reached its dependent boundary'

    /usr/bin/rm -f -- "$sentinel"
    printf '%s\n' 'mutated canonical bytes' >>"$archive"
    if FIXTURE_PROJECT_ROOT="$project_root" \
        FIXTURE_CARGO_HOME="$cargo_home" \
        FIXTURE_DEPENDENT_SENTINEL="$sentinel" \
        /usr/bin/bash -p "$harness" >"$fixture/mutated.output" 2>&1; then
        die 'mutated canonical registry archive passed its lockfile identity check'
    fi
    [[ ! -e "$sentinel" ]] || die 'mutated canonical registry archive reached source materialization'

    body="$(extract_function prepare_verified_cargo_sources)"
    [[ -n "$body" ]] || die 'prepare_verified_cargo_sources function is missing'
    assert_unique_order "$body" 'verify_registry_archive_cache' \
        '/usr/bin/gnurm -rf -- "$CARGO_HOME/registry/src"' \
        'registry archive cache validation order'
    locked_dispatch="$(extract_locked_dispatch)"
    [[ -n "$locked_dispatch" ]] || die 'locked verifier dispatch is missing'
    assert_unique_order "$locked_dispatch" 'prepare_verified_cargo_sources' \
        'verify_feature_closures_and_builds' \
        'verified canonical Cargo sources before dependent builds'
}

test_sdk_cargo_git_database_matches_verified_release_archive_before_source_materialization() {
    local fixture="$TEST_ROOT/sdk-git-db" project_root task_tmp source_tree archive archive_size archive_sha256
    local repository git_db canonical_path harness sentinel body
    project_root="$fixture/project"
    task_tmp="$fixture/tmp"
    source_tree="$fixture/release/polkadot-sdk-fixture-revision"
    archive="$fixture/polkadot-sdk-fixture-revision.tar.gz"
    repository="$fixture/repository"
    canonical_path='canonical/sdk-git-db'
    git_db="$project_root/$canonical_path"
    harness="$fixture/sdk-git-db-harness.sh"
    sentinel="$fixture/dependent-executed"
    /usr/bin/mkdir -p -- "$source_tree/nested" "$task_tmp" "$repository" "$project_root/canonical"
    printf '%s\n' 'verified SDK release fixture' >"$source_tree/nested/source.txt"
    /usr/bin/tar -czf "$archive" -C "$fixture/release" "polkadot-sdk-fixture-revision"
    archive_size="$(/usr/bin/stat -c '%s' -- "$archive")"
    archive_sha256="$(/usr/bin/sha256sum -- "$archive")"
    archive_sha256="${archive_sha256%% *}"

    /usr/bin/git -C "$repository" init --quiet
    /usr/bin/git -C "$repository" config user.name fixture
    /usr/bin/git -C "$repository" config user.email fixture@example.invalid
    /usr/bin/mkdir -p -- "$repository/nested"
    /usr/bin/cp -- "$source_tree/nested/source.txt" "$repository/nested/source.txt"
    /usr/bin/git -C "$repository" add nested/source.txt
    /usr/bin/git -C "$repository" commit --quiet -m fixture
    /usr/bin/git -C "$repository" tag fixture-revision
    /usr/bin/git clone --quiet --bare "$repository" "$git_db"
    write_sdk_fixture_harness "$harness" verify_sdk_git_database cargo_git_db_path

    FIXTURE_PROJECT_ROOT="$project_root" FIXTURE_TASK_TMP="$task_tmp" \
        FIXTURE_SDK_ARCHIVE="$archive" FIXTURE_ARCHIVE_SIZE="$archive_size" \
        FIXTURE_ARCHIVE_SHA256="$archive_sha256" FIXTURE_CANONICAL_RELATIVE_PATH="$canonical_path" \
        FIXTURE_DEPENDENT_SENTINEL="$sentinel" /usr/bin/bash -p "$harness"
    [[ -e "$sentinel" ]] || die 'exact SDK Cargo git database fixture never reached its dependent boundary'

    printf '%s\n' 'mutated canonical SDK git object' >"$repository/nested/source.txt"
    /usr/bin/git -C "$repository" add nested/source.txt
    /usr/bin/git -C "$repository" commit --quiet -m mutated
    /usr/bin/git -C "$repository" push --quiet "$git_db" HEAD:refs/heads/mutated
    /usr/bin/git --git-dir="$git_db" update-ref refs/tags/fixture-revision refs/heads/mutated
    expect_fixture_mutation_rejection sdk-git-db "$harness" "$project_root" "$task_tmp" \
        "$archive" "$archive_size" "$archive_sha256" "$canonical_path" "$sentinel"

    body="$(extract_function prepare_verified_cargo_sources)"
    [[ -n "$body" ]] || die 'prepare_verified_cargo_sources function is missing'
    assert_unique_order "$body" 'verify_sdk_git_database' \
        '/usr/bin/gnurm -rf -- "$CARGO_HOME/registry/src"' \
        'SDK Cargo git database validation order'
    assert_unique_order "$body" 'CARGO_NET_OFFLINE=true "$rustup" run "$(pin rust channel)" cargo fetch \' 'verify_materialized_sdk_checkout' \
        'offline SDK materialization before checkout validation order'
}

test_materialized_sdk_checkout_matches_verified_release_archive_before_build() {
    local fixture="$TEST_ROOT/sdk-checkout" project_root task_tmp source_tree archive archive_size archive_sha256
    local checkout canonical_path harness sentinel body
    project_root="$fixture/project"
    task_tmp="$fixture/tmp"
    source_tree="$fixture/release/polkadot-sdk-fixture-revision"
    archive="$fixture/polkadot-sdk-fixture-revision.tar.gz"
    canonical_path='canonical/sdk-checkout'
    checkout="$project_root/$canonical_path"
    harness="$fixture/sdk-checkout-harness.sh"
    sentinel="$fixture/dependent-executed"
    /usr/bin/mkdir -p -- "$source_tree/nested" "$task_tmp" "$checkout"
    printf '%s\n' 'verified materialized SDK fixture' >"$source_tree/nested/source.txt"
    /usr/bin/tar -czf "$archive" -C "$fixture/release" "polkadot-sdk-fixture-revision"
    archive_size="$(/usr/bin/stat -c '%s' -- "$archive")"
    archive_sha256="$(/usr/bin/sha256sum -- "$archive")"
    archive_sha256="${archive_sha256%% *}"
    /usr/bin/mkdir -p -- "$checkout/nested"
    /usr/bin/cp -- "$source_tree/nested/source.txt" "$checkout/nested/source.txt"
    write_sdk_fixture_harness "$harness" verify_materialized_sdk_checkout cargo_checkout_path

    FIXTURE_PROJECT_ROOT="$project_root" FIXTURE_TASK_TMP="$task_tmp" \
        FIXTURE_SDK_ARCHIVE="$archive" FIXTURE_ARCHIVE_SIZE="$archive_size" \
        FIXTURE_ARCHIVE_SHA256="$archive_sha256" FIXTURE_CANONICAL_RELATIVE_PATH="$canonical_path" \
        FIXTURE_DEPENDENT_SENTINEL="$sentinel" /usr/bin/bash -p "$harness"
    [[ -e "$sentinel" ]] || die 'exact materialized SDK checkout fixture never reached its dependent boundary'

    printf '%s\n' 'mutated canonical checkout bytes' >"$checkout/nested/source.txt"
    expect_fixture_mutation_rejection sdk-checkout "$harness" "$project_root" "$task_tmp" \
        "$archive" "$archive_size" "$archive_sha256" "$canonical_path" "$sentinel"

    body="$(extract_function verify_feature_closures_and_builds)"
    [[ -n "$body" ]] || die 'verify_feature_closures_and_builds function is missing'
    assert_unique_order "$body" 'verify_materialized_sdk_checkout' 'materialize_foundation_snapshot' \
        'materialized SDK checkout validation order'
    assert_unique_order "$body" 'materialize_foundation_snapshot' 'cargo check --manifest-path "$work/Cargo.toml"' \
        'foundation materialization before dependent build order'
}

test_locked_isolation_dispatch_is_tristate() {
    local fixture="$TEST_ROOT/isolation-dispatch" wrapper harness returned dispatched body
    fixture="$TEST_ROOT/isolation-dispatch"
    wrapper="$fixture/probe-wrapper.sh"
    harness="$fixture/harness.sh"
    returned="$fixture/returned"
    dispatched="$fixture/dispatched"
    /usr/bin/mkdir -p -- "$fixture"

    cat >"$wrapper" <<'EOF'
#!/usr/bin/bash -p
set -euo pipefail
case "${1:-}" in
    __cubikan_loopback_bound_memory_v1__)
        hint=$2
        shift 4
        [[ "${1:-}" == __cubikan_loopback_clean_entry_v1__ ]] || exit 96
        shift
        fixture_dir=${hint%/*}
        case "${1:-}" in
            --assert-current-isolated)
                status="$(<"$fixture_dir/probe-status")"
                case "$status" in
                    0) printf '%s\n' 'loopback-netns: current-process-isolation=verified' >&2 ;;
                    125) printf '%s\n' 'loopback-netns: current-process-isolation=outside-clean-launcher' >&2 ;;
                    *) printf '%s\n' 'loopback-netns: partial-isolation-fixture' >&2 ;;
                esac
                exit "$status"
                ;;
            --)
                shift
                printf '%s\n' "$@" >"$fixture_dir/dispatched"
                ;;
            *) exit 97 ;;
        esac
        ;;
    --assert-current-isolated)
        case "$(<"${0%/*}/probe-status")" in
            0) printf '%s\n' 'loopback-netns: current-process-isolation=verified' >&2 ;;
            125) printf '%s\n' 'loopback-netns: current-process-isolation=outside-clean-launcher' >&2 ;;
            *) printf '%s\n' 'loopback-netns: partial-isolation-fixture' >&2 ;;
        esac
        exit "$(<"${0%/*}/probe-status")"
        ;;
    --)
        shift
        printf '%s\n' "$@" >"${DISPATCH_LOG:?}"
        ;;
    *) exit 97 ;;
esac
EOF
    /usr/bin/chmod 0700 -- "$wrapper"

    {
        printf '%s\n' '#!/usr/bin/bash -p' 'set -euo pipefail'
        printf '%s\n' \
            'readonly LOOPBACK="${FIXTURE_LOOPBACK:?}"' \
            'LOOPBACK_CONTENT=""' \
            'IFS= read -r -d "" LOOPBACK_CONTENT <"$LOOPBACK" || [[ -n "$LOOPBACK_CONTENT" ]]' \
            'readonly LOOPBACK_CONTENT' \
            'loopback_digest="$(printf "%s" "$LOOPBACK_CONTENT" | /usr/bin/sha256sum)"' \
            'readonly LOOPBACK_CONTENT_SHA256="${loopback_digest%% *}"' \
            'readonly VERIFIER_SELF=/fixture/verify-pins.sh' \
            'readonly VERIFIER_BOUND_TOKEN=__cubikan_verifier_bound_memory_v1__' \
            'readonly VERIFIER_CONTENT="printf verifier"' \
            'verifier_digest="$(printf "%s" "$VERIFIER_CONTENT" | /usr/bin/sha256sum)"' \
            'readonly VERIFIER_CONTENT_SHA256="${verifier_digest%% *}"' \
            'die() { printf "fixture: %s\n" "$*" >&2; exit 1; }' \
            'pin() {' \
            '    [[ "$1:$2" == host_tools:bash_path ]] || die "unexpected pin lookup"' \
            '    printf "%s\n" /usr/bin/bash' \
            '}'
        extract_function enter_or_verify_locked_isolation
        printf '%s\n' 'enter_or_verify_locked_isolation' '/usr/bin/touch -- "${RETURN_SENTINEL:?}"'
    } >"$harness"
    /usr/bin/chmod 0700 -- "$harness"

    printf '%s\n' 0 >"$fixture/probe-status"
    FIXTURE_LOOPBACK="$wrapper" \
        RETURN_SENTINEL="$returned" /usr/bin/bash -p "$harness" >"$fixture/proven.out" 2>"$fixture/proven.err"
    [[ -e "$returned" && ! -e "$dispatched" ]] || die 'proven isolation did not continue in place'

    /usr/bin/rm -f -- "$returned" "$dispatched"
    printf '%s\n' 125 >"$fixture/probe-status"
    FIXTURE_LOOPBACK="$wrapper" \
        RETURN_SENTINEL="$returned" /usr/bin/bash -p "$harness" >"$fixture/outside.out" 2>"$fixture/outside.err"
    [[ ! -e "$returned" && -e "$dispatched" ]] ||
        die 'clean outside isolation did not enter exactly one fresh wrapper'
    [[ "$(<"$dispatched")" == *$'__cubikan_verifier_bound_memory_v1__\n/fixture/verify-pins.sh\n__cubikan_prebound_verifier_memory_v1__'* &&
        "$(<"$dispatched")" == *$'\nprintf verifier\n--locked\n--offline' ]] ||
        die 'clean outside isolation dispatched unexpected verifier argv'

    /usr/bin/rm -f -- "$returned" "$dispatched"
    printf '%s\n' 1 >"$fixture/probe-status"
    if FIXTURE_LOOPBACK="$wrapper" \
        RETURN_SENTINEL="$returned" /usr/bin/bash -p "$harness" >"$fixture/partial.out" 2>"$fixture/partial.err"; then
        die 'partial isolation fixture unexpectedly succeeded'
    fi
    [[ ! -e "$returned" && ! -e "$dispatched" ]] || die 'partial isolation reached continuation or nested dispatch'
    /usr/bin/grep -Fq 'refusing nested namespace entry' "$fixture/partial.err" ||
        die 'partial isolation failure omitted its fail-closed reason'

    body="$(extract_function enter_or_verify_locked_isolation)"
    [[ "$body" == *'case "$isolation_status" in'* && "$body" == *'125)'* &&
        "$body" == *'partial or suspicious; refusing nested namespace entry'* ]] ||
        die 'locked isolation dispatcher no longer has the reviewed tri-state source shape'
}

test_plain_bash_compatibility_reenters_the_sanitized_verifier() {
    local subject marker output
    subject="$PROJECT_ROOT/$(pin_value repository_tools argv_grammar_path)"
    marker='identity-checked:argv_grammar:dependent-boundary-not-entered'
    if ! output="$(/usr/bin/bash "$VERIFIER" --test-identity argv_grammar "$subject" 2>&1)"; then
        printf '%s\n' "$output" >&2
        die 'canonical plain-bash verifier invocation did not re-enter privileged sanitized mode'
    fi
    [[ "$output" == *"$marker"* ]] || die 'plain-bash compatibility check omitted its dependent-boundary marker'
}

test_verifier_bind_helper_captures_reviewed_bytes_in_memory() {
    local fixture source alternate output harness expected source_identity source_size
    fixture="$TEST_ROOT/verifier-bind-snapshot"
    source="$fixture/reviewed.sh"
    alternate="$fixture/mutated.sh"
    output="$fixture/executed"
    harness="$fixture/harness.sh"
    /usr/bin/mkdir -p -- "$fixture"
    printf '%s\n' '#!/usr/bin/bash' 'printf "%s\n" original >>"${OUTPUT:?}"' >"$source"
    printf '%s\n' '#!/usr/bin/bash' 'printf "%s\n" mutated! >>"${OUTPUT:?}"' >"$alternate"
    [[ "$(/usr/bin/stat -c '%s' -- "$source")" == "$(/usr/bin/stat -c '%s' -- "$alternate")" ]] ||
        die 'verifier bind fixture scripts are not the same size'
    /usr/bin/chmod 0700 -- "$source" "$alternate"
    expected="$(/usr/bin/sha256sum -- "$source")"
    expected=${expected%% *}
    source_identity="$(/usr/bin/stat -c '%d:%i' -- "$source")"
    source_size="$(/usr/bin/stat -c '%s' -- "$source")"

    {
        printf '%s\n' '#!/usr/bin/bash -p' 'set -euo pipefail' 'shopt -u varredir_close'
        printf '%s\n' \
            'die() { printf "fixture: %s\n" "$*" >&2; exit 1; }'
        extract_function sha256_file
        extract_function require_hash
        extract_function bind_verified_repository_script
        printf '%s\n' \
            'bind_verified_repository_script "${SOURCE:?}" "${EXPECTED:?}" fixture-script BOUND_CONTENT BOUND_DIGEST' \
            '/usr/lib/cargo/bin/coreutils/dd if="${ALTERNATE:?}" of="$SOURCE" conv=notrunc status=none' \
            '[[ "$(/usr/bin/stat -c "%d:%i" -- "$SOURCE")" == "${SOURCE_IDENTITY:?}" ]]' \
            '[[ "$(/usr/bin/stat -c "%s" -- "$SOURCE")" == "${SOURCE_SIZE:?}" ]]' \
            '[[ "$(sha256_file "$SOURCE")" != "$EXPECTED" ]]' \
            '[[ "$BOUND_DIGEST" == "$EXPECTED" ]]' \
            'OUTPUT="${OUTPUT_FILE:?}" /usr/bin/bash -c "$BOUND_CONTENT"' \
            'OUTPUT="$OUTPUT_FILE" /usr/bin/bash -c "$BOUND_CONTENT"'
    } >"$harness"
    /usr/bin/chmod 0700 -- "$harness"

    SOURCE="$source" ALTERNATE="$alternate" EXPECTED="$expected" \
        SOURCE_IDENTITY="$source_identity" SOURCE_SIZE="$source_size" OUTPUT_FILE="$output" \
        /usr/bin/bash -p "$harness"
    [[ "$(<"$output")" == $'original\noriginal' ]] ||
        die 'verifier bind helper executed bytes from the overwritten workspace inode'
}

test_verifier_continuation_uses_reviewed_memory() {
    local fixture verifier_copy sentinel subject output content digest
    fixture="$TEST_ROOT/verifier-self-swap"
    verifier_copy="$fixture/chain/tools/verify-pins.sh"
    sentinel="$fixture/alternate-ran"
    subject="$fixture/chain/tools/node-argv-grammar-v1.txt"
    /usr/bin/mkdir -p -- "$fixture/chain/tools" "$fixture/chain/.cache/tmp"
    /usr/bin/cp -- "$VERIFIER" "$verifier_copy"
    /usr/bin/cp -- "$PINS" "$fixture/chain/pins.toml"
    /usr/bin/cp -- "$PROJECT_ROOT/chain/tools/node-argv-grammar-v1.txt" "$subject"
    /usr/bin/chmod 0700 -- "$verifier_copy"
    content=''
    IFS= read -r -d '' content <"$verifier_copy" || [[ -n "$content" ]]
    digest="$(printf '%s' "$content" | /usr/bin/sha256sum | /usr/bin/awk '{print $1}')"
    printf '#!/usr/bin/bash\n/usr/bin/touch -- %q\nexit 0\n' "$sentinel" >"$verifier_copy"
    /usr/bin/chmod 0700 -- "$verifier_copy"
    if ! output="$(/usr/lib/cargo/bin/coreutils/env -i CUBIKAN_VERIFIER_SANITIZED=1 \
        HOME=/home/charles CARGO_HOME="$fixture/chain/.cache/cargo-home" \
        RUSTUP_HOME=/home/charles/.rustup PATH=/home/charles/.cargo/bin:/usr/bin:/bin \
        LC_ALL=C LANG=C TZ=UTC TMPDIR="$fixture/chain/.cache/tmp" \
        GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null \
        GIT_NO_REPLACE_OBJECTS=1 GIT_OPTIONAL_LOCKS=0 \
        /usr/bin/bash --noprofile --norc -p -c "$content" "$verifier_copy" \
        __cubikan_verifier_bound_memory_v1__ "$verifier_copy" \
        "$digest" "$content" \
        --sanitized-entry --test-identity argv_grammar "$subject" 2>&1)"; then
        printf '%s\n' "$output" >&2
        die 'private verifier snapshot did not complete its identity-only continuation'
    fi
    [[ "$output" == *'identity-checked:argv_grammar:dependent-boundary-not-entered'* ]] ||
        die 'memory-bound verifier continuation omitted its dependent-boundary marker'
    [[ ! -e "$sentinel" ]] || die 'alternate verifier pathname bytes executed'
}

test_hostile_environment_is_removed_before_bootstrap() {
    local hostile_bin="$TEST_ROOT/hostile-bin" startup="$TEST_ROOT/hostile-bash-env.sh"
    local sentinel="$TEST_ROOT/hostile-bootstrap-executed" output command subject marker
    /usr/bin/mkdir -p -- "$hostile_bin"
    for command in dirname env awk sha256sum; do
        printf '#!/usr/bin/bash\n/usr/bin/touch -- %q\nexit 88\n' "$sentinel" >"$hostile_bin/$command"
        /usr/bin/chmod 0700 -- "$hostile_bin/$command"
    done
    printf '/usr/bin/touch -- %q\n' "$sentinel" >"$startup"
    subject="$PROJECT_ROOT/$(pin_value repository_tools argv_grammar_path)"
    marker='identity-checked:argv_grammar:dependent-boundary-not-entered'

    if ! output="$({
        PATH="$hostile_bin"
        BASH_ENV="$startup"
        ENV="$startup"
        CDPATH="$hostile_bin"
        dirname() { /usr/bin/touch -- "$sentinel"; return 88; }
        sha256sum() { /usr/bin/touch -- "$sentinel"; return 88; }
        export PATH BASH_ENV ENV CDPATH
        export -f dirname sha256sum
        "$VERIFIER" --test-identity argv_grammar "$subject"
    } 2>&1)"; then
        printf '%s\n' "$output" >&2
        die 'verifier bootstrap failed under a hostile inherited environment'
    fi
    [[ ! -e "$sentinel" ]] || die 'hostile PATH, BASH_ENV, or exported function executed during verifier bootstrap'
    [[ "$output" == *"$marker"* ]] || die 'sanitized hostile-environment check omitted its boundary marker'
}

test_fast_namespace_and_normalizer_guards() {
    /usr/bin/bash "$TOOL_DIR/loopback-netns.test.sh"
    /usr/bin/bash "$TOOL_DIR/normalize-node-argv.test.sh"
}

if [[ "${1:-}" == --test-isolation-dispatch ]]; then
    [[ $# -eq 1 ]] || die 'isolation-dispatch test accepts no additional arguments'
    test_locked_isolation_dispatch_is_tristate
    printf '%s\n' 'verify-pins tri-state isolation dispatch test passed'
    exit 0
fi
if [[ "${1:-}" == --test-script-snapshots ]]; then
    [[ $# -eq 1 ]] || die 'script-memory test accepts no additional arguments'
    test_verifier_bind_helper_captures_reviewed_bytes_in_memory
    test_verifier_continuation_uses_reviewed_memory
    printf '%s\n' 'verify-pins reviewed script memory tests passed'
    exit 0
fi
[[ $# -eq 0 ]] || die 'usage: verify-pins.test.sh [--test-isolation-dispatch | --test-script-snapshots]'

initialize_identity_cases
test_chain_dependency_toolchain_metadata_and_artifact_pins_are_exact
test_nested_chain_workspace_has_only_allowlisted_root_delta
test_pin_verifier_rejects_every_identity_mismatch
test_canonical_registry_archive_cache_identities_precede_source_materialization
test_sdk_cargo_git_database_matches_verified_release_archive_before_source_materialization
test_materialized_sdk_checkout_matches_verified_release_archive_before_build
test_locked_isolation_dispatch_is_tristate
test_plain_bash_compatibility_reenters_the_sanitized_verifier
test_verifier_bind_helper_captures_reviewed_bytes_in_memory
test_verifier_continuation_uses_reviewed_memory
test_hostile_environment_is_removed_before_bootstrap
test_fast_namespace_and_normalizer_guards
printf '%s\n' 'verify-pins exact-identity, identity-mutation, bootstrap, and fail-before-execution tests passed'
