# Product Vision

## Definition

Kineto is a local-first AI filmmaking workspace that converts source material into reusable, production-ready media assets through a human-directed agent workflow.

It is not a one-click "AI movie generator".

It is closer to an IDE and production operating system for generative filmmaking.

## Target workflow

```text
Source material
    ↓
Story analysis
    ↓
Screenplay
    ↓
User review / edit / lock
    ↓
World bible + character bible
    ↓
Visual style candidates
    ↓
Character candidates
    ↓
User lock
    ↓
Voice casting
    ↓
Scene breakdown
    ↓
Shot planning
    ↓
Keyframe candidates
    ↓
User selection
    ↓
Video candidate
    ↓
Refine / accept
    ↓
Preference update
    ↓
Next shot
```

## Core user promise

The user owns:

- the source
- the prompts
- the API keys
- the model choice
- the project state
- the generated files
- the final production assets

Kineto owns only the orchestration logic and UX.

## Differentiation

The product should optimize for capabilities that cloud-first AI filmmaking tools commonly under-emphasize:

- bring-your-own provider credentials
- local LLM and local GPU support
- open project files
- deterministic production state
- provider portability
- human approval at every important artifact boundary
- reusable character/location/prop memory
- prompt compilation per provider
- continuity tracking across shots
- export without forcing editing inside the app

## Non-goals

Kineto should not become:

- a full timeline editor
- a DAW
- a color grading system
- a compositor
- a subtitle editor
- a proprietary cloud storage service
- a credits marketplace
- a single-model wrapper

## Product language

Preferred positioning:

> A local-first, model-agnostic AI filmmaking workspace that turns source material into production-ready shots through a human-directed agent workflow.

Internal shorthand:

> GitHub + IDE for AI filmmaking.

Product principle:

> You own the story, models, and files. Kineto orchestrates production.
