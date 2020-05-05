# Cosmic Cantina

A game written for [Sim Jam](https://itch.io/jam/dogpit-sim-jam).

[Design Wiki](https://www.notion.so/khonsulabs/SimJam-Cosmic-Cantina-ee2cb91e444f425ca24399142970dc8f)

## Repository Layout

- `client/`: The game client project
- `server/`: The game client server
- `shared/`: Shared code that both the server and client utilize

# Server Information

## Requirements

- PostgreSQL 11+
- Redis (unsure of exact required version)

## Setup

- Create a database in PostgreSQL, and set up a .env file containing these settings:
  - `DATABASE_URL`: The connection string, a la `postgres://cosmiccantina:password@localhost/cosmiccantina`
  - `OAUTH_CLIENT_ID`: The client ID from itch.io's OAuth application setup
  - `OAUTH_CLIENT_SECRET`: The client secret from itch.io's OAuth application setup
- Run the migrations: `cargo run --pakage migrations`
- Run the server: `cargo run --package server`

# Client Information

## Requirements

The goal is for this game to be runnable on Windows 10 and Mac OS X without needing to install any extra frameworks.

## Setup

- Run the client: `cargo run --package client`
