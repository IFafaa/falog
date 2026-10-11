# Task specs

One Markdown file per piece of work, so the next person (or AI assistant) can pick it up without the
conversation that produced it.

```
tasks/
  _template.md     copy this
  backlog/         specced, not started
  doing/           in progress (keep this to one or two)
  done/            shipped; kept as a record of decisions
  shelved/         decided against for now; kept with the reason
```

## Naming

`NNNN-short-slug.md`, numbered in creation order (`0006-tags.md`). Numbers are never reused.

## Lifecycle

1. **Backlog**: copy `_template.md`, fill *Goal*, *Context* and *Acceptance criteria*; add it to
   [roadmap.md](../roadmap.md).
2. **Doing**: `git mv` the file to `doing/`, write the *Plan* (files, steps, risks) before coding.
3. **Done**: tick the criteria, fill *Outcome* (what changed, decisions, follow-ups), move to `done/`,
   and update the area specs it affected.
4. **Shelved**: when an idea is dropped, write why under *Outcome*, move it to `shelved/` and list it
   under *Shelved* in the roadmap instead of the table.
