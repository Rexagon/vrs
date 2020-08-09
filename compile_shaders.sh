#!/bin/bash -eE

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)

SHADERS_DIR="${SCRIPT_DIR}/shaders"
SPV_DIR="${SCRIPT_DIR}/shaders/spv"

mkdir -p "${SPV_DIR}"

find "${SHADERS_DIR}" -maxdepth 1 -type f -printf "%f\0" | while IFS= read -r -d $'\0' file;
do
  echo "Compiling ${file}..."
  glslc "${SHADERS_DIR}/${file}" -o "${SPV_DIR}/${file}.spv"
  echo ""
done
