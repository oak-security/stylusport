.RECIPEPREFIX := >
.PHONY: build-book build-cli build serve-book cli-gifs

build-book:
> mdbook build handbook/

build-cli:
> cargo build --release --package stylus-port-cli

build: build-book build-cli

serve-book:
> mdbook serve handbook/

TAPE_FILES := $(wildcard cli/tapes/*.tape)
GIF_FILES := $(TAPE_FILES:.tape=.gif)

# Pattern rule to convert .tape to .gif (becomes stale if cli source files change)
cli/tapes/%.gif: cli/tapes/%.tape $(wildcard cli/src/**/*.rs)
> vhs $< --output $@

cli-gifs: $(GIF_FILES)
