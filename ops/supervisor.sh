#!/usr/bin/env bash

SVC_PATH="./target/release/request-receiver"
WRK_PATH="./target/release/downstreamer"

if [ ! -d ./run ]; then
  mkdir ./run
fi

if [ ! -e $SVC_PATH ] || [ ! -e $WRK_PATH ]; then
  cargo build --release
fi
supervisord -c ops/supervisor.conf
