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

- Run the server: `cargo run --package server`

# Client Information

## Requirements

The goal is for this game to be runnable on Windows 10 and Mac OS X without needing to install any extra frameworks.

## Setup

- Run the client: `cargo run --package client`
