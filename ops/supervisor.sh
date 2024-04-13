#!/usr/bin/env bash

if [ ! -d ./run ]; then
  mkdir ./run
fi
supervisord -c ops/supervisor.conf
