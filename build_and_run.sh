#!/bin/bash

time cargo build

p="./target/debug/"
if [ $? -eq 0 ]; then
    time ${p}capp_log_parser -t stringlist -l -c "$PWD/test/config.json" "$PWD/test/access-83.log"
    # time ${p}capp_log_parser -t json --page-index 1 -c "$PWD/test/config.json" "$PWD/test/access-83.log"
    # time ${p}capp_log_parser -t json -l -c "$PWD/test/config.json" "$PWD/test/access-83.log"
else
    echo ""
    echo "------------ BUILD FAIL ------------"
    echo ""
fi