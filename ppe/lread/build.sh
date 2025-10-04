#!/bin/bash
for ver in 300 310 320 330 340 400
do
	../../target/debug/pplc --runtime $ver
done

