---
id: project-0002-principles
type: project
code: "0002"
slug: principles
title: Project Principles
description: Defines the core design principles that guide the VECTOR project.
created: 2026-05-01
updated: 2026-05-01
tags:
  - project
  - principles
  - architecture
related:
  - spec-00003-project-documentation-folder
  - project-0001-definition
---

# Project Principles

## 1. Decentralization

VECTOR prioritizes decentralized documentation hubs over a single centralized documentation system.

Documentation should live close to the teams, codebases, and workflows that own it while still remaining governable.

## 2. Performance

VECTOR uses Rust as its performance-oriented systems layer and favors efficient local execution characteristics.

This principle also informs transport choices, including the use of `stdio` for the MCP transport in the initial architecture.

## 3. Extensibility

VECTOR is designed as a small core that can grow through plugins.

Core capabilities should remain focused, while feature expansion should be possible without forcing all behavior into the base implementation.

## 4. Upgradability

VECTOR is designed so that it can update itself when needed.

Upgrade paths must be considered part of the product architecture rather than an afterthought.

## 5. Discoverability

VECTOR tools must be discoverable through the plugin system.

Plugins should be able to expose capabilities in a way that remains visible and understandable to users and clients consuming the MCP surface.

## 6. Configurability

VECTOR must not impose a single development workflow model on every project.

It should allow projects to define different ways of working because governance needs, release practices, and tooling constraints vary across teams.
