.RECIPEPREFIX := >
.PHONY: build-book build-cli build serve-book

build-book:
> mdbook build handbook/

build-cli:
> cargo build --release --package stylus-port-cli

build: build-book build-cli

serve-book:
> mdbook serve handbook/
