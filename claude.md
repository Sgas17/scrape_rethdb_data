Claude Code Configuration for Rust Projects
🚨 CRITICAL: RUST PARALLEL EXECUTION PATTERNS

MANDATORY RULE: Rust projects require memory-safe coordination with Cargo parallel compilation and ownership management.
🚨 CRITICAL: CONCURRENT EXECUTION FOR ALL RUST OPERATIONS

ABSOLUTE RULE: ALL Rust operations MUST be concurrent/parallel in a single message:
🔴 MANDATORY CONCURRENT PATTERNS FOR RUST:

    Cargo Operations: ALWAYS batch ALL cargo build/test/run commands
    Crate Management: ALWAYS batch ALL dependency installations
    Testing: ALWAYS run ALL test suites in parallel with cargo test
    Memory Safety: ALWAYS batch ALL borrowing/ownership patterns
    Concurrency: ALWAYS batch ALL async/threading implementations

⚡ RUST GOLDEN RULE: "1 MESSAGE = ALL MEMORY-SAFE OPERATIONS"

Examples of CORRECT Rust concurrent execution:

// ✅ CORRECT: Everything in ONE message
[Single Message]:
  - TodoWrite { todos: [10+ todos with all Rust tasks] }
  - Task("You are Rust architect. Coordinate via hooks for ownership design...")
  - Task("You are Systems programmer. Coordinate via hooks for performance...")
  - Task("You are Safety engineer. Coordinate via hooks for memory safety...")
  - Bash("cargo new my-rust-app --bin")
  - Bash("cd my-rust-app && cargo add serde tokio reqwest")
  - Bash("cd my-rust-app && cargo add --dev proptest criterion")
  - Write("Cargo.toml", cargoConfiguration)
  - Write("src/main.rs", mainApplication)
  - Write("src/lib.rs", libraryModule)
  - Write("src/models/user.rs", userStruct)
  - Write("src/services/api.rs", apiService)
  - Write("src/utils/helpers.rs", utilityFunctions)
  - Write("tests/integration_test.rs", integrationTests)
  - Bash("cd my-rust-app && cargo build && cargo test && cargo run")

🎯 RUST-SPECIFIC SWARM PATTERNS
🦀 Cargo Project Coordination

Rust Project Setup Strategy:

# Always batch Cargo operations
cargo new my-app --bin
cargo add serde serde_json tokio
cargo add --dev proptest criterion
cargo build --release
cargo test

Parallel Development Setup:

// ✅ CORRECT: All setup in ONE message
[BatchTool]:
  - Bash("cargo new rust-project --bin")
  - Bash("cd rust-project && cargo add serde serde_json tokio reqwest")
  - Bash("cd rust-project && cargo add --dev proptest criterion mockall")
  - Write("Cargo.toml", optimizedCargoToml)
  - Write("src/main.rs", asyncMainFunction)
  - Write("src/lib.rs", libraryRoot)
  - Write("src/config.rs", configurationModule)
  - Write("src/error.rs", errorHandlingTypes)
  - Write("src/models/mod.rs", modelsModule)
  - Write("tests/common/mod.rs", testUtilities)
  - Bash("cd rust-project && cargo build && cargo clippy && cargo test")

🏗️ Rust Agent Specialization

Agent Types for Rust Projects:

    Systems Architect Agent - Memory management, ownership patterns
    Performance Engineer Agent - Zero-cost abstractions, optimization
    Safety Specialist Agent - Borrow checker, lifetime management
    Concurrency Expert Agent - Async/await, threading, channels
    Testing Agent - Unit tests, integration tests, property testing
    Ecosystem Agent - Crate selection, FFI, WebAssembly

🔧 Memory Safety Coordination

Ownership and Borrowing Patterns:

// Memory safety coordination
[BatchTool]:
  - Write("src/ownership/smart_pointers.rs", smartPointerExamples)
  - Write("src/ownership/lifetimes.rs", lifetimePatterns)
  - Write("src/ownership/borrowing.rs", borrowingExamples)
  - Write("src/memory/allocator.rs", customAllocatorUsage)
  - Write("src/safety/invariants.rs", safetyInvariants)
  - Write("tests/memory_safety.rs", memorySafetyTests)
  - Bash("cargo build && cargo miri test")

⚡ Async/Concurrency Coordination

Tokio Async Runtime Setup:

// Async coordination pattern
[BatchTool]:
  - Write("src/async/runtime.rs", tokioRuntimeConfig)
  - Write("src/async/tasks.rs", asyncTaskHandling)
  - Write("src/async/channels.rs", channelCommunication)
  - Write("src/async/streams.rs", asyncStreamProcessing)
  - Write("src/network/client.rs", asyncHttpClient)
  - Write("src/network/server.rs", asyncWebServer)
  - Write("tests/async_tests.rs", asyncTestCases)
  - Bash("cargo test --features async")

🧪 RUST TESTING COORDINATION
⚡ Comprehensive Testing Strategy

Testing Setup:

// Test coordination pattern
[BatchTool]:
  - Write("tests/integration_test.rs", integrationTests)
  - Write("tests/common/mod.rs", testUtilities)
  - Write("src/lib.rs", unitTestsInline)
  - Write("benches/benchmark.rs", criterionBenchmarks)
  - Write("proptest-regressions/", propertyTestRegressions)
  - Write("tests/property_tests.rs", proptestCases)
  - Bash("cargo test --all-features")
  - Bash("cargo bench")
  - Bash("cargo test --doc")

🔬 Property Testing and Fuzzing

Advanced Testing Coordination:

[BatchTool]:
  - Write("fuzz/fuzz_targets/fuzz_parser.rs", fuzzingTargets)
  - Write("tests/quickcheck_tests.rs", quickcheckTests)
  - Write("tests/model_based_tests.rs", modelBasedTesting)
  - Bash("cargo fuzz run fuzz_parser")
  - Bash("cargo test --features property-testing")

🚀 RUST PERFORMANCE COORDINATION
⚡ Performance Optimization

Performance Enhancement Batch:

[BatchTool]:
  - Write("src/performance/simd.rs", simdOptimizations)
  - Write("src/performance/zero_copy.rs", zeroCopyPatterns)
  - Write("src/performance/cache_friendly.rs", cacheOptimization)
  - Write("src/performance/profiling.rs", profilingIntegration)
  - Write("benches/performance_bench.rs", performanceBenchmarks)
  - Write("Cargo.toml", releaseOptimizations)
  - Bash("cargo build --release")
  - Bash("cargo bench --all-features")
  - Bash("perf record cargo run --release")

🔄 Parallel Processing

Rayon Parallel Coordination:

// Parallel processing batch
[BatchTool]:
  - Write("src/parallel/rayon_examples.rs", rayonParallelization)
  - Write("src/parallel/custom_threadpool.rs", customThreadPool)
  - Write("src/parallel/work_stealing.rs", workStealingQueues)
  - Write("src/data/parallel_processing.rs", parallelDataProcessing)
  - Bash("cargo add rayon crossbeam")
  - Bash("cargo test parallel_")

🌐 RUST WEB DEVELOPMENT COORDINATION
🕸️ Web Framework Integration

Axum/Warp Web Service Setup:

// Web development coordination
[BatchTool]:
  - Write("src/web/server.rs", axumWebServer)
  - Write("src/web/handlers.rs", requestHandlers)
  - Write("src/web/middleware.rs", customMiddleware)
  - Write("src/web/routes.rs", routingConfiguration)
  - Write("src/database/connection.rs", databaseIntegration)
  - Write("src/models/schema.rs", databaseSchema)
  - Write("migrations/001_initial.sql", databaseMigrations)
  - Bash("cargo add axum tokio tower sqlx")
  - Bash("cargo run --bin server")

🗄️ Database Integration

SQLx Database Coordination:

// Database integration batch
[BatchTool]:
  - Write("src/database/models.rs", databaseModels)
  - Write("src/database/queries.rs", sqlQueries)
  - Write("src/database/migrations.rs", schemaMigrations)
  - Write("src/database/connection_pool.rs", connectionPooling)
  - Write("tests/database_tests.rs", databaseTests)
  - Bash("cargo add sqlx --features runtime-tokio-rustls,postgres")
  - Bash("sqlx migrate run")

🔒 RUST SECURITY COORDINATION
🛡️ Security Best Practices

Security Implementation Batch:

[BatchTool]:
  - Write("src/security/crypto.rs", cryptographicOperations)
  - Write("src/security/validation.rs", inputValidation)
  - Write("src/security/auth.rs", authenticationLogic)
  - Write("src/security/sanitization.rs", dataSanitization)
  - Write("src/security/secrets.rs", secretsManagement)
  - Write("audit.toml", cargoAuditConfig)
  - Bash("cargo add ring argon2 jsonwebtoken")
  - Bash("cargo audit")
  - Bash("cargo deny check")

Rust Security Checklist:

    Memory safety by design
    Integer overflow protection
    Secure random number generation
    Constant-time cryptographic operations
    Input validation and sanitization
    Dependency vulnerability scanning
    Safe FFI interfaces
    Secure compilation flags

🔧 RUST BUILD COORDINATION
📦 Cargo Advanced Configuration

Advanced Cargo Setup:

// Advanced build coordination
[BatchTool]:
  - Write("Cargo.toml", advancedCargoConfig)
  - Write(".cargo/config.toml", cargoLocalConfig)
  - Write("build.rs", buildScript)
  - Write("Cross.toml", crossCompilationConfig)
  - Write("Dockerfile", rustDockerfile)
  - Bash("cargo build --target x86_64-unknown-linux-musl")
  - Bash("cross build --target aarch64-unknown-linux-gnu")

🎯 WebAssembly Coordination

WASM Integration Setup:

// WebAssembly coordination
[BatchTool]:
  - Write("src/wasm/lib.rs", wasmBindings)
  - Write("src/js/wasm_interface.js", jsWasmInterface)
  - Write("pkg/package.json", wasmPackageJson)
  - Write("webpack.config.js", wasmWebpackConfig)
  - Bash("cargo add wasm-bindgen web-sys js-sys")
  - Bash("wasm-pack build --target web")
  - Bash("npm run serve")

🚀 RUST DEPLOYMENT COORDINATION
⚙️ Production Deployment

Deployment Configuration:

[BatchTool]:
  - Write("Dockerfile", optimizedRustDockerfile)
  - Write("docker-compose.yml", dockerComposeRust)
  - Write("k8s/deployment.yaml", kubernetesDeployment)
  - Write("scripts/deploy.sh", deploymentScript)
  - Write("systemd/rust-service.service", systemdService)
  - Bash("cargo build --release --target x86_64-unknown-linux-musl")
  - Bash("docker build -t rust-app:latest .")
  - Bash("kubectl apply -f k8s/")

📦 Distribution and Packaging

Crate Publishing Coordination:

[BatchTool]:
  - Write("README.md", crateDocumentation)
  - Write("CHANGELOG.md", versionHistory)
  - Write("LICENSE", licenseFile)
  - Write("src/lib.rs", publicApiDocumentation)
  - Write("examples/basic_usage.rs", usageExamples)
  - Bash("cargo doc --open")
  - Bash("cargo package --dry-run")
  - Bash("cargo publish --dry-run")

📊 RUST CODE QUALITY COORDINATION
🎨 Code Quality Tools

Quality Toolchain Batch:

[BatchTool]:
  - Write("rustfmt.toml", rustfmtConfiguration)
  - Write("clippy.toml", clippyConfiguration)
  - Write(".gitignore", rustGitignore)
  - Write("deny.toml", cargoServerDenyConfig)
  - Write("rust-toolchain.toml", toolchainConfiguration)
  - Bash("cargo fmt --all")
  - Bash("cargo clippy --all-targets --all-features -- -D warnings")
  - Bash("cargo deny check")

📝 Documentation Coordination

Documentation Generation:

[BatchTool]:
  - Write("src/lib.rs", comprehensiveDocComments)
  - Write("docs/architecture.md", architecturalDocs)
  - Write("docs/api.md", apiDocumentation)
  - Write("examples/", codeExamples)
  - Bash("cargo doc --no-deps --open")
  - Bash("cargo test --doc")

🔄 RUST CI/CD COORDINATION
🏗️ GitHub Actions for Rust

CI/CD Pipeline Batch:

[BatchTool]:
  - Write(".github/workflows/ci.yml", rustCI)
  - Write(".github/workflows/security.yml", securityWorkflow)
  - Write(".github/workflows/release.yml", releaseWorkflow)
  - Write("scripts/ci-test.sh", ciTestScript)
  - Write("scripts/security-audit.sh", securityAuditScript)
  - Bash("cargo test --all-features")
  - Bash("cargo clippy --all-targets -- -D warnings")
  - Bash("cargo audit")

💡 RUST BEST PRACTICES
📝 Code Design Principles

    Ownership Model: Understand borrowing and lifetimes
    Zero-Cost Abstractions: Write high-level code with low-level performance
    Error Handling: Use Result and Option types effectively
    Memory Safety: Eliminate data races and memory bugs
    Performance: Leverage compiler optimizations
    Concurrency: Safe parallel programming patterns

🎯 Advanced Patterns

    Type System: Leverage advanced type features
    Macros: Write declarative and procedural macros
    Unsafe Code: When and how to use unsafe blocks
    FFI: Foreign function interface patterns
    Embedded: Bare metal and embedded development
    WebAssembly: Compile to WASM targets

📚 RUST LEARNING RESOURCES
🎓 Recommended Topics

    Core Rust: Ownership, borrowing, lifetimes
    Advanced Features: Traits, generics, macros
    Async Programming: Tokio, async/await patterns
    Systems Programming: Low-level development
    Web Development: Axum, Warp, Rocket frameworks
    Performance: Profiling, optimization techniques

🔧 Essential Tools

    Toolchain: rustc, cargo, rustup, clippy
    IDEs: VS Code with rust-analyzer, IntelliJ Rust
    Testing: Built-in test framework, proptest, criterion
    Debugging: gdb, lldb, rr (record and replay)
    Profiling: perf, valgrind, cargo-flamegraph
    Cross-compilation: cross, cargo-zigbuild

🌟 Ecosystem Highlights

    Web Frameworks: Axum, Actix-web, Warp, Rocket
    Async Runtime: Tokio, async-std, smol
    Serialization: Serde, bincode, postcard
    Databases: SQLx, Diesel, sea-orm
    CLI Tools: Clap, structopt, colored
    Graphics: wgpu, bevy, ggez, nannou
