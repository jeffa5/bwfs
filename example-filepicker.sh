#!/usr/bin/env bash

set -ex

bwfs="cargo run -- "

root=${1:?no filesystem root given}

if ! $bwfs status >/dev/null 2>&1; then
  $bwfs unlock
fi

echo "Choosing files from $root"

file=$(cd $root && find . -type f | fzf)

if [[ -f "$root/$file" ]]; then
  echo "Selected $file"
  cat "$root/$file" | wl-copy
else
  exit 1
fi
