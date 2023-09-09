The Technomancy Server
======================

This is the main repository of the technomancy project, it will encompass the
following parts:

- A server that serves as the main coordinator of games between the frontend
  and the engine, simply called the 'server'
- A server that serves as the backend engine, where the game rules are loaded
  in, simply called the 'engine'

## Server

The server is naturally split up between two parts: frontend and backend.

### Frontend

The frontend (aka, the visible parts of clients) is expected to be written in
HTML and Javascript and/or WASM.

Explicit targets:

- Latest Firefox
- Latest Chrome
- Latest Safari/iOS
- Latest Android

Potentially other web targets


### Backend

The backend is written in Rust and serves HTML as well as provide connectivity
through for example websockets.

The server _can only communicate to the engine via RPC_. This is a **hard
requirement**.

## Engine

The engine is what runs and verifies the rules of actions done in games powered
by it. It communicates with the server via a duplex RPC connection.

## Contributing

The contribution process is currently being defined. This includes the code of
conduct as well as some examples of how new features might be added.
