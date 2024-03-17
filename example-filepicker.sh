#!/usr/bin/env bash

root=${1:?no filesystem root given}

# TODO: probably want to check the status of the filesystem and ask the user to unlock it if needed

echo "Choosing files from $root"

file=$(cd $root && find . -type f | fzf)

if [[ -f "$root/$file" ]]; then
  echo "Selected $file"
  cat "$root/$file" | wl-copy
else
  exit 1
fi
