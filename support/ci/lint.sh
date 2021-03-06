#!/bin/bash

# Fail if there are any unset variables and whenever a command returns a
# non-zero exit code.
set -eu

# If the variable `$DEBUG` is set, then print the shell commands as we execute.
if [ -n "${DEBUG:-}" ]; then
  set -x
  export DEBUG
fi

info() {
  case "${TERM:-}" in
    *term | xterm-* | rxvt | screen | screen-*)
      printf -- "   \033[1;32m${program}: \033[1;37m$1\033[0m\n"
      ;;
    *)
      printf -- "   ${program}: $1\n"
      ;;
  esac
  return 0
}

warn() {
  case "${TERM:-}" in
    *term | xterm-* | rxvt | screen | screen-*)
      >&2 echo -e "   \033[1;32m${program}: \033[1;33mWARN \033[1;37m$1\033[0m"
      ;;
    *)
      >&2 echo "   ${pkg_name}: WARN $1"
      ;;
  esac
  return 0
}

exit_with() {
  case "${TERM:-}" in
    *term | xterm-* | rxvt | screen | screen-*)
      >&2 printf -- "\033[1;31mERROR: \033[1;37m$1\033[0m\n"
      ;;
    *)
      >&2 printf -- "ERROR: $1\n"
      ;;
  esac
  exit $2
}

program=$(basename $0)
rf_version="0.3.8"

# Fix commit range in Travis, if set.
# See: https://github.com/travis-ci/travis-ci/issues/4596
if [ -n "${TRAVIS_COMMIT_RANGE:-}" ]; then
  TRAVIS_COMMIT_RANGE="${TRAVIS_COMMIT_RANGE/.../..}"
fi

# Since we removed the generated protocol files from the repo, we need to
# ensure they're present before linting, otherwise rustfmt will die.
this_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ ! -f "$this_dir/../../components/builder-protocol/src/message/jobsrv.rs" ]; then
  echo "There are no generated protocol files in builder-protocol/src/message. Regenerating those now."
  pushd "$this_dir/../../components/builder-protocol"
  cargo clean
  cargo build
  popd
fi

info "Checking for rustfmt"
if ! command -v rustfmt >/dev/null; then
  exit_with "Program \`rustfmt' not found on PATH, aborting" 1
fi

info "Checking for version $rf_version of rustfmt"
actual="$(rustfmt --version | cut -d ' ' -f 1)"
if [[ "$actual" != "$rf_version-nightly" ]]; then
  echo "rustfmt is located at $(which rustfmt)"
  exit_with "\`rustfmt' version $actual doesn't match expected: $rf_version" 2
fi

failed="$(mktemp -t "$(basename $0)-failed-XXXX")"
trap 'code=$?; rm -f $failed; exit $code' INT TERM EXIT

if [[ -n "${LINT_ALL:-}" ]]; then
  cmd="find components -type f -name '*.rs'"
  info "Linting all files, selecting files via: '$cmd'"
elif [[ $(git diff --name-only | wc -l) -gt 0 ]]; then
  cmd="git diff --name-only"
  info "Unstaged changes detected, selecting files via: '$cmd'"
elif [[ $(git diff --name-only --cached | wc -l) -gt 0 ]]; then
  cmd="git diff --name-only --cached"
  info "Staged changes detected, selecting files via: '$cmd'"
else
  treeish="${1:-${TRAVIS_COMMIT_RANGE:-${TRAVIS_COMMIT:-HEAD}}}"
  cmd="git diff-tree --no-commit-id --name-only -r $treeish"
  info "Selecting files from Git via: '$cmd'"
fi

eval "$cmd" | while read file; do
  case "${file##*.}" in
    rs)
      if [ ! -e "$file" ]; then
        # skip files which were deleted
        break
      fi
      if echo "$file" | grep -q "components/builder-protocol/src/message" >/dev/null; then
        info "Skipping generated Rust code file $file"
        break
      fi
      info "Running rustfmt on $file"
      set +e
      output="$(rustfmt --skip-children --write-mode diff "$file" 2>&1)"
      rf_exit="$?"
      set -e
      case $rf_exit in
        0|3)
          if echo "$output" | grep -q "Diff at line " >/dev/null; then
            warn "File $file generates a diff after running rustfmt $rf_version"
            warn "Perhaps you forgot to run \`rustfmt' or \`cargo fmt'?"
            warn "Diff for $file:"
            echo "$output"
            echo "$file" >> "$failed"
          fi
          ;;
        101)
          warn "File $file exited with $rf_exit"
          warn "Error output:"
          echo "$output"
          warn "Skipping this failure until next release of rustfmt"
          ;;
        *)
          warn "File $file exited with $rf_exit"
          warn "Error output:"
          echo "$output"
          echo "$file" >> "$failed"
          ;;
      esac
      ;;
  esac
done

if [[ $(cat "$failed" | wc -l) -gt 0 ]]; then
  echo
  echo
  warn "Summary: One or more files failed linting:"
  cat "$failed" | while read file; do
    warn "  * $file"
  done
  exit_with "File(s) failed linting" 10
else
  info "Summary: All checked files passed their lints."
fi
