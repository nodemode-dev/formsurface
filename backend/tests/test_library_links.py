import json
import os
import shutil
import subprocess
import sys
import time

import pytest

from conftest import APP_MODULE, BACKEND, DATA_ENV, HEALTH_PATH, PORT_ENV, free_port, get_json

FSPJ = b"FSPJ" + bytes(range(64))
PNG = b"\x89PNG\r\n\x1a\n" + b"\x00" * 16
GLB = b"glTF" + b"\x00" * 20
SECRET = b"outside the library"


def link(path, target, directory=False):
    try:
        path.symlink_to(target, target_is_directory=directory)
    except (OSError, NotImplementedError):
        pytest.skip("this system does not let the test make a symbolic link")


@pytest.fixture
def outside(data_dir):
    folder = data_dir.parent / "outside-links"
    shutil.rmtree(folder, ignore_errors=True)
    folder.mkdir()
    yield folder
    shutil.rmtree(folder, ignore_errors=True)


@pytest.fixture
def project(client, data_dir):
    name = "links project"
    folder = data_dir / "projects" / name
    shutil.rmtree(folder, ignore_errors=True)
    assert client.post(f"/api/projects/{name}/file", content=FSPJ).status_code == 200
    yield name, folder
    if folder.is_symlink():
        folder.unlink()
    shutil.rmtree(folder, ignore_errors=True)


@pytest.fixture
def linked_folder(data_dir, outside):
    projects = data_dir / "projects"
    projects.mkdir(exist_ok=True)
    path = projects / "links folder"
    link(path, outside, directory=True)
    yield "links folder"
    path.unlink()


def names(folder):
    return sorted(p.name for p in folder.iterdir())


def test_the_map_save_does_not_write_through_a_linked_project_folder(client, linked_folder, outside):
    r = client.post(f"/api/projects/{linked_folder}", data={"document": "{}"}, files=[("maps", ("base.png", PNG, "image/png"))])
    assert r.status_code in (400, 404)
    assert names(outside) == []


def test_the_map_document_is_not_read_through_a_linked_project_folder(client, linked_folder, outside):
    (outside / "project.json").write_text(json.dumps({"secret": "outside"}))
    r = client.get(f"/api/projects/{linked_folder}")
    assert r.status_code == 404
    assert "outside" not in r.text


def test_info_and_opened_do_not_follow_a_linked_project_folder(client, linked_folder, outside):
    (outside / "project.fsproj").write_bytes(FSPJ)
    assert client.put(f"/api/projects/{linked_folder}/info", json={"title": "x"}).status_code == 404
    assert client.post(f"/api/projects/{linked_folder}/opened").status_code == 404
    assert client.get(f"/api/projects/{linked_folder}/thumb").status_code == 404
    assert client.get(f"/api/projects/{linked_folder}/mesh").status_code == 404
    assert names(outside) == ["project.fsproj"]


@pytest.mark.parametrize("partial,method,suffix,body", [
    ("project.fsproj.part", "POST", "/file", FSPJ + b"new"),
    ("thumb.webp.part", "PUT", "/thumb", PNG),
    ("mesh.glb.part", "PUT", "/mesh?file=a.glb", GLB),
    ("info.json.part", "PUT", "/info", b'{"title": "renamed"}'),
])
def test_a_linked_partial_file_is_not_written_through(client, project, outside, partial, method, suffix, body):
    name, folder = project
    target = outside / "victim.bin"
    target.write_bytes(SECRET)
    link(folder / partial, target)
    r = client.request(method, f"/api/projects/{name}{suffix}", content=body)
    assert 400 <= r.status_code < 500
    assert target.read_bytes() == SECRET
    assert names(outside) == ["victim.bin"]
    assert (folder / "project.fsproj").read_bytes() == FSPJ


def test_a_project_saves_again_once_the_linked_partial_file_is_gone(client, project, outside):
    name, folder = project
    link(folder / "project.fsproj.part", outside / "victim.bin")
    assert client.post(f"/api/projects/{name}/file", content=FSPJ + b"new").status_code == 400
    (folder / "project.fsproj.part").unlink()
    assert client.post(f"/api/projects/{name}/file", content=FSPJ + b"new").status_code == 200
    assert (folder / "project.fsproj").read_bytes() == FSPJ + b"new"


def test_a_linked_project_file_is_not_served_copied_or_touched(client, project, outside):
    name, folder = project
    target = outside / "victim.fsproj"
    target.write_bytes(b"FSPJ" + SECRET)
    os.utime(target, (1_000_000, 1_000_000))
    (folder / "project.fsproj").unlink()
    link(folder / "project.fsproj", target)
    read = client.get(f"/api/projects/{name}/file")
    assert read.status_code == 404
    assert SECRET not in read.content
    before = names(folder.parent)
    assert client.post(f"/api/projects/{name}/duplicate").status_code == 404
    assert client.post(f"/api/projects/{name}/opened").status_code == 404
    assert names(folder.parent) == before
    assert target.stat().st_mtime == 1_000_000
    assert target.read_bytes() == b"FSPJ" + SECRET


def test_a_linked_map_file_is_not_written_through(client, project, outside):
    name, folder = project
    target = outside / "victim.png"
    target.write_bytes(SECRET)
    link(folder / "base.png", target)
    r = client.post(f"/api/projects/{name}", data={"document": "{}"}, files=[("maps", ("base.png", PNG, "image/png"))])
    assert r.status_code == 400
    assert target.read_bytes() == SECRET


def test_a_linked_library_settings_file_is_not_read_or_written_through(client, data_dir, outside):
    projects = data_dir / "projects"
    projects.mkdir(exist_ok=True)
    settings = projects / ".home.json"
    kept = settings.read_bytes() if settings.is_file() and not settings.is_symlink() else None
    settings.unlink(missing_ok=True)
    target = outside / "home.json"
    target.write_text(json.dumps({"pinned": ["from outside"], "recentHidden": []}))
    link(settings, target)
    try:
        assert client.get("/api/project-library").json()["settings"]["pinned"] == []
        assert client.post("/api/project-library/settings", json={"pinned": ["mine"]}).json()["pinned"] == ["mine"]
        assert json.loads(target.read_text())["pinned"] == ["from outside"]
        assert not settings.is_symlink()
        assert json.loads(settings.read_text())["pinned"] == ["mine"]
    finally:
        settings.unlink(missing_ok=True)
        if kept is not None:
            settings.write_bytes(kept)


def test_a_linked_info_file_is_not_shown_in_the_library(client, project, outside):
    name, folder = project
    (outside / "info.json").write_text(json.dumps({"title": "title from outside", "mesh": "mesh from outside"}))
    (folder / "info.json").unlink(missing_ok=True)
    link(folder / "info.json", outside / "info.json")
    listed = {p["id"]: p for p in client.get("/api/projects?detail=1").json()["projects"]}
    assert listed[name]["title"] == name
    assert listed[name]["mesh"] == ""


@pytest.fixture
def linked_trash(client, data_dir, outside):
    trash = data_dir / "projects" / ".trash"
    held = data_dir / "projects" / ".trash-held"
    if trash.exists():
        trash.rename(held)
    entry = outside / "kept-123"
    entry.mkdir()
    (entry / "project.fsproj").write_bytes(FSPJ)
    link(trash, outside, directory=True)
    yield "kept-123", entry
    trash.unlink()
    if held.exists():
        held.rename(trash)


def test_a_linked_trash_takes_no_project_and_gives_none_up(client, project, linked_trash, outside):
    name, folder = project
    entry, path = linked_trash
    assert client.get("/api/projects/trash").json()["projects"] == []
    assert client.get("/api/project-library").json()["counts"]["trash"] == 0
    assert 400 <= client.delete(f"/api/projects/{name}").status_code < 500
    assert (folder / "project.fsproj").read_bytes() == FSPJ
    assert client.post(f"/api/projects/trash/{entry}/restore").status_code == 404
    assert client.delete(f"/api/projects/trash/{entry}").status_code == 404
    assert client.delete("/api/projects/trash").json()["removed"] == 0
    assert names(outside) == [entry]
    assert names(path) == ["project.fsproj"]
    assert not (folder.parent / "kept").exists()


def test_a_library_whose_own_folder_is_a_link_still_saves_reads_and_trashes(tmp_path):
    real = tmp_path / "another-disk" / "projects"
    real.mkdir(parents=True)
    data = tmp_path / "data"
    data.mkdir()
    link(data / "projects", real, directory=True)
    port = free_port()
    env = dict(os.environ, **{DATA_ENV: str(data), PORT_ENV: str(port)})
    proc = subprocess.Popen([sys.executable, "-m", "uvicorn", APP_MODULE, "--host", "127.0.0.1", "--port", str(port)],
                            cwd=BACKEND, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    base = f"http://127.0.0.1:{port}"
    try:
        deadline = time.monotonic() + 60
        while True:
            try:
                if get_json(base + HEALTH_PATH)[0] == 200:
                    break
            except OSError:
                assert time.monotonic() < deadline and proc.poll() is None, "the service did not start"
                time.sleep(0.25)
        import httpx

        with httpx.Client(base_url=base, trust_env=False, timeout=10) as http:
            made = http.post("/api/projects", json={"template": "sphere", "title": "on a link"}).json()
            pid = made["id"]
            assert http.post(f"/api/projects/{pid}/file", content=FSPJ).status_code == 200
            assert (real / pid / "project.fsproj").read_bytes() == FSPJ
            assert http.get(f"/api/projects/{pid}/file").content == FSPJ
            assert http.put(f"/api/projects/{pid}/thumb", content=PNG).status_code == 200
            assert http.put(f"/api/projects/{pid}/mesh?file=a.glb", content=GLB).status_code == 200
            assert http.get(f"/api/projects/{pid}/mesh").content == GLB
            assert [p["title"] for p in http.get("/api/projects?detail=1").json()["projects"]] == ["on a link"]
            copy = http.post(f"/api/projects/{pid}/duplicate").json()["id"]
            entry = http.delete(f"/api/projects/{pid}").json()["entry"]
            assert [p["entry"] for p in http.get("/api/projects/trash").json()["projects"]] == [entry]
            assert http.post(f"/api/projects/trash/{entry}/restore").json()["id"] == pid
            assert http.delete(f"/api/projects/{copy}").status_code == 200
            assert http.delete("/api/projects/trash").json()["removed"] == 1
            assert sorted(p.name for p in real.iterdir() if not p.name.startswith(".")) == [pid]
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()
