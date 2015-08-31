
all: build run

build:
	cargo build

run:
	echo "das ist ein Test" | ./target/debug/rabin-cdc
	echo "end"
