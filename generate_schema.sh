#!/usr/bin/env bash
# This script will generate the src/schema.rs file based on the tables found in the database
# The DATABASE_URL environment variable should be set (or use a .env file)
# `example_models.rs` will also be created which contains an example struct for each table

# If on WSL use the Windows versions
if cat /proc/version | grep microsoft; then
  CARGO=cargo.exe
  DIESEL=diesel.exe
  DIESEL_EXT=diesel_ext.exe
else
  CARGO=cargo
  DIESEL=diesel
  DIESEL_EXT=diesel_ext
fi

if [[ "$($DIESEL migration pending)" != "false" ]]; then
  echo "There are pending diesel migrations"
  exit 1
fi

if ! command -v $DIESEL_EXT &>/dev/null; then
  echo 'Installing diesel_cli_ext...'
  $CARGO install diesel_cli_ext
fi

$DIESEL print-schema > src/schema.rs
$DIESEL_EXT > example_models.rs
