#!/bin/env bash

if [ "$#" -gt 1 ]; then
  echo "At most one argument allowed" >&2
  exit 2
fi

HERE=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
TEST_DIR=${1:-"."}

find "${TEST_DIR}" -iname "*.tst" -exec bash "${HERE}/../tools/HardwareSimulator.sh" {} \;
