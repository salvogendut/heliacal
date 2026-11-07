CARGO ?= cargo

.PHONY: all run check smoke
all:
	$(CARGO) build --release --locked
run:
	$(CARGO) run --release --locked -- $(ARGS)
check:
	$(CARGO) fmt -p heliacal --check
	$(CARGO) clippy --locked --all-targets --no-deps -- -D warnings
	$(CARGO) test --locked
smoke:
	SDL_VIDEO_DRIVER=dummy SDL_RENDER_DRIVER=software $(CARGO) run --locked -- --smoke-test
