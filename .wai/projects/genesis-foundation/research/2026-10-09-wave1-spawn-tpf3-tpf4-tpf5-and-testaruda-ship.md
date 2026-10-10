---
tags: [epic:genesis-tpf, wave:1, orchestration]
---

## 2026-10-09 ~20:50 — genesis-tpf wave-1 orchestration (renew session)

- Verified NO parallel agents in donor repos (live pi sessions were in specodelic/gently/wild only).
- genesis-side claims: genesis-tpf.3/.4/.5 (this bd). Twins (espectacular-o77, pretender-4hc, dont-9gry) claimed by subagents.
- SHIPPED stranded tpf.2 work: testaruda feat/genesis-git-migration rebased onto origin/main (d6c4265), gates green (331 passed, clippy/fmt clean, genesis-vibes 0.13.0), branch pushed. Plus root-caused + fixed a pre-push hook env leak: git exports GIT_DIR when hooks run from a LINKED WORKTREE (not from main checkouts — why prior sessions never saw it); lefthook passes it through; fixture `git init` then honors GIT_DIR and creates no .git in the tmp dir → "not inside a git repository". Fix: tests/common/mod.rs strips GIT_DIR/GIT_WORK_TREE/GIT_INDEX_FILE/GIT_COMMON_DIR/GIT_OBJECT_DIRECTORY/GIT_ALTERNATE_OBJECT_DIRECTORIES/GIT_PREFIX/GIT_CONFIG_PARAMETERS/GIT_QUARANTINE_PATH at binary init (.init_array, pre-threads, race-free) in the 5 integration test binaries that spawn git or the tool (cli, cli_exit_codes, adapter_python, adapter_rust, dotnet_adapter). Verified: full suite green with GIT_DIR+GIT_WORK_TREE explicitly set.
- SPAWNED wave-1 subagents (nohup, timeout 1800 = 30min hard cap, model openrouter/z-ai/glm-5.3-flash, per-repo dedicated worktrees on feat/genesis-git-migration):
  - tpf.3 pid=94535 → espectacular-git-epic, brief genesis-tpf.3-migrate-espectacular-changed-files.md
  - tpf.4 pid=94538 → pretender-git-epic, brief genesis-tpf.4-migrate-pretender-git-rs.md
  - tpf.5 pid=94540 → dont-git-epic (new worktree), brief genesis-tpf.5-migrate-dont-git-plumbing.md
- Watcher: /var/tmp/genesis-tpf-wave1/watch.sh (pid 252445), polls every 1200s → /var/tmp/genesis-tpf-wave1/status.log
- Wave-2 queue (after wave-1 verify): tpf.8 (wai close.rs, worktree), tpf.6 (whisper), tpf.7 (dulce) — all P3 except tpf.8 P2.
- Ship convention for wave tickets: subagent commits RED+GREEN locally on feat/genesis-git-migration; LEAD verifies gates, rebases, pushes branch, closes genesis+ twin tickets. Epic merge-to-main deferred to epic end (testaruda branch pushed but not merged — same for the wave repos).
