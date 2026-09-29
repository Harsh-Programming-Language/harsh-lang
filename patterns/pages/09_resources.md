# Additional Resources

*Original: [Additional Resources](https://rust-unofficial.github.io/patterns/additional_resources/index.html)*

The original closes with material behind the patterns rather than patterns
themselves; its main page is the design principles.

## 9.1 Design principles

*Original: [Design principles](https://rust-unofficial.github.io/patterns/additional_resources/design-principles.html)*

Principles older than Rust, which hold in Harsh unchanged; this companion
names them and leaves their discussion to the original:

- **SOLID** — a unit has one responsibility; open to extension, closed to
  modification; subtypes stand in for their types; small, specific
  interfaces; depend on abstractions (in Rust: traits), not concrete types.
- **Composite reuse, or composition over inheritance** — which Rust enforces,
  having no inheritance; see *Deref Polymorphism*.
- **DRY** — every piece of knowledge has one representation in the system.
- **KISS** — most systems work best kept simple.
- **Law of Demeter** — a unit talks to its immediate collaborators only.
- **Design by contract** — preconditions, postconditions and invariants,
  stated and checked.
- **Encapsulation** — the invariants a module keeps are its own business; see
  *Contain unsafety in small modules*.
- **Command–query separation** — a function either changes something or
  answers a question, not both.
- **Principle of least astonishment** — a component behaves as its users
  expect.
- **Linguistic modular units** — modules are units of the language itself, as
  Rust's `mod` and crates are.
- **Self-documentation** — the documentation lives with the code it
  describes, as `///` comments and their tested examples do.
- **Uniform access** — a service is used the same way whether it is stored or
  computed.
- **Single choice** — where a system must choose among alternatives, one
  module alone knows the full list — as one `enum` and its `match`es do.
- **Persistence closure** — storing a value stores everything it depends on.

Harsh's own principle is the one Rust keeps: say what you mean, and let the
compiler check it. The layout and the pipe only make what you mean easier to
read.
