# LXI Web Interface

Based on the very easy to understand [Tide Example Application](https://github.com/jbr/tide-example).

## Running this repository locally

You need to add two environment variables to run this application,
`DATABASE_URL` and `TIDE_SECRET`.

### Database setup

This application is built on the [sqlite
database](https://www.sqlite.org/), and accepts database urls of the
form `"sqlite:///path/to/sqlite.db"`.

This repository does yet not use migrations. In order to set up the database, run
```bash
$ sqlite3 db/sqlite3.db < db/schema.sql
```

You will also need to add `DATABASE_URL` to your environment. We
recommend using [`direnv`](https://direnv.net/), and include an
example .envrc.

``` bash
export DATABASE_URL="sqlite://./db/sqlite3.db"
```

### Sessions
The `TIDE_SECRET` needs to be a cryptographically random key of at
least 32 bytes in your execution environment. An easy way to generate
it is:

``` bash
$ openssl rand -base64 64
```

### Running the app

``` bash
cargo run
```