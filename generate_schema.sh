#!/usr/bin/env bash

echo "NOTE: Ensure valid database state with \`diesel migration run\`"

if ! command -v cargo &>/dev/null && command -v cargo.exe &>/dev/null; then
  CARGO=cargo.exe
  DIESEL=diesel.exe
  DIESEL_EXT=diesel_ext.exe
else
  CARGO=cargo
  DIESEL=diesel
  DIESEL_EXT=diesel_ext
fi

$CARGO install diesel_cli_ext
$DIESEL print-schema > src/schema.rs
$DIESEL_EXT > src/models_new.rs
