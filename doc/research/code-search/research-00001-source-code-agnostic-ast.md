---
id: research-00001-source-code-agnostic-ast
type: research
code: "00001"
slug: source-code-agnostic-ast
title: Source Code Agnostic AST
description: Research on a language-neutral AST abstraction for code search across procedural, functional, object-oriented, and other programming paradigms.
category: code-search
created: 2026-06-09
updated: 2026-06-09
tags:
  - research
  - code-search
  - ast
  - language-analysis
related: []
---

# Source Code Agnostic AST

## Context

Vector needs a way to improve code search across many programming languages without binding the search model to each language-specific AST shape. Every language exposes different syntax, node names, declaration forms, and body structures, but many code intelligence workflows need the same conceptual questions:

- Where is a function, method, procedure, macro, or callable unit defined?
- Where is a callable unit invoked?
- Which top-level components exist in a file, module, namespace, package, or compilation unit?
- Which components own, reference, export, import, override, or compose other components?

The goal is not to choose parsing libraries. The goal is to define a theoretical intermediate representation that can normalize useful source-code facts while preserving enough language-specific detail to avoid losing meaning.

## Research Goal

Define a source-code agnostic AST abstraction that supports broad search and code-navigation use cases across procedural, functional, object-oriented, declarative, concurrent, and metaprogramming-heavy languages.

The abstraction should be:

- **Searchable:** optimized for indexing definitions, references, calls, scopes, and relationships.
- **Language-neutral:** based on semantic roles instead of parser-specific node names.
- **Loss-aware:** explicit about what cannot be normalized safely.
- **Extensible:** able to attach language-specific facts without polluting the common model.
- **Incremental:** suitable for updating indexes when files change.

## Core Idea

A generic model should not try to erase every language difference. It should separate the representation into two layers:

1. **Universal code graph:** normalized entities, scopes, relationships, and executable regions.
2. **Language evidence:** original syntax spans, raw AST node references, modifiers, annotations, and parser-specific metadata.

This avoids forcing every language into one artificial tree while still giving search a stable vocabulary.

## Proposed Universal Concepts

### Source Unit

A source unit is the smallest indexed source artifact. It is usually a file, notebook cell, generated source fragment, module file, or script.

Useful fields:

- Stable source unit identifier
- Language identifier
- Path or logical module name
- Content hash
- Parse status
- List of top-level entities
- Diagnostics and confidence level

### Entity

An entity is a named or addressable component in code.

Common entity kinds:

- Package
- Module
- Namespace
- Type
- Interface
- Trait
- Protocol
- Enum
- Object
- Function
- Method
- Constructor
- Procedure
- Macro
- Variable
- Constant
- Field
- Property
- Parameter
- Type parameter
- Test case
- Route or endpoint
- Query
- Schema
- Task or workflow step

Entities should support anonymous forms. Anonymous functions, blocks, classes, records, and callbacks may still be important search targets when they are assigned, passed, exported, or invoked.

### Scope

A scope defines where names can be resolved.

Common scope kinds:

- Global scope
- Package scope
- Module scope
- Type scope
- Function scope
- Block scope
- Pattern scope
- Comprehension scope
- Macro expansion scope
- Template or generic scope

Scope is essential for call search because the same textual name can refer to different symbols depending on imports, lexical bindings, receivers, and shadowing.

### Relationship

Relationships turn the normalized AST into a code graph.

Core relationship kinds:

- Defines
- Contains
- Imports
- Exports
- References
- Calls
- Reads
- Writes
- Instantiates
- Extends
- Implements
- Overrides
- Decorates
- Annotates
- Matches
- Throws
- Catches
- Awaits
- Yields
- Sends
- Receives
- Subscribes
- Publishes
- Registers

Search should index relationships independently from tree shape. For example, a function call, a method dispatch, an operator overload, and a macro invocation may all produce a `calls` relationship with different confidence and resolution metadata.

### Executable Region

An executable region is a body where evaluation order, side effects, calls, and control flow can occur.

Examples:

- Function body
- Method body
- Constructor body
- Lambda body
- Module initialization body
- Class initialization block
- Comprehension body
- Pattern guard
- Macro body
- Query body
- Template body
- Test body

Executable regions are the main unit for body analysis and function-call search.

### Expression and Statement Roles

Different languages disagree on whether constructs are expressions or statements. A generic model should normalize by role:

- Declaration role
- Binding role
- Invocation role
- Selection role
- Assignment role
- Control-flow role
- Pattern-matching role
- Literal role
- Construction role
- Access role
- Error-handling role
- Concurrency role
- Type-level role

This supports expression-oriented languages, statement-oriented languages, and languages where type-level computation is part of the program.

## Search Use Cases

### Function and Callable Search

The model should find callable definitions even when the language does not use the word "function".

Examples:

- Procedures in procedural languages
- Methods and constructors in object-oriented languages
- Functions and multimethods in functional languages
- Macros and compile-time callable forms
- Operators implemented as functions or methods
- Anonymous callables assigned to names
- Callable objects
- Test definitions
- Endpoint handlers
- Event handlers

Searchable fields:

- Name and aliases
- Qualified name
- Parameters
- Return information when available
- Visibility
- Modifiers
- Annotations
- Owning scope
- Body span
- Documentation span
- Export status

### Function Call Search

Call search should cover direct calls and indirect invocation forms.

Examples:

- `foo()`
- `object.method()`
- `new Type()`
- Operator calls
- Higher-order function calls
- Callback registration
- Dynamic dispatch
- Message sending
- Macro invocation
- Decorator application
- Pipeline or chaining forms
- Awaited calls
- Reflection-based calls when statically visible

Each call edge should include:

- Call site span
- Callee text
- Resolved target when known
- Receiver expression when present
- Argument count and named arguments
- Dispatch kind
- Confidence level
- Whether the edge is static, inferred, dynamic, or unresolved

### Top-Level Component Search

Top-level search should identify the meaningful structure of a source unit.

Examples:

- Modules
- Namespaces
- Classes
- Interfaces
- Traits
- Functions
- Constants
- Imports and exports
- Type aliases
- Schemas
- Routes
- Tests
- Build tasks
- Configuration declarations

This use case is especially important for repository summarization and navigation.

### Additional Use Cases

A source-code agnostic AST can also support:

- Dependency discovery between modules and packages
- Impact analysis for changed components
- Dead-code detection
- Public API inventory
- Ownership and boundary analysis
- Test coverage mapping from tests to components
- Security-sensitive sink and source search
- Data-flow entry point discovery
- Error-handling pattern search
- Concurrency and async boundary discovery
- Framework convention indexing
- Documentation extraction
- Duplicate abstraction detection
- Migration planning across languages
- Architectural rule enforcement
- Search by annotation, decorator, attribute, or metadata
- Search for generated code boundaries
- Search for unresolved or low-confidence references

## Paradigm Coverage

### Procedural Languages

Procedural languages emphasize procedures, variables, control flow, and shared state. The generic model should preserve:

- Procedures and functions as callable entities
- File, module, and block scopes
- Global and local variables
- Reads and writes
- Control-flow structures
- Calls and side effects
- Include or import mechanisms

Main risk: global mutable state and preprocessor behavior can make relationship resolution imprecise.

### Functional Languages

Functional languages emphasize expressions, immutable bindings, higher-order functions, recursion, pattern matching, and algebraic data types.

The model should preserve:

- Bindings as first-class definitions
- Anonymous functions
- Higher-order calls
- Pattern scopes
- Match relationships
- Type constructors and variants
- Function composition and pipeline forms
- Lazy evaluation boundaries when relevant

Main risk: call relationships may be indirect because functions are values and dispatch can depend on runtime composition.

### Object-Oriented Languages

Object-oriented languages emphasize types, objects, methods, inheritance, interfaces, encapsulation, and dynamic dispatch.

The model should preserve:

- Types as entities
- Methods, constructors, fields, and properties
- Inheritance and implementation edges
- Override relationships
- Receiver-aware calls
- Visibility and modifiers
- Static versus instance context

Main risk: dynamic dispatch means a call site may map to many possible targets.

### Declarative Languages

Declarative languages describe desired state, queries, schemas, or rules rather than step-by-step execution.

The model should preserve:

- Rules, facts, queries, resources, and schemas as entities
- References between declarations
- Dependency and containment relationships
- Evaluation or resolution scopes
- Query bodies as executable or evaluable regions when useful

Main risk: "call" may not be the right primitive. The model may need `depends_on`, `matches`, or `resolves_to` relationships.

### Concurrent and Actor-Based Languages

Concurrent languages may use actors, channels, async tasks, futures, coroutines, or message passing.

The model should preserve:

- Spawned tasks
- Await points
- Send and receive relationships
- Channel or topic references
- Callback and handler registration
- Synchronization boundaries

Main risk: execution order and ownership are often not statically obvious.

### Metaprogramming and Macro Systems

Metaprogramming can create, transform, or invoke code at compile time or runtime.

The model should preserve:

- Macro definitions
- Macro invocations
- Generated-code spans when available
- Expansion relationships
- Template parameters
- Confidence level for generated entities

Main risk: a source-level index can miss generated definitions and calls unless expansion artifacts are available.

## Normalization Strategy

The abstraction should prefer semantic roles over syntax names.

Recommended normalized families:

- `source_unit`
- `scope`
- `entity`
- `executable_region`
- `binding`
- `reference`
- `invocation`
- `type_relation`
- `module_relation`
- `control_flow`
- `data_access`
- `metadata`
- `diagnostic`

Each normalized node should keep:

- Stable identifier
- Kind
- Name when available
- Qualified name when available
- Source span
- Parent scope
- Raw language kind
- Confidence level
- Relationship edges
- Optional language-specific properties

## Confidence Model

The model should distinguish facts by certainty.

Suggested confidence levels:

- **Exact:** the parser and resolver identify the target directly.
- **Inferred:** the target is inferred through local syntax and scope.
- **Candidate:** multiple possible targets exist.
- **Textual:** the relationship is based on token shape without semantic resolution.
- **Unknown:** the construct is recognized, but the relationship cannot be resolved.

This is important because useful search can tolerate imperfect results if uncertainty is visible and queryable.

## Gaps

- Cross-language symbol resolution is harder than AST normalization.
- Dynamic features, reflection, macros, monkey patching, and generated code can hide definitions and calls.
- Build systems and dependency managers often provide context that a source file alone cannot provide.
- Some languages make top-level execution meaningful, while others restrict top-level code to declarations.
- Type-level computation and compile-time evaluation may need separate treatment.
- Notebook, literate programming, and mixed-language files complicate the source unit model.

## Flaws and Risks

- A model that is too generic can become shallow and lose the details that make search precise.
- A model that includes too many language-specific fields can become difficult to query consistently.
- Call search can produce misleading results if unresolved dynamic dispatch is shown as exact.
- Treating all top-level constructs as equivalent can hide important differences between public API, private helpers, tests, configuration, and generated code.
- Framework conventions may require semantic adapters beyond the core language model.

## Tradeoffs

- **Tree versus graph:** ASTs preserve syntax structure, while graphs better represent definitions, references, and calls. A graph should be the primary search model, with links back to AST spans for evidence.
- **Precision versus coverage:** strict resolution gives high-quality edges but misses dynamic code. Textual and inferred edges improve coverage but need confidence labels.
- **Universal schema versus adapters:** a universal schema improves query consistency, but language adapters are still required to map real syntax into the schema.
- **Static-only versus build-aware indexing:** static parsing is fast and broad, but build-aware indexing can resolve imports, generated code, and type information more accurately.
- **Minimal entity set versus rich domain concepts:** a small model is easier to maintain, but richer concepts such as routes, tests, schemas, and workflows make search more useful.

## Recommendation

Vector should model source code as a language-neutral code graph backed by source spans and raw AST evidence. The common graph should focus on entities, scopes, executable regions, and relationships. Language adapters should map parser-specific constructs into this graph and attach language-specific metadata only when it improves search.

The first useful milestone should support:

- Source units
- Top-level entities
- Callable entities
- Callable body spans
- Import and export relationships
- Lexical scopes
- Invocation edges with confidence levels
- Raw source spans for every normalized fact

This gives enough structure for function search, call search, and top-level component search while leaving room for stronger symbol resolution later.

## Open Questions

- What minimum confidence level should be indexed by default?
- Should the first version include type relationships, or should those come after callable and module relationships?
- How should generated code be represented when the generated source is unavailable?
- Should framework-level concepts such as routes, jobs, migrations, and tests be part of the core model or extension metadata?
- How should mixed-language source units be split and linked?
