
all: build run

build:
	cargo build

bench:
	cargo bench

run:
	echo "das ist ein Test" | ./target/debug/rabin-cdc
	echo "end"
