# Getting Started

To run the project locally, you need the following tools:

- [docker CLI](https://docs.docker.com/engine/reference/commandline/cli/)
- [cargo](https://rustup.rs/)
- [pnpm](https://pnpm.io/)
- [concurrently](https://www.npmjs.com/package/concurrently)
- [cURL](https://curl.se/)
- [Make](https://formulae.brew.sh/formula/make)

The following ports also must be free on your machine:

- `5432` for PostgreSQL
- `8000` for this application

With those requirements met, running the project locally should be as simple as
using the Makefile:

```
make dev
```

There are very few unit tests, but you can run them with:

```
cargo test
```

There are some utilities in the Makefile for working with the database. In
particular:

```
make db        # reset the DB, and then live-tail the logs until you ctrl-C
make shell-db  # attach to an interactive PostgreSQL shell inside the DB
```

# Other Database Options

Of course, the application will happily converse with any PostgreSQL instance.
You can easily direct the program to your PostgreSQL instance of your choosing
by simply changing the `.env` file. Note that the `.env` file is created by
copying `env-template` the first time you run `make dev`. Naturally, it contains
other handy config levers.

# Auth & Getting Around

Navigation and flows between routes has generally not yet joined the chat, so
you need to know where to go if you're running the app locally:

- `/authentication/register` to make an account
- `/authentication/login` to log in if you already did so
- `/collection/1` to view the one and only default collection, though you'll be
  redirected to `/authentication/login` if you've not authenticated yet.
- to logout, if you wish, delete your cookies!

Once you get to `/collection/1`, there's a more complete navigation experience
between the 3 views there:

- main view (`/collection/1`)
- add page (`/collection/1/new-page`)
- reorder columns (`/collection/1/prop-order`)
