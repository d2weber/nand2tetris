#!/bin/env bash
find . -iname "*.tst" -exec bash ../../tools/HardwareSimulator.sh {} \;
