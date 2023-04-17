# Code

All code for the project.

| Folder         | Type | Description                                                                           |
|----------------|------|---------------------------------------------------------------------------------------|
| `executor`     | bin  | The executor, which runs in the `docker` container and is the brain of `collective`   |
| `frontend-cli` | bin  | A vim-like frontend implementation to communicate with the `executor`                 |
| `html-to-md`   | lib  | A library to turn HTML into markdown. Useful as a utility to browse the internet      |
| `launcher`     | bin  | The launcher, which runs on the host and starts the `docker` container and `executor` |
| `protocol`     | lib  | The definition of the frontend <-> executor protocol. Allows for multiple frontends   |
| `utils`        | lib  | utility functions used across libraries                                               |