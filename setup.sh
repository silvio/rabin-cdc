#!/bin/sh

[ ! -f /tmp/test.data ] && dd if=/dev/urandom of=/tmp/test.data  count=50 bs=1M
