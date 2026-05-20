---
id: rfc-00025-vector-site-frontend
type: rfc
code: "00025"
slug: vector-site-frontend
title: Vector Landing Site and Product Documentation
description: Proposes creating two Astro sites for Vector: a landing website and a separate product documentation site, without introducing a monorepo for now.
status: accepted
created: 2026-05-18
updated: 2026-05-18
authors:
  - Codex
tags:
  - website
  - astro
  - landing
  - documentation
related:
  - spec-00006-typescript-quality-gate-contract-for-vs-code-extensions
supersedes: []
superseded_by: null
aliases:
  - "RFC 00025: Vector Landing Site and Product Documentation"
---

# RFC 00025: Vector Landing Site and Product Documentation

```vector-agent-action
label: Create a task
profile: create-doc
prompt: prompts-00005-create-document
input:
  message: Create a task to implement the `rfc-00025-vector-site-frontend`
  document-type: task
  document-name: implement-rfc-00025-vector-site-frontend
```

## 1. Problem

Vector currently lacks a dedicated public website that explains the product clearly, presents its main components, and creates a coherent entry point for users evaluating the project. Information about the MCP server and the VS Code extension is fragmented and not packaged as a product narrative.

The current presentation needs also mix two distinct goals that should evolve independently: a landing experience for product positioning and a documentation experience for product usage. Treating both as a single site would create avoidable content and layout tension from the start.

## 2. Proposal

Create two separate sites using Astro:

- `frontend/website` for the public landing site.
- `frontend/docs` for the product documentation site.

The landing site should:

- Explain what Vector is and what problem it solves.
- Present the main product components, with explicit sections for the MCP server and the VS Code extension.
- Focus on product positioning, navigation, adoption, and entry points into the docs.

The documentation site should:

- Explain installation, configuration, workflows, and reference material.
- Be optimized for product documentation rather than marketing presentation.
- Remain structurally separate from the landing site so each site can evolve with its own information architecture.

Both package bootstraps must adopt the quality gate baseline defined in [[spec-00006-typescript-quality-gate-contract-for-vs-code-extensions]] as part of their initial setup. At minimum, the `frontend/website` and `frontend/docs` packages must be bootstrapped with explicit package-level quality scripts and enforcement points rather than treating the Astro scaffold as sufficient by default.

After this RFC is accepted, the repository should contain:

- A bootstrapped Astro site in `frontend/website`.
- A bootstrapped Astro site in `frontend/docs`.
- Bootstrap configuration for both packages aligned with [[spec-00006-typescript-quality-gate-contract-for-vs-code-extensions]].
- A clear integration path between both sites through links and consistent navigation concepts.
- No monorepo orchestration layer introduced yet beyond what is strictly required to host both sites in the repository.

## 3. Alternatives Considered

- **Build a single site for both landing and docs:** Discarded because marketing and documentation have different content models, navigation needs, and evolution paths.
- **Introduce a monorepo with Turbo now:** Discarded because it adds structural complexity before the repository has validated the operational need for frontend workspace orchestration.
- **Create the sites as standalone repositories:** Discarded because it would reduce short-term coupling but fragment governance and coordinated product delivery.

## 4. Tradeoffs

| Pro                                                                                                  | Con                                                                                                                                                                         |
|------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Separate Astro sites let landing and docs evolve without forcing one navigation model to serve both. | Two sites introduce duplicate setup, dependency management, and deployment concerns.                                                                                        |
| Avoiding monorepo tooling keeps the initial implementation simpler and easier to reason about.       | Shared frontend concerns may be duplicated until a stronger packaging strategy is introduced later.                                                                         |
| Using Astro for both sites keeps the stack consistent.                                               | Consistency at the framework level does not by itself solve content governance or design-system reuse.                                                                      |
| Reusing the existing TypeScript quality gate contract raises the baseline from the first commit.     | The referenced contract targets VS Code extension packages, so some clauses may not map cleanly to Astro sites without interpretation or a follow-up web-specific contract. |
| Co-locating both sites in the main repository keeps delivery aligned with the product.               | The repository still absorbs more frontend surface area and operational ownership.                                                                                          |

## 5. Acceptance Criteria

- [ ] A new Astro project exists at `frontend/website`.
- [ ] A new Astro project exists at `frontend/docs`.
- [ ] Both packages are bootstrapped using the quality gate baseline from [[spec-00006-typescript-quality-gate-contract-for-vs-code-extensions]].
- [ ] The landing site explains Vector, the MCP server, and the VS Code extension.
- [ ] The documentation site contains a product-focused structure for installation, configuration, and usage guidance.
- [ ] Both sites provide clear cross-linking between marketing and documentation entry points.
- [ ] No Turbo or equivalent monorepo orchestration is introduced as part of this RFC.

## 6. Open Questions

- Should both sites be deployed under the same domain with subpaths or as separate subdomains?
- How much visual and component reuse should be enforced before introducing shared frontend packages?
- Should [[spec-00006-typescript-quality-gate-contract-for-vs-code-extensions]] be generalized into a web package contract once Astro-specific needs become clearer?
- Will both sites share one CI delivery workflow or maintain separate deployment pipelines?
