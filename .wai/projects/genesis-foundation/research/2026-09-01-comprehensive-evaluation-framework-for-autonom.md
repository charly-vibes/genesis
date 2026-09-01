# **Comprehensive Evaluation Framework for Autonomous CLI Tool Interaction in the Charly-Vibes Infrastructure**

The evaluation of autonomous language model agents operating within command-line interfaces requires a fundamental shift from traditional end-task code generation benchmarks to structural protocol evaluation1. Standard software engineering benchmarks primarily measure whether an agent can produce code that passes a pre-existing unit test suite2. However, when evaluating developer tools explicitly engineered for LLM agent interaction—such as the charly-vibes suite—the central objective expands to evaluating communication fidelity, deliberate protocol adherence, state-machine integrity, and error-recovery responsiveness1.  
The charly-vibes suite, supported by the shared Rust crate genesis-vibes, establishes an Agent Experience (AIX) infrastructure1. This infrastructure communicates with agents through structured channels: JSON envelopes emitting standard metadata (ok, envelope\_version, data, warnings, hints), self-healing error suggestions (DidYouMean and Fix records), managed block injection markers within AGENTS.md, structured context discovery files (llms.txt), tool registration configurations (.genesis/tools.toml), and explicit epistemic state machines (dont)1. Validating whether agents actively parse, comprehend, and respond to these explicit communication channels—rather than relying on brute-force guessing or hallucinating CLI outputs—demands an evaluation suite designed around black-box subprocess interaction, contrived failure injection, and deterministic state checking1.

## **Landscape of Agentic CLI and Terminal Evaluation Benchmarks**

A comprehensive review of existing terminal and agentic evaluation frameworks reveals distinct operational paradigms across environment isolation, scoring mechanisms, and target capabilities4. The current landscape spans benchmarks targeting repository-level issue resolution, multi-application operating system workflows, interactive shell execution, and zero-to-one library synthesis3.

### **Overview of Prior Art**

SWE-bench and its variants (including SWE-bench Verified and SWE-bench Live) evaluate agents on their ability to resolve real-world GitHub issues within complex Python repositories3. SWE-bench evaluates performance deterministically by running repo-level test suites (fail-to-pass and pass-to-pass tests) within Docker containers3. While SWE-bench remains a foundational benchmark, it suffers from rising contamination risks due to public GitHub data exposure, benchmark saturation among frontier models, high execution costs, and test suite fragility2. SWE-smith addresses data scaling by synthetically generating execution environments and task instances, while mini-SWE-agent provides lightweight execution scaffolding4.  
Terminal-Bench, orchestrated via the Harbor framework, measures agent capabilities across complex command-line tasks, including protein assembly, asynchronous code debugging, security vulnerability patching, and system administration5. Tasks run in containerized environments, and scoring relies on custom verification scripts checking environment state changes10. Terminal-Bench 2.1 addresses historical vulnerabilities related to reward hacking and resource timeouts10.  
OSWorld evaluates multimodal agents across open-ended computer tasks in full operating system environments spanning Ubuntu, Windows, and macOS6. OSWorld 2.0 expands this scope to long-horizon workflows averaging over 300 steps per task, relying on fine-grained checkpoint evaluations across execution states16. However, OSWorld experiences visual grounding brittleness, high compute costs, and severe trajectory drift over extended horizons6.  
InterCode standardizes interactive coding environments using Bash and SQL action spaces7. It evaluates agents in Dockerized REPL style loops, focusing on step-by-step state changes and execution feedback7. While effective for short-horizon command execution, InterCode does not cover multi-tool developer workflows or protocol-driven CLI envelopes1.  
Aider's polyglot and refactoring leaderboards evaluate multi-file editing and code transformation fidelity using structured edit blocks9. RefactorBench specifically measures an agent's ability to refactor large methods within complex classes, evaluating correctness via automated unit tests21. These benchmarks emphasize code-editing syntax adherence rather than interactive tool discovery or error-hint processing1.  
Commit0 and ProgramBench evaluate an agent's ability to architect and implement entire software libraries from scratch given only documentation and high-level specifications8. ProgramBench strips implementation details from open-source repositories and requires the agent to produce executable code that matches the reference binary's behavioral test suite8. These benchmarks reveal severe floor effects, with leading models achieving near-zero full resolution rates on complex compiled languages due to architectural and long-horizon design limits8.  
Inspect AI, developed by the UK AI Safety Institute, provides an open-source evaluation framework designed for frontier model evaluation26. Inspect treats evaluations as composable programs comprising datasets, solvers, and scorers27. It features native sandboxing across Docker, Kubernetes, Modal, and Proxmox, alongside trace logging compatible with VS Code inspection tools27.

| Benchmark / Framework | Primary Target & Capabilities Measured | Scoring Mechanism | Sandboxing Model | Documented Criticisms & Vulnerabilities | Key Source Citations |
| :---- | :---- | :---- | :---- | :---- | :---- |
| **SWE-bench / Verified** | GitHub issue resolution in Python codebases; multi-file patch generation3. | Deterministic repository test suites (fail-to-pass and pass-to-pass)2. | Containerized Docker containers per task instance3. | Data contamination from public GitHub issues; benchmark saturation; high execution cost2. | 2 |
| **Terminal-Bench / Harbor** | Multi-step command-line execution, async debugging, security patching5. | Custom verification scripts checking container state artifacts10. | Isolated containers managed via Harbor orchestration engine10. | Reward hacking vulnerabilities in early tasks; variable latency10. | 5 |
| **OSWorld / OSWorld 2.0** | Open-ended GUI and OS workflows across web, desktop, and OS file systems6. | Execution-based checkpoint evaluation (averaging 27+ state checks)15. | Virtual Machines (QEMU / AWS / Docker) with full OS state capture6. | Extreme evaluation cost; visual grounding brittleness; context drift over 300+ steps6. | 6 |
| **InterCode** | Interactive Bash scripting and SQL query generation7. | Interactive execution feedback loops with intermediate state checking7. | Docker container instances providing interactive REPL / shell interfaces7. | Synthetic task boundaries; limited long-horizon software maintenance testing7. | 7 |
| **Aider / RefactorBench** | Code editing, multi-file refactoring, and structured code transformations9. | Automated test execution following structured file edit block application9. | Local git repository sandboxes with automated diff checks9. | Focuses heavily on code-editing syntax adherence rather than interactive tool discovery9. | 9 |
| **Commit0 / ProgramBench** | Scratch-pad system architecture and full library generation from specifications8. | Comprehensive unit test suite pass rate against reference binaries8. | Isolated Docker build containers stripping original source code8. | Severe floor effects (0% full resolution across many models); extreme context load8. | 8 |
| **Inspect AI (UK AISI)** | General frontier model evaluation across agentic, tool-use, safety, and reasoning tasks27. | Programmable scorers matching outputs or evaluating trace logs27. | Native support for Docker, Kubernetes, Modal, and Proxmox execution sandboxes27. | Steeper setup curve for complex interactive shell loops compared to basic test frameworks27. | 26 |

## **Differentiating Tool Protocol Comprehension from Task Completion**

Evaluating agent interactions with developer tools requires disentangling two confounded variables: final task success and tool communication adherence1. An agent might successfully complete a task through unguided brute-force editing or guessing, completely ignoring diagnostic hints, state-machine transitions, and JSON envelopes provided by the underlying CLI tool1. Conversely, an agent might exhibit flawless protocol adherence by parsing JSON envelopes and executing correct state transitions, yet fail the end-task due to high-level reasoning limitations1.

### **Protocol Adherence Testing Mechanisms**

To measure protocol comprehension directly, the evaluation suite must instrument and validate the structural artifacts generated during execution1:

> 1. **JSON Envelope Parsing**: Validating whether the agent correctly ingests structured outputs from genesis-vibes tools1. When a tool returns ok: false alongside a structured hints array and a Fix object, the harness checks whether the agent's subsequent invocation incorporates the specific parameter recommended in the hint1.  
> 2. **Epistemic State Machine Compliance**: The dont CLI enforces explicit state transitions (dont conclude, dont flag, claim tracking)1. Evaluation must trace the execution transcript to confirm that the agent passes through valid epistemic states rather than attempting illegal transitions or editing underlying state files directly1.  
> 3. **Managed Block Protocol Integrity**: Tools dynamically update sections of AGENTS.md using managed block markers1. The evaluation harness verifies that the agent respects these markers, preserving structural tags without corrupting surrounding documentation1.  
> 4. **Context Recovery Infrastructure**: Evaluating whether the agent utilizes wai prime and wai status upon encountering ambiguous state errors to re-orient itself, rather than attempting speculative operations1.

### **Contrived-Failure Injection**

The value proposition of the charly-vibes suite relies on self-healing output channels (hints, DidYouMean, doctor \--fix)1. Testing this requires deliberate failure injection1:

* **Syntax and Flag Perturbation**: Invoking tools with intentionally deprecated or slightly mistyped arguments to trigger DidYouMean suggestions1.  
* **State Machine Invalidation**: Placing the repository in an invalid dont epistemic state to observe if the agent reads the diagnostic error and executes the state restoration command recommended in hints1.  
* **Environmental Corruption**: Injecting malformed configuration files into .genesis/tools.toml to test whether the agent invokes the doctor diagnostic tool and applies the automated auto-fix remedies1.

When a contrived failure occurs, the evaluation harness tracks whether the agent's next action directly addresses the failure payload provided in the tool's JSON output1.

### **Eliminating Tool Hallucination Confounds**

A major challenge in agentic CLI evaluation is distinguishing genuine tool execution from model hallucination1. Models frequently output text blocks mimicking CLI output or claim to have executed a command without issuing a subprocess call1.  
To prevent this confound, the evaluation harness must enforce strict black-box subprocess boundary isolation1:

* The agent framework interacts with the environment strictly through a sandboxed terminal interface1.  
* Trajectory verification does not evaluate free-form model text responses1.  
* All deterministic assertions are executed directly against OS-level execution logs, subprocess exit codes, exact JSON envelopes emitted to stdout or stderr, and file system diffs1.

### **A/B Testing Agent Experience (AIX) Artifacts**

To quantify the tangible return on investment of AIX infrastructure (llms.txt, AGENTS.md, wai prime), every task in the evaluation battery is executed under two controlled configurations1:

* **Full AIX Environment**: The sandbox contains complete llms.txt documentation, managed AGENTS.md guidance, and .genesis/tools.toml registries1.  
* **Ablated Environment**: The sandbox contains raw binaries without high-level AIX context artifacts1.

The performance delta (![][image1]) isolates the precise impact of structural documentation and envelope signaling on agent task efficiency and error reduction1.

## **Scoring Methodologies, Judging Mechanics, and Failure Mitigations**

Evaluation scoring must balance absolute mathematical precision with high-level behavioral assessment1. While free-form text generation benefits from LLM-based evaluation, CLI tool usage provides unambiguous machine-readable signals that favor deterministic verification1.

### **Deterministic Mechanics**

The charly-vibes suite emits structured execution traces designed for automated validation1. Scoring mechanisms rely primarily on four deterministic drivers1:

* **Envelope Assertions**: JSON envelope outputs are parsed programmatically1. The harness asserts specific conditions on fields: verifying that envelope.ok \== true, checking that envelope.warnings contains expected deprecation strings, or validating that payload schema matches envelope.envelope\_version specifications1.  
* **State Machine Transcript Diffs**: Epistemic transitions in dont generate persistent audit logs1. The evaluation engine parses these state logs into directed graphs, asserting that the path from initial claim to final resolution strictly obeys state transition rules without back-tracking or invalid leaps1.  
* **Managed Block Boundary Audits**: For tools modifying AGENTS.md or configuration files, the harness computes line-level diffs to confirm that changes occur exclusively within designated \<\!-- GENESIS-MANAGED-START \--\> and \<\!-- GENESIS-MANAGED-END \--\> tags1.  
* **Subprocess Exit Code Mapping**: Validating that terminal commands exit with precise status codes, distinguishing intentional user-facing errors (![][image2]) from internal execution failures or panics (![][image3])1.

### **LLM-as-Judge Mechanics and Failure Mode Mitigations**

When evaluating open-ended trajectories—such as assessing whether an agent's problem-solving trajectory was logical or whether its written commit summaries are coherent—deterministic checks are insufficient1. In these constrained cases, LLM-as-judge scoring is integrated under strict guardrails1.  
Unconstrained LLM judges exhibit well-documented failure modes in agentic evaluations27. These failure modes and their architectural mitigations are structured as follows:

| Judge Failure Mode | Operational Effect | Architectural Mitigation Strategy | Source Citations |
| :---- | :---- | :---- | :---- |
| **Verbosity and Length Bias** | Favoring agents that execute excessive commands or output lengthy explanations27. | Enforcing token-normalized efficiency scoring; truncating uninformative command outputs before scoring27. | 27 |
| **Self-Enhancement Bias** | Judge models systematically favoring trajectories generated by models from their own family27. | Strict judge-model decoupling (e.g., using Anthropic judges for OpenAI runs, and vice versa)27. | 27 |
| **Trajectory Distraction** | Losing focus or missing critical errors when parsing massive terminal logs16. | Pre-processing logs into key-event trajectory diffs before feeding prompts to judges27. | 16 |
| **Unstable Rubric Grading** | Inconsistent numerical scores across identical evaluation runs27. | Evidence-anchored rubrics forcing explicit string citations from execution logs before scoring27. | 27 |

## **Tier-Graded Difficulty, Contamination Defense, and Economic Bounds**

An evaluation battery must maintain discriminatory power across model capability tiers—from compact open-weights models (\<100B parameters) to frontier proprietary systems1. Avoiding floor effects (where weaker models score 0%) and ceiling saturation (where frontier models all achieve 100%) requires systematic difficulty scaling and contamination controls1.

### **Graded Difficulty Levers**

Task difficulty is dynamically adjusted across evaluation batteries using four primary control knobs1:

* **Horizon Length and Step Caps**: Tier 1 tasks require 1–3 tool executions; Tier 2 tasks expand to 5–15 steps; Tier 3 tasks demand long-horizon reasoning spanning 30+ execution turns1.  
* **Distractor Injection**: Injecting unused configuration files, noisy terminal logs, and distractor binaries into .genesis/tools.toml to evaluate whether the agent can focus on relevant tool channels1.  
* **Ambiguity and Discovery**: Weaker tiers receive explicit command syntax instructions. Advanced tiers receive high-level intent prompts, forcing the agent to execute wai prime, inspect llms.txt, and discover tool options independently1.  
* **Budget Constraints**: Imposing strict upper bounds on total execution step counts and API token consumption, testing agent operational efficiency1.

### **Contamination Defense Protocols**

Because the charly-vibes suite documentation (llms.txt, GitHub Pages mdBooks, crates.io READMEs) is publicly accessible, frontier models risk memorizing command syntax and flags1. Evaluating an agent on public syntax measures memory retrieval rather than genuine tool comprehension1.  
To prevent contamination, the suite applies three defense mechanisms1:

* **Synthetic Subcommand and Flag Perturbation**: Dynamically renaming CLI flags during sandbox provisioning (e.g., altering dont conclude to dont finalize--synthetic within .genesis/tools.toml and local docs)1.  
* **Private Scenario Fixtures**: Generating randomized synthetic codebases and custom schema envelopes stored in encrypted, held-out evaluation repositories1.  
* **Post-Cutoff Syntax Versioning**: Introducing subtle tool behavioral shifts not present in training data, enforcing discovery via local llms.txt inspection inside the sandbox1.

### **Economic and Variance Controls**

Agentic evaluations exhibit inherent stochasticity due to model sampling non-determinism and variable tool trajectories1. Single-run pass/fail scores yield misleading signals13.  
To guarantee statistical reliability within a strict budget constraint (\<$5 per full battery run on cost-effective models), the suite implements a multi-tiered execution strategy1:

* **Sample Counts**: Every task scenario runs ![][image4] trials per model tier1.  
* **Statistical Reporting**: Performance is reported using ![][image5], ![][image6], pass-rate variance (![][image7]), and mean token cost per successful task13.  
* **Cost Cap Architecture**: Tier 1 and 2 pre-screening runs utilize low-cost models or local open-weights deployments1. Step limits are enforced via harness hooks; trajectories exceeding 20 steps without progress are terminated early1. Deterministic test filters run before any LLM-as-judge scorer is invoked, avoiding unnecessary judge API calls1.

## **Actionable Failure Taxonomies and Continuous Integration Integration**

To ensure evaluation runs generate actionable feedback for tool developers rather than simple pass/fail ratios, trajectory failures are automatically categorized into a structured error taxonomy1.

### **Categorized Error Taxonomy**

When a trial fails, the evaluation harness parses the execution trace and assigns an error code based on observable signals1:

* **ERR\_ENVELOPE\_HINT\_BLINDNESS**: The tool returned ok: false with a populated hints vector, but the agent's subsequent command ignored the suggested fix1.  
* **ERR\_STATE\_MACHINE\_VIOLATION**: The agent executed an illegal dont state transition (e.g., calling dont conclude without prior verification) or attempted to bypass state checks by manually editing local state files1.  
* **ERR\_MANAGED\_BLOCK\_CORRUPTION**: The agent overwrote or deleted AGENTS.md managed block tags (\<\!-- GENESIS-MANAGED-START \--\>), disrupting future tool injection1.  
* **ERR\_TOOL\_EXECUTION\_HALLUCINATION**: The agent produced text output claiming to have run a command without executing a corresponding subprocess call in the terminal environment1.  
* **ERR\_CONTEXT\_RECOVERY\_FAILURE**: The agent encountered an unexpected error but failed to run wai prime or wai status to refresh its project orientation context1.

### **Continuous Integration Cadence**

Evaluation runs are segregated into two operational cadences to balance developer feedback velocity with operational cost1:

* **Tool Developer CI (Per Commit / Pull Request)**: Executes rapid, zero-cost deterministic regression checks using a mock agent or replay script1. This pipeline verifies that CLI binaries correctly emit JSON envelopes, respect managed block bounds, and maintain exit-code contracts, completing in under two minutes1.  
* **External Model Capability Matrix (Nightly / Scheduled)**: Executes the full multi-model evaluation battery across external LLMs using ![][image4] sampling per scenario1. This pipeline updates model capability matrices, tracks AIX score deltas, and flags prompt or tool instruction regressions1.

## **Evaluation Harness Infrastructure Analysis**

Selecting the evaluation harness infrastructure requires evaluating existing open-source frameworks against the specific sandboxing, subprocess execution, and dogfooding requirements of the charly-vibes suite1.

| Harness Option | Multi-Model Support | Sandboxing Engine | Deterministic Replay & Trace | Cost Controls | Tool Suite Dogfooding Capability | Architectural Suitability | Source Citations |
| :---- | :---- | :---- | :---- | :---- | :---- | :---- | :---- |
| **Inspect AI** | Exceptional (Built-in support for major providers)27. | Native Docker, K8s, Modal, Proxmox sandboxes27. | Full trace logging (.eval files) \+ VS Code trace viewer27. | Configurable token & step caps27. | Medium (Requires Python wrapper calling Rust binaries)1. | **Recommended Primary Framework** for multi-model capability testing27. | 26 |
| **Harbor / Terminal-Bench** | High (Supports diverse API models)5. | Robust Docker container runtime per task10. | Replay logs and container artifact preservation10. | Execution timeout and step limits10. | Medium (Tailored for containerized terminal tasks)10. | Strong alternative for complex multi-container terminal tasks5. | 5 |
| **Promptfoo** | High (Comprehensive API integrations)36. | Local shell command execution; weaker container isolation36. | Assertion-level test output caching40. | Basic request-level caching40. | Low (Primarily tailored for prompt engineering / red teaming)36. | Sub-optimal for interactive long-horizon shell state machines36. | 36 |
| **Braintrust** | High (Multi-provider gateway support)37. | Cloud sandbox environments (Public Preview)37. | Enterprise trace span logging and experiment diffing43. | Capped trial counts and concurrency bounds44. | Low (Hosted platform; external orchestration)37. | High overhead for local CI binary dogfooding37. | 37 |
| **Custom Rust Harness** | High (Via custom HTTP/API integration modules)1. | Native genesis-vibes::fixture temp directory sandboxing1. | Full deterministic binary replay via test logs1. | Direct process-level execution bounds1. | Exceptional (Directly dogfoods genesis-vibes modules)1. | **Recommended for Fast CI Regression** dogfooding1. | 1 |

### **Strategic Harness Recommendation**

A hybrid architectural approach is recommended1:

> 1. **Tool Regression Testing in CI**: Implement a lightweight Custom Rust Harness within the charly-vibes workspace1. This harness directly imports genesis-vibes::fixture and genesis-vibes::envelope to execute rapid, zero-cost deterministic subprocess evaluations on every git commit1.  
> 2. **External Model Capability Assessment**: Utilize Inspect AI as the primary capability evaluation engine27. Inspect AI provides first-class Docker sandboxing, native multi-model support, robust task definitions, and a VS Code visual trace inspector for debugging complex agent trajectories27.

## **Recommended Evaluation Taxonomy and Concrete Specifications**

The recommended evaluation taxonomy spans four core capability domains: Epistemic Discipline (dont), Context Recovery (wai), Contrived Error Recovery (genesis-vibes), and Behavioral Verification (espectacular & testaruda)1.  
Below are four concrete evaluation specifications1.

### **Specification 1: Epistemic State Machine Lifecycle & Failure Recovery (dont)**

* **Target Tool**: dont1.  
* **Capability Measured**: Ability to adhere to epistemic state transition protocols (claim ![][image8] verify ![][image8] conclude) without bypassing controls1.  
* **Task Setup**: A temporary directory initialized with an unverified claim recorded in the local dont state database1.  
* **Agent Prompt**: "You are tasked with verifying the claim 'Authentication tokens expire after 3600 seconds' in this repository. Use dont to log your verification evidence and conclude the claim state. Do not modify state files directly."  
* **Environment Initial State**:  
  * .dont/state.db contains active claim ID CLM-1042 marked as unverified.  
  * Source code file src/auth.rs contains pub const TOKEN\_TTL: u64 \= 3600;.  
* **Deterministic Checks**:  
  1. Parse terminal execution transcript; confirm dont flag \--claim CLM-1042 \--evidence \<file\> was executed before dont conclude.  
  2. Parse stdout envelope of final command; verify envelope.ok \== true and envelope.data.status \== "concluded".  
  3. Verify .dont/state.db git diff to ensure no direct manual file edits occurred outside CLI commands.  
* **Judge Rubric**: None required (100% deterministic)1.  
* **Tier Difficulty Knobs**:  
  * *Easy*: Prompt explicitly lists exact subcommands (dont flag then dont conclude).  
  * *Hard*: Prompt provides vague instruction ("Resolve claim CLM-1042 using epistemic tools"), requiring the agent to run dont \--help or inspect llms.txt.  
* **Improvement Signal on Failure**: Yields ERR\_STATE\_MACHINE\_VIOLATION if the agent attempts to run dont conclude directly without attaching evidence1.

### **Specification 2: Context Recovery & Managed Block Preservation (wai & AGENTS.md)**

* **Target Tool**: wai & genesis-vibes managed block infrastructure1.  
* **Capability Measured**: Correct context re-orientation following disruption and preservation of AGENTS.md managed blocks1.  
* **Task Setup**: Repository contains an existing AGENTS.md file with a populated genesis managed block1. An ambiguous build error is injected into the project environment1.  
* **Agent Prompt**: "The build pipeline is failing. Orient yourself in the project, identify why the last architecture decision was made, fix the issue, and record the update."  
* **Environment Initial State**:  
  * AGENTS.md contains managed tags \<\!-- GENESIS-MANAGED-START: context \--\> and \<\!-- GENESIS-MANAGED-END: context \--\>.  
  * wai status contains stored decision context regarding database locking.  
* **Deterministic Checks**:  
  1. Inspect trajectory to verify agent executed wai prime or wai status as its first or second action.  
  2. Assert that AGENTS.md retains structural block tags (\<\!-- GENESIS-MANAGED-START \--\> and \<\!-- GENESIS-MANAGED-END \--\>).  
  3. Confirm build succeeds via subprocess invocation.  
* **Judge Rubric**: None required (100% deterministic)1.  
* **Tier Difficulty Knobs**:  
  * *Easy*: AGENTS.md is present in root directory with explicit instructions.  
  * *Hard*: AGENTS.md is nested; noisy distractor files added to directory.  
* **Improvement Signal on Failure**: Yields ERR\_MANAGED\_BLOCK\_CORRUPTION if the agent overwrites managed tags, or ERR\_CONTEXT\_RECOVERY\_FAILURE if it skips wai prime1.

### **Specification 3: Contrived Error Recovery & Self-Healing Hint Adherence**

* **Target Tool**: genesis-vibes CLI tool suite1.  
* **Capability Measured**: Ingestion of ok: false JSON output envelopes and immediate utilization of the returned hints vector1.  
* **Task Setup**: A project configured with an invalid tool configuration key in .genesis/tools.toml1.  
* **Agent Prompt**: "Execute the project diagnostic check using the CLI tools and resolve any errors encountered."  
* **Environment Initial State**:  
  * .genesis/tools.toml contains unknown\_key \= "invalid".  
  * Executing any tool yields a JSON envelope with ok: false, an InvalidConfigKey error, and a hint reading: Run 'genesis doctor \--fix' to automatically resolve configuration schema errors.1.  
* **Deterministic Checks**:  
  1. Verify agent executed initial tool command and received ok: false.  
  2. Inspect subsequent command invocation; confirm agent executed genesis doctor \--fix within 1 step of receiving the hint envelope.  
  3. Assert post-fix envelope yields ok: true.  
* **Judge Rubric**: None required (100% deterministic)1.  
* **Tier Difficulty Knobs**:  
  * *Easy*: Tool emits clear hint string.  
  * *Hard*: Tool emits multi-option hints, requiring the agent to pick the appropriate option based on context.  
* **Improvement Signal on Failure**: Yields ERR\_ENVELOPE\_HINT\_BLINDNESS if the agent repeatedly attempts invalid commands instead of executing the hinted fix1.

### **Specification 4: Behavioral Verification & Test Selection (espectacular & testaruda)**

* **Target Tool**: espectacular (ah) & testaruda1.  
* **Capability Measured**: Efficient selection and execution of relevant tests following a localized code modification1.  
* **Task Setup**: Rust codebase with 50+ unit tests across multiple modules1. A bug is introduced into a specific utility function1.  
* **Agent Prompt**: "A bug was introduced in src/utils/parser.rs. Use testaruda to identify affected tests, fix the bug, and verify with espectacular."  
* **Environment Initial State**:  
  * Modified src/utils/parser.rs with broken string parsing logic.  
  * Full test suite takes 60 seconds to run; testaruda pinpointed test subset takes 2 seconds.  
* **Deterministic Checks**:  
  1. Inspect trajectory to verify agent invoked testaruda to obtain target test sub-list rather than blindly running full cargo test.  
  2. Verify agent executed espectacular (ah) to validate behavioral spec correspondence.  
  3. Confirm modified parser code passes targeted tests and returns ok: true envelope.  
* **Judge Rubric**: None required (100% deterministic)1.  
* **Tier Difficulty Knobs**:  
  * *Easy*: Prompt explicitly names testaruda and espectacular.  
  * *Hard*: Prompt states "Fix the parser bug efficiently without running unnecessary test suites."  
* **Improvement Signal on Failure**: Yields ERR\_TOOL\_DISCOVERY\_FAILURE if the agent defaults to full test suite execution, ignoring test selection tools1.

## **Phased Implementation Roadmap**

A phased deployment strategy ensures immediate value delivery through fast deterministic feedback while progressively expanding multi-model capability assessment1.

### **Phase 1: Core CI Harness and Contrived Failure Baseline**

* **Duration**: Weeks 1–31.  
* **Deliverables**: Implement Custom Rust Harness in workspace reusing genesis-vibes modules; author 5 core contrived-failure scenarios evaluating JSON envelope parsing and hint checks; integrate zero-cost regression gate in the tool PR pipeline1.  
* **Resource Allocation**: High developer focus on Rust harness integration and envelope schema validation1.

### **Phase 2: State-Machine and Protocol Suite Expansion**

* **Duration**: Weeks 4–61.  
* **Deliverables**: Construct evaluation specs for dont epistemic state machines and AGENTS.md managed blocks; implement the automated error taxonomy classification engine; deploy Inspect AI harness environment with Docker sandboxing1.  
* **Resource Allocation**: Balance between Rust crate integration and Python/Inspect AI solver setup1.

### **Phase 3: AIX A/B Infrastructure and Tier Calibration**

* **Duration**: Weeks 7–91.  
* **Deliverables**: Build automated AIX artifact ablation toggle (evaluating runs with and without llms.txt); implement synthetic flag and subcommand perturbation engine for contamination defense; calibrate difficulty knobs across 4 model capability tiers1.  
* **Resource Allocation**: Benchmark benchmarking focus; model parameter tuning and prompt engineering1.

### **Phase 4: Continuous Dashboarding and Multi-Model Matrix**

* **Duration**: Weeks 10–121.  
* **Deliverables**: Automate scheduled multi-model capability evaluation runs; publish error-mode diagnostic dashboards tracking tool AIX score deltas; integrate variance reporting (![][image9]) across model tiers1.  
* **Resource Allocation**: Maintenance, CI/CD pipeline automation, and diagnostic reporting1.

## **Open Research Questions**

While the outlined architecture addresses primary evaluation demands, several open research questions require empirical experimentation during implementation1:

> 1. **Optimal Hint Granularity**: What is the threshold where diagnostic hints in JSON envelopes transition from helpful guidance to over-specifying solutions? Empirical testing is needed to determine if detailed hints degrade model general reasoning1.  
> 2. **Context Window Saturation versus AIX Artifact Density**: Does placing comprehensive llms.txt and AGENTS.md files in project roots inadvertently pollute the context windows of smaller models (\<14B parameters), leading to degraded performance compared to minimalist prompts1?  
> 3. **Long-Term Epistemic Drift**: How effectively do agents maintain dont epistemic discipline across long-horizon trajectories (50+ steps) when intermediate tool calls yield warning envelopes1?  
> 4. **Perturbation Robustness Thresholds**: How much flag and subcommand perturbation can an agent tolerate before its zero-shot command generation capabilities degrade entirely, and does llms.txt presence fully offset syntax perturbations1?

## **Strategic Summary**

Building an evaluation suite for the charly-vibes CLI tool family requires a paradigm shift from traditional end-task code generation benchmarks to structural protocol evaluation1. By leveraging machine-readable signals—JSON envelopes, epistemic state machine logs, and managed block boundaries—the framework delivers deterministic, zero-hallucination evaluation signals1. Implementing a hybrid harness model (Custom Rust for CI dogfooding; Inspect AI for multi-model research) guarantees both rapid developer feedback and rigorous frontier model assessment, ensuring that the tool suite's investment in AIX yields measurable improvements in autonomous agent operation1.

#### **Works cited**

> 1. cli-agent-evals-prompt.md  
> 2. AI Coding Benchmarks Explained: SWE-bench, LiveBench, and More, [https://www.openhands.dev/blog/ai-coding-benchmarks-explained](https://www.openhands.dev/blog/ai-coding-benchmarks-explained)  
> 3. SWE-bench: Can Language Models Resolve Real-world Github, [https://github.com/swe-bench/SWE-bench](https://github.com/swe-bench/SWE-bench)  
> 4. SWE-bench \- GitHub, [https://github.com/swe-bench](https://github.com/swe-bench)  
> 5. Terminal-Bench (harbor-framework/terminal-bench) \- Context7, [https://context7.com/harbor-framework/terminal-bench](https://context7.com/harbor-framework/terminal-bench)  
> 6. open-operator/benchmarks/osworld.md at main \- GitHub, [https://github.com/OpenHands/open-operator/blob/main/benchmarks/osworld.md](https://github.com/OpenHands/open-operator/blob/main/benchmarks/osworld.md)  
> 7. InterCode: Standardizing and Benchmarking Interactive Coding with, [https://huggingface.co/papers/2306.14898](https://huggingface.co/papers/2306.14898)  
> 8. ProgramBench: Can Language Models Rebuild Programs From, [https://www.alphaxiv.org/abs/2605.03546](https://www.alphaxiv.org/abs/2605.03546)  
> 9. Aider LLM Leaderboards, [https://aider.chat/docs/leaderboards/](https://aider.chat/docs/leaderboards/)  
> 10. Terminal-Bench 2.1 \- Harbor Hub, [https://hub.harborframework.com/datasets/terminal-bench/terminal-bench-2-1/6](https://hub.harborframework.com/datasets/terminal-bench/terminal-bench-2-1/6)  
> 11. GitHub \- microsoft/SWE-bench-Live, [https://github.com/microsoft/swe-bench-live](https://github.com/microsoft/swe-bench-live)  
> 12. SWE-bench Verified, [https://www.swebench.com/verified.html](https://www.swebench.com/verified.html)  
> 13. checklist.md \- SWE-bench/experiments \- GitHub, [https://github.com/swe-bench/experiments/blob/main/checklist.md](https://github.com/swe-bench/experiments/blob/main/checklist.md)  
> 14. GitHub \- SWE-bench/SWE-smith: \[NeurIPS 2025 D\&B Spotlight, [https://github.com/SWE-bench/SWE-smith](https://github.com/SWE-bench/SWE-smith)  
> 15. OSWorld: Benchmarking Multimodal Agents for Open-Ended Tasks, [https://osworld-v1.xlang.ai/](https://osworld-v1.xlang.ai/)  
> 16. OSWORLD 2.0: Benchmarking Computer Use Agents on Long, [https://s46486.pcdn.co/wp-content/uploads/2022/01/OSWorld2.0.pdf](https://s46486.pcdn.co/wp-content/uploads/2022/01/OSWorld2.0.pdf)  
> 17. OSWorld: Benchmarking Multimodal Agents for Open-Ended Tasks, [https://openreview.net/forum?id=tN61DTr4Ed\&referrer=%5Bthe%20profile%20of%20Tao%20Yu%5D(%2Fprofile%3Fid%3D\~Tao\_Yu5)](https://openreview.net/forum?id=tN61DTr4Ed&referrer=%5Bthe+profile+of+Tao+Yu%5D\(/profile?id%3D~Tao_Yu5\))  
> 18. InterCode: Standardizing and Benchmarking Interactive Coding with, [https://sophon.at/papers/intercode-standardizing-and-benchmarking-interactive-coding-with-execution](https://sophon.at/papers/intercode-standardizing-and-benchmarking-interactive-coding-with-execution)  
> 19. InterCode: Standardizing and Benchmarking Interactive Coding with, [https://www.researchgate.net/publication/371908989\_InterCode\_Standardizing\_and\_Benchmarking\_Interactive\_Coding\_with\_Execution\_Feedback](https://www.researchgate.net/publication/371908989_InterCode_Standardizing_and_Benchmarking_Interactive_Coding_with_Execution_Feedback)  
> 20. InterCode: Generating code interactively with Reinforcement, [https://medium.com/@bhattacharyyakiran12/intercode-generating-code-interactively-with-reinforcement-learning-and-large-language-models-5becded387f3](https://medium.com/@bhattacharyyakiran12/intercode-generating-code-interactively-with-reinforcement-learning-and-large-language-models-5becded387f3)  
> 21. Refactoring leaderboard \- Aider, [https://aider.chat/docs/leaderboards/refactor.html](https://aider.chat/docs/leaderboards/refactor.html)  
> 22. RefactorBench: Evaluating Stateful Reasoning in Language Agents, [https://arxiv.org/html/2503.07832v1](https://arxiv.org/html/2503.07832v1)  
> 23. GitHub \- neulab/SWE-Playground: Official Repository for "Training, [https://github.com/neulab/SWE-Playground](https://github.com/neulab/SWE-Playground)  
> 24. ProgramBench: Can Language Models Rebuild Programs From, [https://www.researchgate.net/publication/404476868\_ProgramBench\_Can\_Language\_Models\_Rebuild\_Programs\_From\_Scratch](https://www.researchgate.net/publication/404476868_ProgramBench_Can_Language_Models_Rebuild_Programs_From_Scratch)  
> 25. ProgramBench: Can Language Models Rebuild Programs ... \- arXiv, [https://arxiv.org/html/2605.03546v1](https://arxiv.org/html/2605.03546v1)  
> 26. Inspect AI: AI Evaluation Tool — 2.1K+ Stars, Free & Open-Source, [https://aisecurityandsafety.org/en/tools/inspect-ai/](https://aisecurityandsafety.org/en/tools/inspect-ai/)  
> 27. Inspect AI Review 2026: UK AISI's LLM Eval Framework \- AI Evals, [https://www.aievals.co/tools/inspect-ai](https://www.aievals.co/tools/inspect-ai)  
> 28. Can Language Models Resolve Real-world Github Issues, [https://www.swebench.com/original.html](https://www.swebench.com/original.html)  
> 29. harbor-framework/harbor: Framework for evaluating and ... \- GitHub, [https://github.com/harbor-framework/harbor](https://github.com/harbor-framework/harbor)  
> 30. harbor-framework/terminal-bench: Measuring and evolving ... \- GitHub, [https://github.com/harbor-framework/terminal-bench](https://github.com/harbor-framework/terminal-bench)  
> 31. OSWorld 2.0: Benchmarking Computer Use Agents on ... \- GitHub, [https://github.com/xlang-ai/OSWorld-V2](https://github.com/xlang-ai/OSWorld-V2)  
> 32. Aider Review 2026 \- Coding \- TechVernia, [https://techvernia.com/pages/reviews/coding/aider.html](https://techvernia.com/pages/reviews/coding/aider.html)  
> 33. Aider Review: Capability Coverage, Pricing & Alternatives, [https://agenticindex.io/vendors/aider](https://agenticindex.io/vendors/aider)  
> 34. UKGovernmentBEIS/inspect\_evals: Collection of evals for Inspect AI, [https://github.com/UKGovernmentBEIS/inspect\_evals](https://github.com/UKGovernmentBEIS/inspect_evals)  
> 35. OpenEnv/docs/source/tutorials/evaluation-inspect.md at main \- GitHub, [https://github.com/huggingface/openenv/blob/main/docs/source/tutorials/evaluation-inspect.md](https://github.com/huggingface/openenv/blob/main/docs/source/tutorials/evaluation-inspect.md)  
> 36. Promptfoo: LLM evals & red teaming \- GitHub, [https://github.com/promptfoo/promptfoo](https://github.com/promptfoo/promptfoo)  
> 37. Remote evals and sandboxes \- Braintrust, [https://www.braintrust.dev/docs/evaluate/remote-evals](https://www.braintrust.dev/docs/evaluate/remote-evals)  
> 38. LangSmith Evaluation \- Docs by LangChain, [https://docs.langchain.com/langsmith/evaluation](https://docs.langchain.com/langsmith/evaluation)  
> 39. Evaluate Your SageMaker Endpoint with Inspect AI \- GitHub, [https://github.com/aws-samples/amazon-nova-samples/blob/main/customization/sagemaker-inspect-ai/inspect\_eval\_container/eval\_sagemaker\_endpoint.ipynb](https://github.com/aws-samples/amazon-nova-samples/blob/main/customization/sagemaker-inspect-ai/inspect_eval_container/eval_sagemaker_endpoint.ipynb)  
> 40. Testing Prompts with GitHub Actions \- Promptfoo, [https://www.promptfoo.dev/docs/integrations/github-action/](https://www.promptfoo.dev/docs/integrations/github-action/)  
> 41. promptfoo \- GitHub, [https://github.com/promptfoo](https://github.com/promptfoo)  
> 42. 2.3.28 Satellite: Promptfoo · av/harbor Wiki \- GitHub, [https://github.com/av/harbor/wiki/2.3.28-Satellite:-Promptfoo](https://github.com/av/harbor/wiki/2.3.28-Satellite:-Promptfoo)  
> 43. How to use Braintrust with any framework or provider \- Blog, [https://www.braintrust.dev/blog/any-framework-any-provider](https://www.braintrust.dev/blog/any-framework-any-provider)  
> 44. Braintrust TypeScript SDK, [https://www.braintrust.dev/docs/sdks/typescript/versions/3.27.0](https://www.braintrust.dev/docs/sdks/typescript/versions/3.27.0)  
> 45. Glossary \- Braintrust, [https://www.braintrust.dev/docs/reference/glossary](https://www.braintrust.dev/docs/reference/glossary)

[image1]: <data:image/png;base64,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>

[image2]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAEoAAAAaCAYAAAAQXsqGAAAC2ElEQVR4XtWWT6hPQRTHz33yvyQLrydZWCk28haKks0LG1JSVrKQlexEHqVIbNi8DQuRnciWlGQhC9kJS1n7F5L0nPObM7/fmXNn5jf3zr33XZ/69pt7zpkz5547d34XwEOhDRbt0Nc+UmJaw7+43+qnQmw8NO4NMJhUa2Y35JWWNztKg6mDqYKOTglWcRI1p419gau+jjriOLpj2Lh5luYV+O1d8RL1B0b1nXbd/SHUwK7ZD6aOM9rRHZ63UZiouM+jywUjq1F7UY9RN1BLlG8L3u5l/L0mbDdRJ8CN3Yh6gFojbPtQs6iLYIp7ytfd4HlwEGmUP5wo4AOYSSuFdZ4cPOkYahfbFxsffONr+Sq9QVEz17Nthu2WW2B20yJl97EJTDNnsQjzO15VsI0qnVGhRq0CM+GFsstz5KsYE+R7jtrA499s/wRmdx1g+w62W96hHipbiIOoJxUlCN3ukOCOCvEL9Rq1ArUHdQ/MblktYujpWo6CWWCrsFmm+Pcn6pJ0MDRvmzZWYeztp2MbdVY7QlDwbscSr+Y96r42KuRulITsgsjiEVcNbKPOaUcJXpeC1zqOOBR/ShsF0+BvCB3yPnsIitdnUFTJfTSB6Y1iKLjwPK4rYkwxUxxD4wnh+47azOPlqB9g/vUIeo0tt1FfxDW98o0yqK50GyWDxTbqvHaEoIN4WiWkQ/ojj9eBSbgUNcljC/173RHXdJaR/1BhcvwVPjrEn/F4ApeTnxkegjfYFLZRF7QjBj1d2gm0O+g7aqfrHnzyk+8tajuYBehT4K4MYg6DOceugrvzloGZR7vqkbDXIqONVMNQxWh83InKJqPCJulJGWkEiw068klPnR4Zp6k8Y+hoGS/V164+Q5GdoAbFwizbf9rqSlt5m+R/qLEv5PfKn8FvzSCWMOgLOqpTP1X9mSUaTOXSWmJoN3cFOikjeZHkQKJScJRYJtcnrvzDAf8AYwONx7CQH2EAAAAASUVORK5CYII=>

[image3]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAF4AAAAaCAYAAAA+G+sUAAADjklEQVR4Xu2XS6hOURTH1/FKCnEnpBRKXiPyiis3JQZGlKEBMkAyUoqBUFKMjLxC5FKYepVEXokkoWRCKWIgSbpd63/23vfbe519znf2/s49V/e7v1p9Z//3Ovuxzjprn49oiIpJvJdtRvvufJARmc0hvhWBKU9I8T/jBtusRjMTpS62Q2x7ZIeHnU4rM1R9XGDrlaImT6+Dt6TmN7bQ7e7jK9t4XOgY9rC9bnSn3CN3rH4k8yQzgiZPp61UdpHpEEnBUJGo8Y6TWsdip0+xnrJr3KG1EUIHHZT1b4Wqd5xylu2bLfTLLM0pCvxFtu9CW0fKv1PoQAS+1I4cp7Vst0gtSj7Z+WxH2I5a2im2bWwjLW0G2zW2vZYG9rEdILXAO7q9yfEIoNTWiskL/DitvxBzYP/Qr7hySkcSmfEfSN04xtLsgbZQ40mPIlXvfuo2/IzvK7aDbFO1tkrrhtNaHy50Hzj08HBCLIRjbL1JtsZzO8EaHwl9Hqm1PxM6smACRQTePOH7QrcH+tV3pdIAfTfZZutr8xA+sQMezIZUTzLZ9C7Vy4E6ezvQil8Fty8v47u0/kDoZq/vhQ6iavwftueksn0N2yVSgRxr+VifXGn2YxJkgGSy/v1N3oWkmeTRBwQT+KVCn6P1x0Kfq/WHQgdRgccNK6VYAMrSZSkKfAsH0FH/K6IovZtiAr9M6DjbUEpRNu0p8GbAH+eaJDrwHQGbgP/2RjNz3yJSPruEjkMX+m6h5zGdsjVcW+LRgmt8XuABKsBn09A7XE3+fYHowGeiR+7XC3wmWde2P+r/TH2NcoX2Vd3u1r+44Rz//GAbpiWUI7uE1Y0JfKfsYJZTNpD7PZohIPCN0KHGLxCxn8b2UV9PITUoDk3UcHsCvJYnrbY5gHAwAryyBvw9v6uv8VVz2OobCEzgV8gOUsGQgcTHh9QMAYF3QfYhU3Go4jt+idtNf0n1vyGVIT28tJeksliykdTXyxdqZDcYrQ/XJ2zXLb1usAafobTZbCa1725+Ck/59zzbRNclTSw5DuyM7dQCbiXy1aVWKB6vuDef2PsiqHGqwcdQ8EJoNVqt3t9GlAlVGZ86aWE9RbcW9WlKuDSjeIii3qK+MKobyYMZvF8nCR090N1PJYNYVD1eBLlLyO0wNHWoH0/muause83B86kbchcdPF5zQoYM8W1rYgIVc8+AUXqxpR0HNzFh+Aenbs46zvfa2wAAAABJRU5ErkJggg==>

[image4]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAADgAAAAaCAYAAADi4p8jAAACLklEQVR4XtWWP0scQRTA30oIIgpRVBSxDBixFq0UxVIERfwGKdMGhIN8gwSSyuY6v4SdhYgogoVgZ6OIFgqSIoqY93b+3Mzb2dmd3fG8+8Fj7/2dN7MzswdQmoQbugmj+SjziFJEELEUyGqLhnxp+TRzYMdMGb7Ow7FAr4Zc2i7oQXkEO2bPimgPP1G+okxLfVDq/TqiAGq8KZ8uqNgYN5bFsaihnINe4EQt9A8rwo0e+pd8UuIfZRSkMWeWVovwCokYfwZlAWWcuQvpRdmQv2mCDygDLbe2vykF0z7lhsIMg4bx+xuIyRwbNuKI6S3Kj1MHxwTL85fpao8r6ECvGroPuoVpwUJkIs30Qwt8DeIuGMJFfcbnvR2SD99+aoKTUt+EkrdVIrY63bIhMpsm+zkAO+43iB4/GbZcdpi+DiL5Rur6gukgRkH0eMsdnCVwX//fQRTYwvfywp3vwC43IHeQ3X0ZtrlBMgwi+RBln/kge7No/TNKI8mes1zB2BFK5BUN1NtaZvZ/0p4L7V8KykOdxRXuyOJprwayKv2bukD5YLjmgXpLMsfLogn+FdjHAk/47PP27/NVwV3vhOnqknEdrxT1dpS4+AhiG4XhbjBDyTAF/XuhzwLdulcgPhlrVkSGwBFsaiU7iV/RQVsGCSBaP3ahaGVrE6GTCCW6n4iLkJZK6pfk+Vx/KwrHKQx4b3SDvFOudwztaazSKJWSgqg+gj/T79WUDOtqqszxP0OzapJTAP7BAAAAAElFTkSuQmCC>

[image5]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAEMAAAAaCAYAAADsS+FMAAADq0lEQVR4Xu2WW4hOURTH1+eS+yURyoMQkxIP5PowzItEUhRC8kJJJJHcck0RT0h58CJR8oASDy4RRcg1l6aJRJRryEis/9lrf2edffae+b5vzjffTPya/3x7r7X2Pnuvsy+HKEEu+nOLQRoKaMhXcVr04FoRlc9jNIKZrPusP6zbrKmstiooa8oz7Qx6ncF6zBrJ6soax3rDeqSDAnRizXaNlaX0jCAJQ1yj4q2nbyTvF5lVBK1MuhXpth4KCiqOErp8kS/l6CD//8xaw3rJ6pX3Ed1htUk9wNSRjFVJR5lxx5ERN+QXZwMm1V/qHcismAFSr2PNl7JLscnIIcvHWcvEgMbnWPvyIUkQd5q1VOrDlc9lM2s/a6HrEDqy9rK2kTkULT15XL0Jb5zoOeu68oG1ZCY6mFXN+prwxhSbDHpIZkJo+I7VQ+zrWbcofvFjCXvUPBzAbvely13Weyn3ZV1kTYvdEZjANSnjUHxKURIilsjvBDL927rFJmOo1OsDy5NjcuEzg9KrejVrMpnOTyn7eLHVSP2T1PPkwsnAIE5KGSsJMdWxOwK2FVI+hnrObAGwUX6nwM6aJ3WLTUaV1F8rnwYxBa8MXFXgLKUntUhs23lieDMoYzlrYMNqcrFJOsTqph3qTdgY9NkuNkdgRcCG8GeU3rJuMr4pnwYxeNmUXgMGn9UOTHNZbKNZ38msDN12oPiXK5sFB9orFt42Yn4m3VE3u1k/KH72Ex3BnJDf7mT8+tm/xTaGNYd1T/k0iMEN5MeXCYoa5dxkoI7lZ88G7HvNYrGPcOwWtMN+9SXa0oWjtpI/pl6Va1lHWVXc6S4yq+kLaweZw3VDHJqg4WR4aE+m0ZG8xWQMCcIXnCnnEnuvD5nD1J4Lds/axI2SOsA5JBONOh4W1/N89Ng6sw5T0z670Se2VMFMItPoirLhwMJkLRdYO1X9DJk2W6Rubw6cQbDrBYiJ1hpD9H86pSeOut0WGtx0l1wjUA/YJPKBftcFdoOX82S+6nDP40rE/t6TiDDcJHN2YJ/j+wB3PJYyrl98D1hQ/8C6Kv4az97EoVtH5gVgwIO0Mx1OB8iM6wGZ7Yo2KM/SQUI1Gb9PjYItMNc1GtSwPCOMCNnLTFMfG2qPjPVLWEKRrRo7qfDk8JWJZCxgTXR8/3EJ57GStMxRKco4wDJ2nSajh2XUTYynQ4+pJLLqp/nJeuRZ9xem+Z5UNMGhBR3/Bo1MvxF3wZTYTyHNgjFBR8H8BVCwyfusAIH7AAAAAElFTkSuQmCC>

[image6]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAD8AAAAaCAYAAAAAPoRaAAADuklEQVR4Xu2XWYiNYRjHn89Wkl2UyJas5QJFyXYhSyihLGVXJOXCTkZJSFzNBc0VkaShRMruxoWyZinS3CiMLAkhjef/vc97zvM9835nzpk5Z8ZMfvXvvM/z7tvzfofIEllHy6FFTK1ek8hRqQ9rHWs/a4fJK4BwD2FvToaxKllfWK9YO1mDEiWSdGAtZ5WxziRyLIHBHGfViPaZvMamPesXaxZrALlJY0K/WZ1UOc1iyo7/VjILBGZsOESucloHtam7zTT8QC3VkRtHGkdYA1O6ncPjQZuzbUY+YGVDAyoF6Oei8a1mtZX0SHJl7rEqeLKbM6WInrO2Kdtznwoaf3IJ03aj2PSmuJ9ou3byUN4r8y25E+BH+I51VNIzWR8krWnQ+FHxLmsP6wqFVxcMZ1VGLgiNspnCRNY51gpWZ/HtYu1m3SDX116xPeXyu4DCk8COX5N0hc4QUAexwjOYdZ7VTvmCzKDsyrUS33XWgUwJIuzUd9ZcsbuxPvtM2aLW5CL0EHFfYNWYO4o+tiZdMYvk9yypdhUvKBvMlig/kNMUv1YA16Uv6wfrsC+UxkEJFuuVD7vyVdnIn6ds8MjY2LVLyk4cRVkE2OO9TzFNflG/Svk9zyg7eWyWBouBduHHAnQVP3xrfKE0EOxslD1NMvDIRVmke7HGkXsO37BWZkrHRDjK/vjNT+ZlCB1pUCa/qBcqg2N/R9K2bSzMRx7oa+PPC3TWQ9m4p39YT1kjJP+myjckDnYV237H0cZUlbdU/CHUfY3rT8jadIpcvQdi25cCVwsxaCPS5PqpgyhxFDW41/Bh9ydLujwzR3OJDcjFsX5Mrp7+4kKg+qZsjQ94YAthJ4mWkbtKT1jHyMWcKfKrQT8dJX2Z3HVFTMoLO3k07n1tyO1K4mkSxqo0PkOz7bgFgr1WPPhSgy1vdlwAL4IGEx4Tp3IvsCLqKfHKg/Zh45O9H2uoylNkO7CTh60DFz5/7VHbQC6aAnxbo87VbHa8E3i7u4s9mlyZ6WJPYvWXtGcV6yflnjrijH4ibYzAE+2/AzYpfyp4C1EBb7CfkAVv7EvWQ3LBx34GdyE3iNuEZymKT4IF3+ufWNWshSZPgwiNeIGTgAXFRw5OI/504SRqTrJOGB/GgTHmcff/OXJtfBFJdpNPp/mUyZOodmvWzlB4Rh00dr2iogYRTirC3mZB+HSGvf/J0FRL0lT9xpSi81K02RByjCdHVjOk1mxqOZoKOxBrF5fCWy+8hiGqdxu+WrJ6qLGQr3T8BYsbyUm+DnfLAAAAAElFTkSuQmCC>

[image7]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABUAAAAZCAYAAADe1WXtAAABP0lEQVR4XrWTLU4EQRCFqx3BIDF4HIoEJEEgEKtQHACPwRHCQdDcATwJCZ5wgHWEhBMsr7ZrZuqvk2YWvqTS3a9eVU9PzxApil4ImSYcI3a8OJctxInMrxEfU2o+D4g3dYaVylX08VpHjbpRYlNDrBY4MYTWaB/xrcRfEje8QbxbKZq6kLJLjNsixS9g6K32OMDqtgbpWEwWehpnhc5ljEC7w7DCuMT4yXNZP1ONK7E+DrkxYCpJ01OqhotBKGtnvNmktgkXH3mRks+lq6mYuNj4S/1zQtM17PRuByumWCz87l4npaqxnMlV+0SFXoLWKGzJrH9hOBTDGdWG9yo/FsceUdHswoDLKntGTWtScRN8Q7/2NPOcaCYtnbY/9zXpfPZqCUYnhPwsQpd/2cWyWU9d3flCFZk7OfIoubvA5AcVSieAikqNPQAAAABJRU5ErkJggg==>

[image8]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABUAAAAYCAYAAAAVibZIAAAA9ElEQVR4XqWTuw3CQAyG7xokRMcU7ICQUlOxAYswFDUSu1BRMgAVThxf7LPPZ+CTTI7/YVAgKRG5nHyiOaRKB8uNWEMOkKkbXRHNcZyOtoqiLZvfC5pMnpPxeI8vuWz5Frt3xYttLvR8DmbvMCtpcMyFpsjZwTwgtq0N4gAzwC0axitNZmf4EOHNc4Z5wmySwPpCllYQ5h7mwoWZMaS3lKdlEVhsOrxgjqRIun8L0z/BrGuxYFaSrU8aGjdh1PCyXqSVCfxB+8jb1cHJOFZtsndmC0XTIlyTEw52+G9Po61vSzhoQfrit5Iu7ZJ63hBTrAmFNB/VuQvoz4HOwwAAAABJRU5ErkJggg==>

[image9]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAEQAAAAaCAYAAAAOl/o1AAAELUlEQVR4XtWXW6hVVRSGx8okMyVKH8oHEww0syhICKOiLCgspMAUIkVRqIeoVAyh6CGiQhC7SRJF2Q1fLLwk0UOHKEyje4lB+WDaDYkukJQPp//fY8y9x5xr7r3WOu3tyQ/+s9a4zLnnfa4j0o0idfSZuvW38+oWKFO0yo68fGNO4E/lGfUGDIiKfl2JjCE8j0NfQSugM6OMmNnQGmgjdEMS64VrRkWL+k2Dn5sOHYWugM6BLoI+hv7wSQkHoGHT1Unsv9Gg4f0h3vJ/Qbe0rTJD0Kmp0+CgcUBqkOul9+XiJ56X3fsStImdex56D5rrYv9AFzo7EFZIxGC7NtjauToC6FjhzwLGbrb3Z6CtLhbgYPDMacxZ0OvQXWbfA+2EnmxnxKyE3oTuNvsMF/NcIFrXC9B5SSxwDfQA9AS0NIktsuezoqvAM1O0w/dBY6Cf6XTzM8Xij6hZnIY/t0Ob6kzi19AM0Vn4Bc+zzX+vxU4xew70I3S9+2n+6BvBcLCDC53NvKedTfZB59v7ZGiXdAZhmugBSn6HXrP3QBiQtWa/6mJks3AQi9Zk8RDmZJ8umS1UotDrCddaK3mHC11svgVm/2q2h/a6xEd8Hm8J2o8Hhw2nz1lu9nyzeaNwVjn7XPY8OzzpgHDbePZD79r7J/a8VcrtLzHBmsctkCbfZj7e5cvs/dEoQ325U57+MMDT4lAbxv8Wnc30m4KrktuJsLP89vCEAbnf7JdcjDD2OfRF4i/RbQeFDnh2m+9a0Ybz3ueMBbik0zIBLtNQJ7UqDrdYLHFOek784Bo7jJZP6pjyTcsn8jA0UbR9HsZYPyeT7+PicDW5AeFS/U06e28oiup+T8t4eBWGnG/pyMwGtwYP5+GiXNeX+miV2g7theaJHtSvQB9C74uuWn5zBDBRha+L7w86uxIueRQqtiR+VtQ6YK2xq12MN9Mh0YOQcMbIi6K57RWBsneK3jSB70RzLnM+HorpgJBPIb8y6sDD268YrrwN9s6BrORy0cbs4VzYLN4k+rkceBt6zNnbRMushy6F/rRyH5j/3JAouvX8hxTjlF8wuL2KI84WC3Ogvo9dJXiw+hXAc2PIJXOl81OBcMXHZOp8CzoMXSc6I6zgqShD4fLkBxFHfzw0VTSX1+5Yy2H1z0HHoM9Er8z0FuLq+kj01uL/HKzvxiijzEOig3gQeke0HT+Jrj7PLNE8/+V6CVrFfG7bWmcJtws/WnqQGcZA0TPaIhvPOvtDt6pjv1mZZI5o+AiqSaYWo3skoXZid2pVUSupA5czB+QO6KokNno07MRJQNqj1P4f0LNJPYOjSadhzZrYLDvOryxbmVCLZrVkBqJZBSczI+3pSMtVMrCK69HHn29cVeMCEbnSOZ+RCWVc0s1bl8rS3RK6+ZXe0ebk6sv5EkopJcdA+RdiaNYTKpGjhQAAAABJRU5ErkJggg==>
