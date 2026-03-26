**Stella** is a Rust-inspired programming language that compiles to JVM bytecode. It is designed primarily for modding but remains fully general-purpose, with a focus on reducing Java’s verbosity.

One motivation for Stella is Java’s multi-file structure. Stella avoids this by internally concatenating all source files during compilation, then automatically splitting the result into `.class` files.
The language uses two file types:

* **`.stella`** — the main plain-text source file
* **`.stellab`** — a binary format

`.stellab` files are pickled Rust structs (the compiler is written in Rust) that store parsed Stella ASTs immediately before concatenation. These binaries enable pre-coded headers, methods, libraries, bindings, or other reusable components.
For example, `fabric-core.stellab` can configure the main class to inherit from `JavaPlugin` while automatically including the required Java Fabric dependencies.
Although `.stellab` files may provide bindings for specific methods, Stella also supports direct Java interoperability, since it ultimately compiles to JVM bytecode.