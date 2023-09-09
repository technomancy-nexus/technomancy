The Technomancy Server
======================

This is the main repository of the technomancy project, it will encompass the
following parts:

- A server that serves as the main coordinator of games between the frontend
  and the engine, simply called the 'server'
- A server that serves as the backend engine, where the game rules are loaded
  in, simply called the 'engine'

## Frontend

The frontend (aka, the visible parts of clients) is expected to be written in
HTML and Javascript and/or WASM.

Explicit targets:

- Latest Firefox
- Latest Chrome
- Latest Safari/iOS
- Latest Android

Potentially other web targets


## Backend

The backend is written in Rust and serves HTML as well as provide connectivity
through for example websockets.

The server _can only communicate to the engine via RPC_. This is a **hard
requirement**.

## Engine

The engine will encompass the technomancy engine, it is being developed at:
https://github.com/technomancy-nexus/engine.


## Contributing

This part of the project is intended to stay private for the time being. This
means we cannot use anything licensed that is under the AGPL or similar
licenses. _Notably, this includes the technomancy engine._ See above on how
this is solved.
