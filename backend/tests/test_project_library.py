import http.client
import json
import struct

import pytest

FSPJ = b"FSPJ" + bytes(range(64))
WEBP = b"RIFF" + struct.pack("<I", 12) + b"WEBP" + b"VP8 " + b"\x00" * 4
PNG = b"\x89PNG\r\n\x1a\n" + b"\x00" * 16
GLB = b"glTF" + b"\x00" * 20
LAYERS = {"layers": [{"kind": "fill", "name": "rust"}]}


def saved(client, name, blob=FSPJ):
    assert client.post(f"/api/projects/{name}/file", content=blob).status_code == 200
    return name


def listed(client):
    return {p["id"]: p for p in client.get("/api/projects?detail=1").json()["projects"]}


def trashed(client):
    return {p["entry"]: p for p in client.get("/api/projects/trash").json()["projects"]}


def raw(engine, method, path, body=None):
    conn = http.client.HTTPConnection("127.0.0.1", engine["port"], timeout=10)
    headers = {"x-formsurface-window-key": engine["key"]}
    if isinstance(body, dict):
        body = json.dumps(body).encode("utf-8")
        headers["content-type"] = "application/json"
    conn.request(method, path, body=body, headers=headers)
    response = conn.getresponse()
    text = response.read().decode("utf-8", "replace")
    conn.close()
    return response.status, text


def tree(folder):
    return sorted(str(p.relative_to(folder)) for p in folder.rglob("*"))


def test_template_listing_names_the_built_in_templates_and_their_categories(client):
    body = client.get("/api/project-templates").json()
    ids = [t["id"] for t in body["templates"]]
    assert "sphere" in ids
    assert len(set(ids)) == len(ids)
    for template in body["templates"]:
        assert template["name"] and template["category"] in body["categories"]
    assert client.get("/api/project-library").json()["counts"]["templates"] == len(ids)


def test_project_from_a_template_carries_the_template_and_is_not_listed_until_saved(client):
    made = client.post("/api/projects", json={"template": "sphere", "title": "lib from template"}).json()
    assert made["template"] == "sphere"
    assert made["title"] == "lib from template"
    assert made["saved"] is False
    assert made["id"] not in listed(client)
    saved(client, made["id"])
    summary = listed(client)[made["id"]]
    assert summary["saved"] is True
    assert summary["template"] == "sphere"
    assert summary["bytes"] == len(FSPJ)
    assert summary["thumb"] is False and summary["meshStored"] is False


def test_unknown_template_is_404_and_a_body_that_is_not_an_object_is_400(client):
    assert client.post("/api/projects", json={"template": "no-such-template"}).status_code == 404
    assert client.post("/api/projects", content=b"junk").status_code == 400


def test_two_projects_with_the_same_title_get_different_ids(client):
    first = client.post("/api/projects", json={"title": "lib twin"}).json()
    second = client.post("/api/projects", json={"title": "lib twin"}).json()
    assert first["title"] == second["title"] == "lib twin"
    assert first["id"] != second["id"]


@pytest.mark.parametrize("title", ["..", "a/b", "x\n", ".hidden", "a\\b"])
def test_an_unsafe_title_never_becomes_a_folder_outside_the_projects_folder(client, data_dir, title):
    before = tree(data_dir.parent)
    made = client.post("/api/projects", json={"title": title})
    assert made.status_code in (200, 400)
    new = [p for p in tree(data_dir.parent) if p not in before]
    assert all(p.startswith("data/projects/") for p in new), new
    if made.status_code == 200:
        folder = made.json()["id"]
        assert folder not in ("", ".", "..") and not folder.startswith(".")
        assert "/" not in folder and "\\" not in folder and "\n" not in folder
        assert (data_dir / "projects" / folder).is_dir()


def test_preview_round_trips_and_shows_in_the_summary(client):
    name = saved(client, "lib preview")
    assert client.get(f"/api/projects/{name}/thumb").status_code == 404
    assert client.put(f"/api/projects/{name}/thumb", content=WEBP).status_code == 200
    got = client.get(f"/api/projects/{name}/thumb")
    assert got.content == WEBP
    assert got.headers["content-type"] == "image/webp"
    assert listed(client)[name]["thumb"] is True


@pytest.mark.parametrize("data", [b"plain text, not an image", b"<svg xmlns='http://www.w3.org/2000/svg'/>", WEBP + b"\x00" * (2 * 1024 * 1024 + 1 - len(WEBP))])
def test_preview_that_is_not_an_image_or_is_over_2_mb_is_refused(client, data_dir, data):
    name = saved(client, "lib bad preview")
    assert client.put(f"/api/projects/{name}/thumb", content=data).status_code == 400
    assert listed(client)[name]["thumb"] is False
    assert sorted(p.name for p in (data_dir / "projects" / name).iterdir()) == ["project.fsproj"]


def test_stored_mesh_comes_back_with_its_bytes_and_original_file_name(client):
    name = saved(client, "lib mesh")
    assert client.get(f"/api/projects/{name}/mesh").status_code == 404
    put = client.put(f"/api/projects/{name}/mesh", params={"file": "model (1).glb"}, content=GLB)
    assert put.status_code == 200
    assert put.json()["bytes"] == len(GLB)
    got = client.get(f"/api/projects/{name}/mesh")
    assert got.content == GLB
    assert got.headers["x-mesh-name"] == "model (1).glb"
    assert listed(client)[name]["meshStored"] is True


@pytest.mark.parametrize("file", ["x.exe", "x.glb.exe", "", "noextension"])
def test_stored_mesh_must_be_a_mesh_file(client, file):
    name = saved(client, "lib mesh kinds")
    assert client.put(f"/api/projects/{name}/mesh", params={"file": file}, content=b"MZ").status_code == 400
    assert listed(client)[name]["meshStored"] is False


def test_stored_mesh_keeps_only_the_file_name_of_a_path(client, data_dir):
    name = saved(client, "lib mesh path")
    assert client.put(f"/api/projects/{name}/mesh", params={"file": "../../escape.glb"}, content=GLB).status_code == 200
    assert client.get(f"/api/projects/{name}/mesh").headers["x-mesh-name"] == "escape.glb"
    assert not (data_dir / "escape.glb").exists()
    assert not (data_dir / "projects" / "escape.glb").exists()


def test_info_updates_the_summary_the_library_lists(client):
    name = saved(client, "lib info")
    put = client.put(f"/api/projects/{name}/info", json={"title": "Shown title", "layers": 3, "sets": 2, "mesh": "model (1).glb"})
    assert put.status_code == 200
    summary = listed(client)[name]
    assert (summary["title"], summary["layers"], summary["sets"], summary["mesh"]) == ("Shown title", 3, 2, "model (1).glb")


def test_duplicate_is_a_full_copy_titled_as_a_copy_and_independent_of_the_original(client):
    name = saved(client, "lib dup")
    client.put(f"/api/projects/{name}/thumb", content=WEBP)
    client.put(f"/api/projects/{name}/mesh", params={"file": "a.glb"}, content=GLB)
    client.put(f"/api/projects/{name}/info", json={"title": "Dup source"})
    copy = client.post(f"/api/projects/{name}/duplicate").json()
    assert copy["id"] != name
    assert copy["title"] == "Dup source copy"
    assert copy["thumb"] is True and copy["meshStored"] is True
    assert client.get(f"/api/projects/{copy['id']}/file").content == FSPJ
    assert client.get(f"/api/projects/{copy['id']}/mesh").content == GLB
    saved(client, copy["id"], b"FSPJ-changed")
    assert client.get(f"/api/projects/{name}/file").content == FSPJ
    second = client.post(f"/api/projects/{name}/duplicate").json()
    assert second["id"] not in (name, copy["id"])


def test_delete_moves_the_project_to_the_trash_with_what_restore_needs(client):
    name = saved(client, "lib trash")
    client.put(f"/api/projects/{name}/info", json={"title": "Trash me", "layers": 4})
    entry = client.delete(f"/api/projects/{name}").json()["entry"]
    assert entry.startswith(f"{name}-") and entry[len(name) + 1:].isdigit()
    assert name not in listed(client)
    assert client.get(f"/api/projects/{name}/file").status_code == 404
    row = trashed(client)[entry]
    assert (row["id"], row["title"], row["layers"]) == (name, "Trash me", 4)
    assert str(row["deletedAt"]) == entry[len(name) + 1:]
    assert client.delete(f"/api/projects/{name}").status_code == 404


def test_restore_brings_the_project_back_unchanged(client):
    name = saved(client, "lib restore")
    client.put(f"/api/projects/{name}/thumb", content=PNG)
    client.put(f"/api/projects/{name}/mesh", params={"file": "a.obj"}, content=b"v 0 0 0\n")
    entry = client.delete(f"/api/projects/{name}").json()["entry"]
    back = client.post(f"/api/projects/trash/{entry}/restore")
    assert back.status_code == 200
    assert back.json()["id"] == name
    assert client.get(f"/api/projects/{name}/file").content == FSPJ
    assert client.get(f"/api/projects/{name}/thumb").content == PNG
    assert client.get(f"/api/projects/{name}/mesh").content == b"v 0 0 0\n"
    assert entry not in trashed(client)
    assert client.post(f"/api/projects/trash/{entry}/restore").status_code == 404


def test_restore_takes_a_new_id_when_the_name_is_taken_again(client):
    name = saved(client, "lib clash", b"FSPJ-old")
    entry = client.delete(f"/api/projects/{name}").json()["entry"]
    saved(client, name, b"FSPJ-new")
    back = client.post(f"/api/projects/trash/{entry}/restore").json()
    assert back["id"] != name
    assert client.get(f"/api/projects/{back['id']}/file").content == b"FSPJ-old"
    assert client.get(f"/api/projects/{name}/file").content == b"FSPJ-new"


def test_delete_permanently_removes_one_trash_entry_and_nothing_else(client, data_dir):
    keep = saved(client, "lib purge keep")
    kept_entry = client.delete(f"/api/projects/{saved(client, 'lib purge other')}").json()["entry"]
    entry = client.delete(f"/api/projects/{saved(client, 'lib purge gone')}").json()["entry"]
    assert client.delete(f"/api/projects/trash/{entry}").status_code == 200
    assert entry not in trashed(client) and kept_entry in trashed(client)
    assert not (data_dir / "projects" / ".trash" / entry).exists()
    assert client.get(f"/api/projects/{keep}/file").content == FSPJ
    assert client.delete(f"/api/projects/trash/{entry}").status_code == 404
    assert client.post(f"/api/projects/trash/{entry}/restore").status_code == 404


def test_counts_follow_the_lists_and_empty_trash_leaves_the_projects_alone(client):
    keep = saved(client, "lib count keep")
    client.delete(f"/api/projects/{saved(client, 'lib count a')}")
    client.delete(f"/api/projects/{saved(client, 'lib count b')}")
    counts = client.get("/api/project-library").json()["counts"]
    assert counts["projects"] == len(listed(client))
    assert counts["trash"] == len(trashed(client)) >= 2
    before = set(listed(client))
    emptied = client.delete("/api/projects/trash").json()
    assert emptied["removed"] == counts["trash"]
    assert trashed(client) == {}
    assert set(listed(client)) == before and keep in before
    assert client.get("/api/project-library").json()["counts"]["trash"] == 0
    assert client.delete("/api/projects/trash").json()["removed"] == 0


def test_library_settings_keep_pinned_and_hidden_lists_and_refuse_a_non_object(client):
    kept = client.post("/api/project-library/settings", json={"pinned": ["lib pin"], "recentHidden": ["lib hide"]}).json()
    assert kept == {"pinned": ["lib pin"], "recentHidden": ["lib hide"]}
    assert client.get("/api/project-library").json()["settings"] == kept
    assert client.post("/api/project-library/settings", json=[1]).status_code == 400
    client.post("/api/project-library/settings", json={"pinned": [], "recentHidden": []})


@pytest.mark.parametrize("route", ["duplicate", "opened"])
def test_library_actions_on_a_missing_project_are_404(client, route):
    assert client.post(f"/api/projects/lib-never-saved/{route}").status_code == 404
    assert client.put("/api/projects/lib-never-saved/info", json={"title": "x"}).status_code == 404


@pytest.mark.parametrize("name", ["%2e%2e", "..", ".trash", ".hidden", "abc%0A", "abc%0D", "a%5Cb", "a%00b", "a%2Fb", "..%2F..%2Fescape", "a" * 81])
@pytest.mark.parametrize("method,suffix,body", [("POST", "/file", FSPJ), ("POST", "/duplicate", None), ("PUT", "/thumb", PNG), ("PUT", "/mesh?file=a.glb", GLB), ("DELETE", "", None)])
def test_project_routes_refuse_names_that_could_leave_the_projects_folder(engine, name, method, suffix, body):
    before = tree(engine["data"].parent)
    status, _ = raw(engine, method, f"/api/projects/{name}{suffix}", body)
    assert status in (400, 404, 405)
    assert tree(engine["data"].parent) == before


@pytest.mark.parametrize("name", ["a b.c-d", "under_score", "9lives"])
def test_project_names_with_spaces_dots_dashes_and_underscores_are_saved(engine, name):
    status, _ = raw(engine, "POST", f"/api/projects/{name.replace(' ', '%20')}/file", FSPJ)
    assert status == 200
    assert (engine["data"] / "projects" / name / "project.fsproj").read_bytes() == FSPJ


@pytest.mark.parametrize("entry", ["%2e%2e", "..-123", ".home.json-1", "abc%0A", "a%2Fb", "never-there-1"])
@pytest.mark.parametrize("method,suffix", [("POST", "/restore"), ("DELETE", "")])
def test_trash_routes_refuse_entries_that_are_not_in_the_trash(engine, entry, method, suffix):
    raw(engine, "POST", "/api/projects/trash%20guard/file", FSPJ)
    before = tree(engine["data"].parent)
    status, _ = raw(engine, method, f"/api/projects/trash/{entry}{suffix}")
    assert status in (400, 404, 405)
    assert tree(engine["data"].parent) == before


def test_a_symlinked_project_folder_is_not_written_through(engine):
    outside = engine["data"].parent / "outside-projects"
    outside.mkdir(exist_ok=True)
    projects = engine["data"] / "projects"
    projects.mkdir(exist_ok=True)
    link = projects / "linked"
    if not link.is_symlink():
        link.symlink_to(outside, target_is_directory=True)
    status, _ = raw(engine, "POST", "/api/projects/linked/file", FSPJ)
    written = sorted(p.name for p in outside.iterdir())
    for p in outside.iterdir():
        p.unlink()
    assert status in (400, 404, 405)
    assert written == []


@pytest.mark.parametrize("method,suffix,body", [
    ("GET", "/file", None),
    ("POST", "/duplicate", None), ("PUT", "/thumb", PNG), ("PUT", "/mesh?file=a.glb", GLB), ("DELETE", "", None)])
def test_library_routes_do_not_follow_a_symlinked_project_folder(engine, method, suffix, body):
    outside = engine["data"].parent / "outside-library"
    outside.mkdir(exist_ok=True)
    (outside / "project.fsproj").write_bytes(FSPJ)
    projects = engine["data"] / "projects"
    projects.mkdir(exist_ok=True)
    link = projects / "linked-library"
    if not link.is_symlink():
        link.symlink_to(outside, target_is_directory=True)
    status, _ = raw(engine, method, f"/api/projects/linked-library{suffix}", body)
    assert status in (400, 404, 405)
    assert sorted(p.name for p in outside.iterdir()) == ["project.fsproj"]
    assert link.is_symlink()


@pytest.mark.parametrize("path,body", [
    ("/api/library/abc%0A", LAYERS),
    ("/api/library/%2e%2e", LAYERS),
    ("/api/library/a%2Fb", LAYERS),
    ("/api/templates/abc%0A", {"maps": [{"suffix": "BC", "rgb": "basecolor"}]}),
    ("/api/smart-masks/abc%0A", {"settings": {"generator": "cavity"}}),
])
def test_saved_asset_names_ending_in_a_newline_or_leaving_the_folder_are_refused(engine, path, body):
    before = tree(engine["data"].parent)
    status, _ = raw(engine, "POST", path, body)
    assert status in (400, 404, 405)
    assert tree(engine["data"].parent) == before


@pytest.mark.parametrize("body,reason", [({}, "id"), ({"id": 5}, "id"), ([1], "JSON object")])
def test_command_bus_answers_bad_arguments_with_400_and_a_reason(client, body, reason):
    r = client.post("/api/commands/terminal.write", json=body)
    assert r.status_code == 400
    assert reason in r.json()["detail"]


def test_command_bus_answers_a_body_that_is_not_json_and_an_unknown_terminal_without_a_500(client):
    assert 400 <= client.post("/api/commands/terminal.write", content=b"{broken").status_code < 500
    assert client.post("/api/commands/terminal.write", json={"id": "no-such-terminal", "data": "x"}).status_code == 404


@pytest.mark.parametrize("args", [{}, {"values": 5}, {"values": []}, {"id": []}, {"id": None, "data": 3}, {"text": {"a": 1}}, {"cols": "wide", "rows": -1}])
def test_no_listed_command_answers_500_to_arguments_of_the_wrong_type(client, args):
    safe = [c["id"] for c in client.get("/api/commands").json()["commands"] if c["id"].split(".")[-1] in ("write", "resize", "close", "rename", "set", "send", "stop", "input", "kill")]
    assert safe
    for cid in safe:
        status = client.post(f"/api/commands/{cid}", json=args).status_code
        assert status < 500, f"{cid} {args} answered {status}"


@pytest.mark.parametrize("method,path", [
    ("POST", "/api/projects"),
    ("POST", "/api/projects/guarded/file"),
    ("PUT", "/api/projects/guarded/thumb"),
    ("PUT", "/api/projects/guarded/mesh?file=a.glb"),
    ("POST", "/api/projects/guarded/duplicate"),
    ("DELETE", "/api/projects/guarded"),
    ("DELETE", "/api/projects/trash"),
    ("POST", "/api/project-library/settings"),
    ("POST", "/api/commands/prefs.set"),
    ("POST", "/api/library/guarded"),
])
@pytest.mark.parametrize("headers", [{"origin": "http://evil.example"}, {"host": "evil.example"}, {"sec-fetch-site": "cross-site"}])
def test_mutating_routes_refuse_a_request_that_is_not_from_this_machine(client, data_dir, method, path, headers):
    before = tree(data_dir)
    assert client.request(method, path, headers=headers, content=FSPJ).status_code == 403
    assert tree(data_dir) == before
