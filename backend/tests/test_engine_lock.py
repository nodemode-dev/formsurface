import json
import os
import re
import subprocess
import sys
import time

import pytest

from conftest import BACKEND, DATA_ENV, PORT_ENV, free_port

pytestmark = pytest.mark.skipif(os.name == "nt", reason="the stand-in shell is a POSIX sleep")
ENTRY = BACKEND / "formsurface_engine.py"
URL = re.compile(r"http://127\.0\.0\.1:(\d+)")


def engine(data, shell, var="FORMSURFACE_PARENT_PID"):
    env = {k: v for k, v in os.environ.items() if not k.startswith("FORMSURFACE_")}
    env.update({DATA_ENV: str(data), PORT_ENV: str(free_port()), "PYTHONUNBUFFERED": "1", var: str(shell)})
    return subprocess.Popen([sys.executable, str(ENTRY)], cwd=BACKEND, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)


def serving(proc, seconds=60):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        line = proc.stdout.readline()
        if not line:
            break
        if URL.search(line):
            return True
    return False


def stop(*procs):
    for proc in procs:
        if proc.poll() is None:
            proc.kill()
        proc.wait(timeout=10)


def test_a_second_engine_is_refused_with_its_sentence_and_exit_2(tmp_path):
    shell = subprocess.Popen(["sleep", "120"])
    first = engine(tmp_path, shell.pid)
    second = None
    try:
        assert serving(first)
        lock = json.loads((tmp_path / "engine.lock").read_text(encoding="utf-8"))
        assert lock["pid"] == first.pid and lock["shell"] == shell.pid
        second = engine(tmp_path, shell.pid, "FORMSURFACE_SHELL_PID")
        out = second.communicate(timeout=60)[0]
        assert second.returncode == 2
        assert f"formsurface is already running on {tmp_path} (engine process {first.pid})." in out
        assert first.poll() is None
        assert "already running" in (tmp_path / "Logs" / "formsurface-engine.log").read_text(encoding="utf-8")
    finally:
        stop(first, shell, *([second] if second else []))


def test_the_engine_leaves_when_its_shell_ends_and_a_leftover_is_replaced(tmp_path):
    shell = subprocess.Popen(["sleep", "120"])
    gone = subprocess.Popen(["sleep", "120"])
    other = subprocess.Popen(["sleep", "120"])
    first = engine(tmp_path, shell.pid)
    left = second = None
    try:
        assert serving(first)
        shell.kill()
        shell.wait()
        first.wait(timeout=15)
        assert not (tmp_path / "engine.lock").exists()

        left = engine(tmp_path, gone.pid)
        assert serving(left)
        os.kill(left.pid, 19)
        gone.kill()
        gone.wait()
        second = engine(tmp_path, other.pid)
        assert serving(second)
        left.wait(timeout=15)
        assert json.loads((tmp_path / "engine.lock").read_text(encoding="utf-8"))["pid"] == second.pid
        assert "left running without its app was stopped" in (tmp_path / "Logs" / "formsurface-engine.log").read_text(encoding="utf-8")
    finally:
        if left is not None and left.poll() is None:
            os.kill(left.pid, 18)
        stop(first, shell, gone, other, *[p for p in (left, second) if p is not None])


def test_a_damaged_lock_is_replaced(tmp_path):
    (tmp_path / "engine.lock").write_text("{not json", encoding="utf-8")
    shell = subprocess.Popen(["sleep", "120"])
    proc = engine(tmp_path, shell.pid)
    try:
        assert serving(proc)
        assert json.loads((tmp_path / "engine.lock").read_text(encoding="utf-8"))["pid"] == proc.pid
    finally:
        stop(proc, shell)


def test_selfcheck_runs_with_its_streams_closed():
    code = ("import os, sys, runpy; sys.stdin = None; sys.stdout = None; sys.stderr = None; "
            "g = runpy.run_path('formsurface_engine.py', run_name='engine'); os.write(1, ('selfcheck %s' % g['selfcheck']()).encode())")
    done = subprocess.run([sys.executable, "-c", code], cwd=BACKEND, capture_output=True, text=True, timeout=120)
    plain = subprocess.run([sys.executable, str(ENTRY), "--selfcheck"], cwd=BACKEND, capture_output=True, text=True, timeout=120)
    assert done.returncode == 0 and done.stdout == f"selfcheck {plain.returncode}", done.stdout + done.stderr
