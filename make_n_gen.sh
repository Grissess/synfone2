#!/bin/sh

GEN="${2:-gens/test_basic.gen}"

echo "["
for i in $(seq $1); do
	printf "\t#${GEN}#"
	if [ "$i" -eq "$1" ]; then
		echo
	else
		echo ","
	fi
done
echo "]"
