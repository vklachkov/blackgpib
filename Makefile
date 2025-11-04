.PHONY: all build deploy

all: build deploy

build:
    cargo build --release

deploy:
    rsync -avzP -e ssh target/aarch64-unknown-linux-gnu/release/blackgpib blackgpib@blackgpib.local:/home/blackgpib/blackgpib
