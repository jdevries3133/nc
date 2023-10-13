SHELL := /bin/bash
ENV=source .env &&
DB_CONTAINER_NAME := "nc_db"

.PHONY: build
.PHONY: setup
.PHONY: dev
.PHONY: db
.PHONY: start-db
.PHONY: stop-db
.PHONY: reset-db
.PHONY: watch-db
.PHONY: shell-db

build: setup
	pnpm run build
	cargo build --release

setup:
	[[ ! -f ./src/htmx-1.9.6.vendor.js ]] \
		&& curl -L https://unpkg.com/htmx.org@1.9.4 > src/htmx-1.9.6.vendor.js \
		|| true
	[[ ! -d node_modules ]] \
		&& pnpm install \
		|| true
	[[ ! -f .env ]] && cp env-template .env || true

dev: setup
	npx concurrently --names 'tailwind,cargo' \
		'pnpm run dev' \
		"cargo watch -x 'run --features live_reload'"

# 99% of the time, this is what you want when you change `initdb.sql`, because
# you want to re-init the DB with that change, and also watch to make sure
# your change doesn't have a bug.
db: reset-db watch-db

start-db:
	$(ENV) docker run \
        --name $(DB_CONTAINER_NAME) \
        -e POSTGRES_DATABASE="$$POSTGRES_DB" \
        -e POSTGRES_USER="$$POSTGRES_USER" \
        -e POSTGRES_PASSWORD="$$POSTGRES_PASSWORD" \
        -v $(PWD)/initdb.sql:/docker-entrypoint-initdb.d/initdb.sql \
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
		psql -U "$$POSTGRES_USER" -h 0.0.0.0 $$POSTGRES_DB
