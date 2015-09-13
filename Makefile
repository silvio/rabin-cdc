
all: build run

build:
	cargo rustc -- -O -g

release:
	cargo rustc --release -- -g

bench:
	$(shell ./setup.sh)
	cargo bench

run:
	echo "das ist ein Test" | ./target/debug/rabin-cdc
	echo "end"
