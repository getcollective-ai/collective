# Code

All code for the project.

| Folder     | Type | Description                                                                           |
|------------|------|---------------------------------------------------------------------------------------|
| `executor` | bin  | The executor, which runs in the `docker` container.                                   |
| `launcher` | bin  | The launcher, which runs on the host and starts the `docker` container and `executor` |
| `openai`   | lib  | library to interact with OpenAI's API                                                 |
| `utils`    | lib  | utility functions                                                                     |