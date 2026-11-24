#!/bin/sh
printf '%s\n' "$@" > arguments.new && mv arguments.new arguments
