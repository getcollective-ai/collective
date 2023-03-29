# `collective`

[![codecov](https://codecov.io/github/getcollective-ai/collective/branch/main/graph/badge.svg?token=C7HBZAAX3B)](https://app.codecov.io/gh/getcollective-ai/collective)

An AI developer that evolves to fit your needs.

---
`collective` is an AI developer that adapts to your coding style.
When prompted to create a project, it engages through prompts and questions, improving
its understanding of
your preferences. Experience a collaborative, evolving coding partner that continuously refines its abilities to support
your programming tasks.

# Speculative Tech Stack

| Component                     | Technology                   |
 |-------------------------------|------------------------------|
| Base OS                       | macOS (at least)             |
| Interface                     | NestJS (or maybe native app) |
| Isolation                     | docker                       |
| Isolation Env (OS)            | ubuntu                       |
| Isolation Env (Shell)         | zsh                          |
| Data (and preference) storage | mongodb                      |
| Core library, executor        | rust                         |
| LLM                           | GPT4                         |

# MVP

- Learn preferences through Q&A while making a project and
  adapt them to make a separate project (as defined by the user) with minimal input from the user

## Requirements

### Internals

- text-based search engine
    - capable of searching `GitHub`, `crates.io`, `docs.rs`, and `rust-lang.org`
        - only look at `GitHub` repos with the correct license

## Evaluation

- style transfer between projects is apparent
- the assistant is able to make the second project with near minimal assistance
