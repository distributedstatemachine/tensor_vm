# Goal Supervisor

Runs `goal.md` as a persistent, restartable Cursor agent task.

The supervisor is intentionally outside TensorVM consensus/runtime code. It is ops automation that enforces one feature-sized iteration at a time:

1. Read the goal contract and canonical TensorVM docs.
2. Resume or create a local Cursor agent.
3. Ask the agent to complete exactly one feature iteration.
4. Require validation, commit, push, and exec-plan evidence.
5. Save machine state in `.cursor/goal-supervisor/state.json`.
6. Stop on blockers instead of silently continuing.

## Install

```bash
cd /home/ubuntu/tensor_vm
uv --version
cp .cursor/goal-supervisor/env.example .cursor/goal-supervisor/supervisor.env
$EDITOR .cursor/goal-supervisor/supervisor.env
chmod +x .cursor/goal-supervisor/supervisor.py
```

`supervisor.env` is gitignored.
`CURSOR_API_KEY` is optional: when omitted, the SDK launches the local Cursor bridge and uses the current local Cursor auth path. Keep an explicit API key only for service-account or non-interactive deployments.

Default model:

```bash
CURSOR_MODEL=gpt-5.5-medium
```

## Run Once

```bash
cd /home/ubuntu/tensor_vm
uv run --script .cursor/goal-supervisor/supervisor.py
```

Run once is the safest manual mode. It performs at most one complete feature iteration or records a blocker.

## Run Continuously

```bash
cd /home/ubuntu/tensor_vm
uv run --script .cursor/goal-supervisor/supervisor.py --loop
```

The loop sleeps `GOAL_SUPERVISOR_SLEEP_SECONDS` between iterations. It exits with:

- `0` after a clean one-shot run or graceful stop.
- `1` for unexpected supervisor failures.
- `2` when progress is blocked and operator input is required.

## systemd User Service

```bash
cd /home/ubuntu/tensor_vm
chmod +x .cursor/goal-supervisor/supervisor.py
mkdir -p ~/.config/systemd/user
ln -sf /home/ubuntu/tensor_vm/.cursor/goal-supervisor/tensorvm-goal-supervisor.service \
  ~/.config/systemd/user/tensorvm-goal-supervisor.service
systemctl --user daemon-reload
systemctl --user enable --now tensorvm-goal-supervisor.service
```

Watch logs:

```bash
journalctl --user -u tensorvm-goal-supervisor.service -f
```

Stop:

```bash
systemctl --user stop tensorvm-goal-supervisor.service
```

## Recovery

If the supervisor records a blocker, inspect:

```bash
cd /home/ubuntu/tensor_vm
git status --short
git diff
sed -n '1,220p' docs/tensorvm/local_chain_production_exec_plan.md
sed -n '1,220p' .cursor/goal-supervisor/state.json
```

After fixing the blocker:

```bash
uv run --script .cursor/goal-supervisor/supervisor.py --retry-blocker
```

For continuous mode:

```bash
systemctl --user restart tensorvm-goal-supervisor.service
```

## State Files

Tracked:

- `.cursor/goal-supervisor/supervisor.py`
- `.cursor/goal-supervisor/tensorvm-goal-supervisor.service`
- `.cursor/goal-supervisor/env.example`
- `.cursor/goal-supervisor/README.md`

Ignored:

- `.cursor/goal-supervisor/supervisor.env`
- `.cursor/goal-supervisor/state.json`
- `.cursor/goal-supervisor/lock`
- `.cursor/goal-supervisor/logs/`

The human source of truth remains `docs/tensorvm/local_chain_production_exec_plan.md`; `state.json` is only for process recovery.
