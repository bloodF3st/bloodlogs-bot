BIN = bloodlogs-bot
TARGET = x86_64-unknown-linux-musl

.PHONY: build release deploy clean

build:
	cargo build

release:
	cargo build --release --target $(TARGET)

deploy: release
	scp target/$(TARGET)/release/$(BIN) server:~/$(BIN)/$(BIN)
	ssh server "systemctl restart $(BIN)"

clean:
	cargo clean
