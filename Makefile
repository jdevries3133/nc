SHELL := /bin/bash
ENV=source .env &&
DB_CONTAINER_NAME := "nc_db"

# The registry is presumed to be docker.io, which is the implicit default
DOCKER_ACCOUNT=jdevries3133
CONTAINER_NAME=nc
TAG?=$(shell git describe --tags)
CONTAINER_QUALNAME=$(DOCKER_ACCOUNT)/$(CONTAINER_NAME)
CONTAINER_EXACT_REF=$(DOCKER_ACCOUNT)/$(CONTAINER_NAME):$(TAG)

.PHONY: build
.PHONY: check
.PHONY: setup
.PHONY: dev
.PHONY: db
.PHONY: start-db
.PHONY: stop-db
.PHONY: reset-db
.PHONY: watch-db
.PHONY: shell-db
.PHONY: build-container
.PHONY: debug-container
.PHONY: push-container

check: setup
ifdef CI
	pnpm run build
endif
	cargo clippy -- -D warnings
	cargo fmt --check
	cargo test

build: setup
	pnpm run build
	cargo build --release

setup:
	[[ ! -f ./src/htmx-1.9.6.vendor.js ]] \
		&& curl -L https://unpkg.com/htmx.org@1.9.4 > src/htmx-1.9.6.vendor.js \
		|| true
ifdef CI
	npm i -g pnpm
endif
	[[ ! -d node_modules ]] \
		&& pnpm install \
		|| true
ifndef CI
	@# we only want the `.env` file locally in practice. We never run the app
	@# in CI (yet). The problem with having the `.env` file in CI is that
	@# sqlx will pickup on the `DATABASE_URL` environment variable and try
	@# to talk to a datbase that isn't there, causing compilation to fail.
	@# See also https://github.com/launchbadge/sqlx/blob/540baf7df55a372cb79d8636d02b1361a495b344/sqlx-cli/README.md#force-building-in-offline-mode
	[[ ! -f .env ]] && cp env-template .env || true
endif

dev: setup
	npx concurrently --names 'tailwind,cargo' \
		'pnpm run dev' \
		"cargo watch -x 'run --features live_reload'"

db: reset-db watch-db

start-db:
	$(ENV) docker run \
        --name $(DB_CONTAINER_NAME) \
        -e POSTGRES_DB="$$POSTGRES_DB" \
        -e POSTGRES_USER="$$POSTGRES_USERNAME" \
        -e POSTGRES_PASSWORD="$$POSTGRES_PASSWORD" \
        -p 5432:5432 \
        -d \
        postgres:15

stop-db:
	docker kill $(DB_CONTAINER_NAME) || true
	docker rm $(DB_CONTAINER_NAME) || true

reset-db: stop-db
	make start-db

watch-db:
	docker logs -f $(DB_CONTAINER_NAME)

shell-db:
	$(ENV) PGPASSWORD=$$POSTGRES_PASSWORD \
		psql -U "$$POSTGRES_USERNAME" -h 0.0.0.0 $$POSTGRES_DB

build-container: setup
	pnpm run build
	rustup target add x86_64-unknown-linux-musl
	cargo build --release --target x86_64-unknown-linux-musl
	docker buildx build --load --platform linux/amd64 -t $(CONTAINER_EXACT_REF) .

# Run the above container locally, such that it can talk to the local
# PostgreSQL database launched by `make start-db`. We expect here that the
# local database is already running and the container has already been built.
debug-container:
	$(ENV) docker run \
		-e RUST_BACKTRACE=1 \
		-e POSTGRES_USERNAME="$$POSTGRES_USERNAME" \
		-e POSTGRES_PASSWORD="$$POSTGRES_PASSWORD" \
		-e POSTGRES_DB="$$POSTGRES_DB" \
		-e POSTGRES_HOST="host.docker.internal" \
		-e SESSION_SECRET="$$SESSION_SECRET" \
		-p 8000:8000 \
		$(CONTAINER_EXACT_REF)

push-container: build-container
	docker push $(CONTAINER_EXACT_REF)
