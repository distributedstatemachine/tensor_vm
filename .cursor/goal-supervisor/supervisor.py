#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = ["cursor-sdk"]
# ///
"""Run goal.md as a persistent, restartable Cursor agent task."""

from __future__ import annotations

import argparse
import contextlib
import dataclasses
import datetime as dt
import fcntl
import json
import os
import signal
import subprocess
import sys
import time
import traceback
from pathlib import Path
from typing import Any


DEFAULT_REPO = Path(__file__).resolve().parents[2]
DEFAULT_STATE_DIR = DEFAULT_REPO / ".cursor" / "goal-supervisor"
DEFAULT_MODEL = "gpt-5.5-medium"
SUPERVISOR_VERSION = 1


class SupervisorError(Exception):
    """Expected supervisor stop condition."""


class Blocked(SupervisorError):
    """The goal cannot safely continue without operator input."""


@dataclasses.dataclass
class SupervisorState:
    supervisor_version: int = SUPERVISOR_VERSION
    agent_id: str | None = None
    last_run_id: str | None = None
    phase: str = "BOOT"
    active_iteration: str | None = None
    last_commit: str | None = None
    blocked: bool = False
    blocker: str | None = None
    consecutive_failures: int = 0
    updated_at: str | None = None

    @classmethod
    def load(cls, path: Path) -> "SupervisorState":
        if not path.exists():
            return cls()
        raw = json.loads(path.read_text())
        allowed = {field.name for field in dataclasses.fields(cls)}
        return cls(**{key: value for key, value in raw.items() if key in allowed})

    def save(self, path: Path) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        self.updated_at = now_iso()
        tmp = path.with_suffix(".tmp")
        tmp.write_text(json.dumps(dataclasses.asdict(self), indent=2, sort_keys=True) + "\n")
        tmp.replace(path)


@dataclasses.dataclass(frozen=True)
class GitSnapshot:
    branch: str
    head: str
    status: str
    upstream: str | None
    ahead_behind: str | None


def now_iso() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def log(message: str) -> None:
    print(f"[{now_iso()}] {message}", flush=True)


def run(repo: Path, args: list[str], *, check: bool = True) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        args,
        cwd=repo,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    if check and result.returncode != 0:
        raise SupervisorError(f"{shell_join(args)} failed with exit {result.returncode}:\n{result.stdout}")
    return result


def shell_join(args: list[str]) -> str:
    return " ".join(subprocess.list2cmdline([arg]) for arg in args)


def git_snapshot(repo: Path) -> GitSnapshot:
    branch = run(repo, ["git", "branch", "--show-current"]).stdout.strip() or "DETACHED"
    head = run(repo, ["git", "rev-parse", "HEAD"]).stdout.strip()
    status = run(repo, ["git", "status", "--porcelain=v1"]).stdout

    upstream_result = run(repo, ["git", "rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"], check=False)
    upstream = upstream_result.stdout.strip() if upstream_result.returncode == 0 else None

    ahead_behind = None
    if upstream:
        ahead_behind = run(repo, ["git", "rev-list", "--left-right", "--count", f"{upstream}...HEAD"]).stdout.strip()

    return GitSnapshot(branch=branch, head=head, status=status, upstream=upstream, ahead_behind=ahead_behind)


def ensure_required_files(repo: Path, goal_file: Path) -> None:
    required = [
        goal_file,
        repo / "docs" / "tensorvm" / "upow.md",
        repo / "docs" / "tensorvm" / "mvp_spec.md",
        repo / "docs" / "tensorvm" / "local_chain_production_readiness.md",
        repo / "docs" / "tensorvm" / "local_chain_production_exec_plan.md",
    ]
    missing = [str(path) for path in required if not path.exists()]
    if missing:
        raise Blocked("required files are missing:\n" + "\n".join(missing))


def acquire_lock(lock_path: Path) -> Any:
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    lock_file = lock_path.open("w")
    try:
        fcntl.flock(lock_file.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError as exc:
        raise Blocked(f"another goal supervisor owns {lock_path}") from exc
    lock_file.write(f"{os.getpid()}\n")
    lock_file.flush()
    return lock_file


def build_iteration_prompt(repo: Path, goal_file: Path, state: SupervisorState, snapshot: GitSnapshot) -> str:
    dirty_notice = "clean"
    if snapshot.status.strip():
        dirty_notice = (
            "dirty; inspect `git status --short` and `git diff` before edits. "
            "Treat existing changes as user or previous-run work; do not revert unrelated changes."
        )

    return f"""
Run one complete feature-sized iteration of the TensorVM goal contract.

Repository: {repo}
Goal contract: {goal_file.relative_to(repo)}
Current supervisor phase: {state.phase}
Previous agent id: {state.agent_id or "none"}
Previous run id: {state.last_run_id or "none"}
Previous blocker: {state.blocker or "none"}
Git branch: {snapshot.branch}
Git head: {snapshot.head}
Git upstream: {snapshot.upstream or "none"}
Git ahead/behind: {snapshot.ahead_behind or "unknown"}
Git working tree: {dirty_notice}

Non-negotiable execution contract:
- Read `goal.md`, `docs/tensorvm/upow.md`, `docs/tensorvm/mvp_spec.md`,
  `docs/tensorvm/local_chain_production_readiness.md`, and
  `docs/tensorvm/local_chain_production_exec_plan.md` before editing.
- Maintain `docs/tensorvm/local_chain_production_exec_plan.md` as durable state.
- Select exactly one coherent feature-sized iteration from the v0 gaps/readiness matrix.
- Write the full checkpoint required by `goal.md` into the exec plan before edits.
- Use read-only exploration/subagents where helpful before implementation.
- Keep the parent/integrator as the only committer and pusher.
- Do not preserve v1 TensorWork proposer behavior, legacy block formats, compatibility aliases,
  or adapter-owned consensus shortcuts.
- Do not overwrite or revert unrelated dirty files.
- Implement production code, focused tests, and checker/docs/status evidence needed for the chosen iteration.
- Run narrow validation for the iteration. Run broader validation when the contract requires it.
- Review ownership boundaries, `git status --short`, and `git diff` before committing.
- Never commit a known-broken targeted gate.
- Commit only files related to the iteration.
- Push the commit to the configured upstream branch.
- Record validation evidence, commit hash, branch/remote, push result, or exact blocker in the exec plan.
- Compact the exec plan if it exceeds the contract size or after the iteration.

Stop conditions:
- Stop after one successful committed-and-pushed feature iteration.
- Stop immediately if validation, environment, credentials, push, CUDA hardware, network, or dirty-tree
  uncertainty blocks safe progress. Record the exact blocker in the exec plan.

Final response format:
- Iteration title
- Files changed
- Validation commands and pass/fail result
- Commit hash and push result, or blocker
""".strip()


def import_cursor_sdk() -> tuple[Any, Any, Any, Any]:
    try:
        from cursor_sdk import Agent, AgentOptions, CursorAgentError, LocalAgentOptions
    except ModuleNotFoundError as exc:
        raise Blocked(
            "cursor-sdk is not available; run this supervisor with "
            "`uv run --script .cursor/goal-supervisor/supervisor.py`"
        ) from exc
    return Agent, AgentOptions, CursorAgentError, LocalAgentOptions


def make_agent_options(repo: Path, model: str, api_key: str | None, agent_options: Any, local_agent_options: Any) -> Any:
    return agent_options(
        api_key=api_key,
        model=model,
        local=local_agent_options(cwd=str(repo)),
    )


def send_to_agent(
    repo: Path,
    goal_file: Path,
    state: SupervisorState,
    snapshot: GitSnapshot,
    *,
    api_key: str | None,
    model: str,
    state_path: Path,
) -> None:
    Agent, AgentOptions, CursorAgentError, LocalAgentOptions = import_cursor_sdk()
    prompt = build_iteration_prompt(repo, goal_file, state, snapshot)
    options = make_agent_options(repo, model, api_key, AgentOptions, LocalAgentOptions)

    try:
        if state.agent_id:
            log(f"resuming Cursor agent {state.agent_id}")
            with Agent.resume(state.agent_id, options) as agent:
                run_once_with_agent(agent, prompt, state, state_path)
        else:
            log("creating Cursor agent")
            with Agent.create(options) as agent:
                state.agent_id = agent.agent_id
                state.save(state_path)
                run_once_with_agent(agent, prompt, state, state_path)
    except CursorAgentError as exc:
        retryable = getattr(exc, "is_retryable", getattr(exc, "isRetryable", None))
        retry_after = getattr(exc, "retry_after", None)
        detail = f"Cursor SDK startup failed: {exc}; retryable={retryable}"
        if retry_after:
            detail += f"; retry_after={retry_after}"
        raise Blocked(detail) from exc


def run_once_with_agent(agent: Any, prompt: str, state: SupervisorState, state_path: Path) -> None:
    state.phase = "RUNNING"
    state.save(state_path)

    run_handle = agent.send(prompt)
    state.last_run_id = run_handle.id
    state.save(state_path)
    log(f"started Cursor run {run_handle.id}")

    result = run_handle.wait()
    status = getattr(result, "status", None)
    log(f"Cursor run {run_handle.id} finished with status={status}")
    if status != "finished":
        raise Blocked(f"Cursor run {run_handle.id} ended with status={status}")


def mark_blocked(state: SupervisorState, state_path: Path, message: str) -> None:
    state.phase = "BLOCKED"
    state.blocked = True
    state.blocker = message
    state.consecutive_failures += 1
    state.save(state_path)


def mark_idle(state: SupervisorState, state_path: Path, repo: Path) -> None:
    snapshot = git_snapshot(repo)
    state.phase = "IDLE"
    state.blocked = False
    state.blocker = None
    state.consecutive_failures = 0
    state.last_commit = snapshot.head
    state.save(state_path)


def run_iteration(args: argparse.Namespace, state: SupervisorState, state_path: Path) -> None:
    repo = args.repo.resolve()
    goal_file = args.goal_file.resolve()
    ensure_required_files(repo, goal_file)

    if state.blocked and not args.retry_blocker:
        raise Blocked(
            "supervisor is blocked from a previous run; inspect state/exec plan and rerun with "
            "`--retry-blocker` when ready"
        )

    snapshot = git_snapshot(repo)
    log(f"repo={repo} branch={snapshot.branch} head={snapshot.head}")
    if snapshot.status.strip():
        log("working tree is dirty; the agent will be instructed to classify and preserve existing changes")

    api_key = args.api_key or os.environ.get("CURSOR_API_KEY")
    if api_key:
        log("using explicit Cursor API key from CLI/env")
    else:
        log("CURSOR_API_KEY is not set; using SDK local bridge auth")

    send_to_agent(
        repo,
        goal_file,
        state,
        snapshot,
        api_key=api_key,
        model=args.model,
        state_path=state_path,
    )
    mark_idle(state, state_path, repo)


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=DEFAULT_REPO, help="repository root")
    parser.add_argument("--goal-file", type=Path, default=DEFAULT_REPO / "goal.md", help="goal contract file")
    parser.add_argument("--state-dir", type=Path, default=DEFAULT_STATE_DIR, help="durable supervisor state dir")
    parser.add_argument("--model", default=os.environ.get("CURSOR_MODEL", DEFAULT_MODEL), help="Cursor model id")
    parser.add_argument(
        "--api-key",
        default=None,
        help="Cursor API key; defaults to CURSOR_API_KEY, otherwise SDK local bridge auth",
    )
    parser.add_argument("--loop", action="store_true", help="keep running iterations until blocked or stopped")
    parser.add_argument(
        "--sleep-seconds",
        type=int,
        default=int(os.environ.get("GOAL_SUPERVISOR_SLEEP_SECONDS", "300")),
        help="sleep between iterations in loop mode",
    )
    parser.add_argument(
        "--retry-blocker",
        action="store_true",
        help="retry after a previously recorded blocker",
    )
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    state_dir = args.state_dir.resolve()
    state_path = state_dir / "state.json"
    lock_path = state_dir / "lock"

    stop_requested = False

    def request_stop(_signum: int, _frame: Any) -> None:
        nonlocal stop_requested
        stop_requested = True
        log("stop requested; finishing current boundary")

    signal.signal(signal.SIGTERM, request_stop)
    signal.signal(signal.SIGINT, request_stop)

    with contextlib.closing(acquire_lock(lock_path)):
        state = SupervisorState.load(state_path)
        state.phase = "BOOT"
        state.save(state_path)

        while True:
            try:
                run_iteration(args, state, state_path)
            except Blocked as exc:
                mark_blocked(state, state_path, str(exc))
                log(f"blocked: {exc}")
                return 2
            except Exception as exc:  # noqa: BLE001 - top-level crash accounting
                mark_blocked(state, state_path, f"unexpected supervisor failure: {exc}")
                traceback.print_exc()
                return 1

            if not args.loop or stop_requested:
                return 0

            log(f"sleeping {args.sleep_seconds}s before next iteration")
            time.sleep(args.sleep_seconds)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
