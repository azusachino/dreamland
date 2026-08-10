# ADR 0006: Roadmap and GitHub Issues

- Status: Proposed
- Date: 2026-08-10

## Context

Dreamland has a repository roadmap and an API design gate. GitHub Issues are
useful for execution, but turning every roadmap bullet into an issue would
create noise and invite feature work before the contract is approved.

## Proposal

Keep `docs/ROADMAP.md` as the canonical product direction. Use GitHub Issues
only for approved, actionable work. Each issue should link to the relevant
roadmap item and ADR or API section, and should have a concrete acceptance
criterion.

Create issues for milestones and vertical slices, not every implementation
subtask. Do not create search, favorites, tags, or batch-download issues until
API v1 and their product semantics are approved.

## Consequences

- GitHub is useful for execution without becoming a second decision store.
- The issue list stays small enough to represent current priorities.
- Creating issues is a remote action and requires explicit approval before
  it is performed.
